use crate::{
    app_menu::app_menu::AppMenu,
    config::AppConfig,
    libraries::libraries_state::LibrariesState,
    panels::{
        panel_kind::PanelKind,
        panel_tree::{CloseError, LeafId, PanelTree, SplitDirection},
        panel_view::{self, PanelAction, PanelWindowId},
        panel_windows::{self, SecondaryWindow},
    },
    themes::theme::{set_theme, Theme},
    APP_MENU_HEIGHT,
};
use eframe::egui;
use epaint::{pos2, CornerRadius, Rect};
use crate::program::Program;
use std::collections::{HashMap, HashSet};
use std::path::PathBuf;

pub const PROFILE: bool = false;
// pub const DEFAULT_WINDOW_WIDTH: f32 = 1280.0;
// pub const DEFAULT_WINDOW_HEIGHT: f32 = 800.0;
// pub const APP_MENU_HEIGHT: f32 = 35.0;


pub struct App {
    app_menu: AppMenu,
    programs: HashMap<String, Program>,
    current_program: Option<String>,
    theme: Theme,
    /// The main window's panel layout. Lazily initialized on first frame once
    /// the work rect is known (config loading arrives in Phase 4).
    main_tree: Option<PanelTree>,
    /// Monotonic allocator for panel leaf ids across all windows.
    next_leaf_id: LeafId,
    /// Secondary OS windows, each hosting its own panel tree (session-only).
    secondary_windows: Vec<SecondaryWindow>,
    /// Monotonic allocator for secondary window ids.
    next_window_id: u64,
    /// App-global Libraries panel state (linked libraries + folder scanner).
    /// Shared by every program tab and window; persisted in `AppConfig`.
    libraries: LibrariesState,
    /// Error text awaiting user acknowledgement, rendered as a modal with an
    /// OK button. Set wherever creating/loading a `Program` fails (menu bar
    /// new/load, Libraries open/create, startup) — with tolerant graph
    /// loading in core, these are now only real IO or top-level JSON
    /// corruption failures, which used to be silently swallowed.
    error_modal: Option<String>,
    /// Deferred close of an unsaved-but-non-empty tab (see [`PendingClose`]).
    /// At most one close is in flight at a time; further close requests are
    /// ignored until this resolves.
    pending_close: Option<PendingClose>,
    /// True while an app quit is being resolved tab by tab: each frame we
    /// prompt for the next unsaved non-empty tab; when none remain we
    /// re-issue `ViewportCommand::Close` and stop cancelling it. Cleared by
    /// a cancel in the prompt (the user changed their mind about quitting).
    quit_requested: bool,
}

/// State of a tab close that couldn't complete immediately because the
/// graph has unsaved content the user must decide about.
#[derive(Clone)]
enum PendingClose {
    /// The save/discard/cancel modal is up for this tab.
    Prompt { program_id: String },
    /// The user chose save and picked a path; waiting for the engine's
    /// `SavedTo` ack before actually closing — closing aborts the engine
    /// task, which would race the write if we didn't wait for confirmation.
    AwaitingSave {
        program_id: String,
        since: std::time::Instant,
    },
}

impl eframe::App for App {
    fn ui(&mut self, outer_ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        if PROFILE {
            puffin::profile_function!();
            puffin::GlobalProfiler::lock().new_frame(); // call once per frame!
            // puffin_egui::profiler_window(ctx); // disabled: puffin_egui not compatible with egui 0.33
        }

        let ctx = outer_ui.ctx().clone();

        // App-quit intercept: while any tab has unsaved content (or a close
        // is already being decided), cancel the OS close and resolve it via
        // the prompt flow below instead. Once every unsaved tab is dealt
        // with, the re-issued Close passes straight through here.
        if ctx.input(|i| i.viewport().close_requested()) {
            let any_unsaved = self.programs.values().any(|p| p.has_unsaved_content());
            if any_unsaved || self.pending_close.is_some() {
                ctx.send_viewport_cmd(egui::ViewportCommand::CancelClose);
                self.quit_requested = true;
            }
        }

        egui::CentralPanel::default().show(outer_ui, |ui| {
            // bg
            ui.painter().add(egui::Shape::rect_filled(
                ui.max_rect(),
                CornerRadius::ZERO,
                self.theme.get().panel_fill,
            ));

            let bar_response = self.app_menu.show(&ctx, ui, &self.programs, &self.current_program, &self.theme);

            // Both of these construct/locate a `Program` and may populate
            // `error_modal` on failure (real IO or top-level JSON corruption
            // — tolerant loading in core absorbs everything else).
            if bar_response.new_graph_requested {
                self.create_new_program();
            }
            if let Some(path) = bar_response.open_path {
                self.open_or_focus(path);
            }

            if let Some(current_program) = bar_response.current_program {
                self.current_program = Some(current_program);
            }

            if let Some(theme) = bar_response.theme_changed_to {
                set_theme(&ctx, theme.clone());
                self.theme = theme.clone();

                // Persist theme choice to config.
                let mut config = AppConfig::load();
                config.theme = Some(theme.config_name().to_string());
                config.save();
            }

            // Work area = everything below the menu bar. Same source the old
            // fixed layout used (`ctx.content_rect()`).
            let content_rect = ctx.content_rect();
            let work_rect =
                Rect::from_min_max(pos2(content_rect.left(), APP_MENU_HEIGHT), content_rect.max);

            // Lazy tree init: the system default needs the work width. Prefer
            // a user-saved layout from config, falling back to system default.
            if self.main_tree.is_none() {
                let config = AppConfig::load();
                self.main_tree = Some(match config.default_layout {
                    Some(root) => {
                        let mut tree = PanelTree { root };
                        tree.reassign_ids(&mut self.next_leaf_id);
                        tree
                    }
                    None => PanelTree::system_default(work_rect.width(), &mut self.next_leaf_id),
                });
            }

            if let Some(action) = bar_response.panel_action {
                self.handle_panel_action(action, work_rect);
            }

            // Split/close commands raised by panels' corner buttons this frame
            // (main window + secondary windows), applied after rendering so the
            // borrow on `program` has ended.
            let mut panel_actions: Vec<PanelAction> = Vec::new();

            if let Some(current_program) = &self.current_program {
                let has_preview_2d_panel = self.has_preview_2d_panel();
                if let Some(program) = self.programs.get_mut(current_program) {
                    program.has_preview_2d_panel = has_preview_2d_panel;
                    program.update(&ctx, ui);
                    // A dropped `.mangler.json` opens as a tab — queue it on the
                    // libraries action channel (drained below), the same path
                    // library double-clicks take. `self.libraries` is a field
                    // disjoint from `self.programs`, so this borrows cleanly
                    // alongside the live `program` borrow.
                    for path in program.take_pending_open_graphs() {
                        self.libraries.push_action(
                            crate::libraries::libraries_state::LibraryAction::OpenGraph { path },
                        );
                    }
                    let resp = panel_view::render_tree(
                        ui,
                        self.main_tree.as_mut().unwrap(),
                        work_rect,
                        PanelWindowId::Main,
                        program,
                        &mut self.libraries,
                        &self.theme,
                    );
                    panel_actions.extend(resp.panel_action);
                    program.show_overlays(&ctx, ui, &self.theme, &resp.graph_rects, work_rect);

                    // Secondary windows render the same current program. These
                    // App fields are all disjoint from `program`, so borrow
                    // them directly (no method calls on `self` here).
                    for win in &mut self.secondary_windows {
                        let resp = panel_windows::show_secondary_window(
                            &ctx,
                            win,
                            program,
                            &mut self.libraries,
                            &self.theme,
                        );
                        panel_actions.extend(resp);
                    }
                }
            }

            // With no graphs open, the panel tree has nothing to render (all
            // its panels show the current program). Fall back to a lone
            // Libraries panel filling the work area — rendered directly, so
            // no splitters and no kind-switcher corner button — from which
            // the user can open or create a graph.
            let has_open_program = self
                .current_program
                .as_ref()
                .is_some_and(|id| self.programs.contains_key(id));
            if !has_open_program {
                ui.scope_builder(egui::UiBuilder::new().max_rect(work_rect), |ui| {
                    ui.set_clip_rect(work_rect);
                    crate::libraries::libraries_panel::show(
                        ui,
                        &mut self.libraries,
                        &self.theme,
                        None,
                        None,
                    );
                });
            }

            // Apply corner-button split/close commands. Each carries its own
            // target window/leaf, so `handle_panel_action` acts on it directly.
            for action in panel_actions {
                self.handle_panel_action(action, work_rect);
            }

            // Perform requests raised by the Libraries panel this frame.
            // Deferred to here (like panel_actions) because they mutate the
            // programs map, which was borrowed during rendering.
            for action in self.libraries.take_pending() {
                self.handle_library_action(action);
            }

            // Reap secondary windows whose titlebar close button was pressed.
            if self.secondary_windows.iter().any(|w| w.close_requested) {
                self.secondary_windows.retain(|w| !w.close_requested);
                // Prune viewers to the leaves that remain live.
                self.prune_all_viewers();
            }

            if let Some(program_id_to_close) = bar_response.program_to_close {
                self.request_close(program_id_to_close);
            }

            // Quit flow: with no close currently being decided, prompt for
            // the next unsaved non-empty tab, or — when none remain — stop
            // cancelling the close and re-issue it (next frame's intercept
            // lets it through). One prompt at a time, sequentially.
            if self.quit_requested && self.pending_close.is_none() {
                let next = self
                    .programs
                    .iter()
                    .find(|(_, p)| p.has_unsaved_content())
                    .map(|(id, _)| id.clone());
                match next {
                    Some(id) => {
                        // Focus the tab so the user sees what they're
                        // deciding about (and so its message pump runs —
                        // see request_close).
                        self.current_program = Some(id.clone());
                        self.pending_close = Some(PendingClose::Prompt { program_id: id });
                    }
                    None => {
                        self.quit_requested = false;
                        ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                    }
                }
            }

            self.show_close_prompt_modal(ui);

            // Error modal: any failed Program creation/open this frame (menu
            // bar, Libraries panel, startup) lands here. OK (or Esc/outside
            // click) dismisses — unlike the file-conflict modal, there is
            // nothing to decide, just something to acknowledge.
            self.show_error_modal(ui);
        });
    }

    

    fn save(&mut self, _storage: &mut dyn eframe::Storage) {}

    fn auto_save_interval(&self) -> std::time::Duration {
        std::time::Duration::from_secs(30)
    }

    // fn clear_color(&self, _visuals: &egui::Visuals) -> [f32; 4] {
    //     // NOTE: a bright gray makes the shadows of the windows look weird.
    //     // We use a bit of transparency so that if the user switches on the
    //     // `transparent()` option they get immediate results.
    //     egui::Color32::from_rgba_unmultiplied(12, 12, 12, 180).to_normalized_gamma_f32()

    //     // _visuals.window_fill() would also be a natural choice
    // }

    fn persist_egui_memory(&self) -> bool {
        true
    }

    /// Shuts down every still-open program's engine task. No file cleanup is
    /// needed: unsaved graphs have no file, and saved graphs keep theirs
    /// (auto-save already flushed any pending edits). Unsaved graphs with
    /// content never reach this point silently — the close-requested intercept
    /// in `ui` prompts for each of them before letting the quit proceed.
    fn on_exit(&mut self, _gl: Option<&eframe::glow::Context>) {
        for program in self.programs.values() {
            program.app.thread_handle.abort();
        }
    }
}

impl App {
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        setup_fonts(&cc.egui_ctx);

        // Load persistent config.
        let mut config = AppConfig::load();

        // Restore theme from config, or use default.
        let theme = config.theme.as_deref()
            .and_then(Theme::from_name)
            .unwrap_or(crate::DEFAULT_THEME);
        set_theme(&cc.egui_ctx, theme.clone());

        // Make sure a default library folder exists (creating it and/or
        // linking it into `config.libraries` on first run) so the Libraries
        // panel has content and first-save dialogs have somewhere sensible to
        // start. `LibrariesState::new` further down clones the (now possibly
        // updated) `config.libraries`, so a freshly-linked default library
        // shows up in the panel immediately, no restart needed.
        config.ensure_default_library();
        config.save();

        let mut programs = HashMap::new();
        let mut current_program: Option<String> = None;
        let mut error_modal: Option<String> = None;

        // The initial empty tab: in-memory and unsaved (no file exists until
        // the user saves). Creating a fresh Program has no file IO, so this
        // failing means something is deeply wrong (e.g. the engine task
        // couldn't spawn) — surface it instead of silently starting with no
        // tab and no explanation.
        match Program::new(None, None) {
            Ok(program) => {
                current_program = Some(program.app.id.clone());
                programs.insert(program.app.id.clone(), program);
            }
            Err(error) => {
                error_modal = Some(format!("Failed to create a new graph: {}", error.0));
            }
        }

        Self {
            app_menu: AppMenu::new(),
            programs: programs,
            current_program: current_program,
            theme: theme,
            main_tree: None,
            next_leaf_id: 0,
            secondary_windows: Vec::new(),
            next_window_id: 0,
            // Spawns the background library scanner and points it at the
            // libraries persisted in config.
            libraries: LibrariesState::new(cc.egui_ctx.clone(), config.libraries.clone()),
            error_modal,
            pending_close: None,
            quit_requested: false,
        }
    }

    /// Renders the pending error modal (if any). Same `egui::Modal` pattern
    /// as the Libraries panel dialogs; colors come from the theme via the
    /// global visuals that `set_theme` installed, so no hardcoded chrome.
    fn show_error_modal(&mut self, ui: &mut egui::Ui) {
        let Some(message) = self.error_modal.clone() else {
            return;
        };

        let mut dismissed = false;
        let modal = egui::Modal::new(egui::Id::new("app_error_modal")).show(ui.ctx(), |ui| {
            ui.set_width(320.0);
            ui.heading("error");
            ui.add_space(8.0);
            ui.label(message);
            ui.add_space(12.0);
            if ui.button("ok").clicked() {
                dismissed = true;
            }
        });

        // Esc or clicking outside also dismisses — an error notice needs
        // acknowledging, not deciding.
        if dismissed || modal.should_close() {
            self.error_modal = None;
        }
    }

    /// Requests closing a tab. Saved or empty tabs close immediately; an
    /// unsaved tab with content defers into the save/discard/cancel prompt
    /// (`pending_close`), which `show_close_prompt_modal` resolves. Only one
    /// close can be pending at a time — further requests are dropped until
    /// it resolves (the close button can simply be clicked again).
    fn request_close(&mut self, program_id: String) {
        if self.pending_close.is_some() {
            return;
        }
        let needs_prompt = self
            .programs
            .get(&program_id)
            .is_some_and(|p| p.has_unsaved_content());
        if needs_prompt {
            // Focus the tab so the user sees what they're deciding about —
            // and, load-bearing: only the focused tab's message pump
            // (`Program::update`) runs each frame, so the SavedTo ack a
            // save-then-close waits on would never arrive for a background
            // tab.
            self.current_program = Some(program_id.clone());
            self.pending_close = Some(PendingClose::Prompt { program_id });
        } else {
            self.close_program(&program_id);
        }
    }

    /// Removes a tab: aborts its engine task and moves focus to some other
    /// tab if the closed one was current. No file cleanup — unsaved graphs
    /// have no file, and saved graphs keep theirs (auto-save flushed any
    /// pending edits; for a just-saved closing tab we waited for the
    /// engine's SavedTo ack before getting here).
    fn close_program(&mut self, program_id: &str) {
        if let Some(program) = self.programs.remove(program_id) {
            program.app.thread_handle.abort();
            drop(program);
            if self.current_program.as_deref() == Some(program_id) {
                self.current_program = self.programs.keys().next().cloned();
            }
        }
    }

    /// Renders and resolves the unsaved-close prompt (see [`PendingClose`]).
    /// Same `egui::Modal` pattern as `show_error_modal`; chrome colors come
    /// from the theme via the global visuals.
    fn show_close_prompt_modal(&mut self, ui: &mut egui::Ui) {
        let Some(pending) = self.pending_close.clone() else {
            return;
        };

        let program_id = match &pending {
            PendingClose::Prompt { program_id } => program_id.clone(),
            PendingClose::AwaitingSave { program_id, .. } => program_id.clone(),
        };

        // The tab vanished out from under the prompt (shouldn't happen —
        // request_close is the only closer — but don't wedge the modal).
        if !self.programs.contains_key(&program_id) {
            self.pending_close = None;
            return;
        }

        // Resolve a pending save before rendering: the focused program's
        // message pump ran earlier this frame, so a SavedTo ack from the
        // engine is visible now.
        if let PendingClose::AwaitingSave { since, .. } = &pending {
            let confirmed = self
                .programs
                .get_mut(&program_id)
                .and_then(|p| p.take_confirmed_save())
                .is_some();
            if confirmed {
                self.pending_close = None;
                self.close_program(&program_id);
                return;
            }
            if since.elapsed() > std::time::Duration::from_secs(5) {
                // The write never got confirmed (engine's SaveError details
                // already surfaced as the tab's status message). Keep the
                // tab and its in-memory graph alive rather than closing
                // over an unconfirmed save.
                self.pending_close = None;
                self.quit_requested = false;
                self.error_modal = Some(
                    "couldn't save the graph — the tab was left open. Check the save location and try again."
                        .to_string(),
                );
                return;
            }
        }

        let display_name = self
            .programs
            .get(&program_id)
            .map(|p| p.display_name())
            .unwrap_or_default();
        let awaiting_save = matches!(pending, PendingClose::AwaitingSave { .. });

        let mut chose_save = false;
        let mut chose_discard = false;
        let mut chose_cancel = false;
        let modal = egui::Modal::new(egui::Id::new("close_prompt_modal")).show(ui.ctx(), |ui| {
            ui.set_width(320.0);
            ui.heading("unsaved graph");
            ui.add_space(8.0);
            ui.label(format!("'{}' has never been saved.", display_name));
            ui.add_space(12.0);
            if awaiting_save {
                ui.horizontal(|ui| {
                    ui.spinner();
                    ui.label("saving…");
                });
            } else {
                ui.label("Save it before closing?");
                ui.add_space(12.0);
                ui.horizontal(|ui| {
                    if ui.button("save").clicked() {
                        chose_save = true;
                    }
                    if ui.button("discard").clicked() {
                        chose_discard = true;
                    }
                    if ui.button("cancel").clicked() {
                        chose_cancel = true;
                    }
                });
            }
        });

        if awaiting_save {
            // Ignore Esc/outside clicks while the (sub-second) save is in
            // flight — the wait resolves above on ack or timeout.
            return;
        }

        if chose_save {
            // Seed the dialog with the default library so first saves land
            // where the Libraries panel can see them (same seed as the
            // settings panel's "save graph" button).
            let mut config = AppConfig::load();
            let default_dir = config.ensure_default_library();
            config.save();

            if let Some(path) = crate::settings::graph_settings_panel::choose_graph_save_path(
                default_dir.as_deref(),
                &display_name,
            ) {
                if let Some(program) = self.programs.get_mut(&program_id) {
                    program.set_save_location(path);
                }
                self.pending_close = Some(PendingClose::AwaitingSave {
                    program_id,
                    since: std::time::Instant::now(),
                });
            }
            // Dialog cancelled: stay in Prompt, the modal remains.
        } else if chose_discard {
            self.pending_close = None;
            self.close_program(&program_id);
            // An active quit flow naturally advances to the next unsaved
            // tab (or the final Close) next frame.
        } else if chose_cancel || modal.should_close() {
            // Esc / outside click also cancels — unlike the file-conflict
            // modal there IS a safe "neither" answer here.
            self.pending_close = None;
            self.quit_requested = false;
        }
    }

    /// Creates a brand-new blank program tab (the menu bar's "new" button).
    /// The graph is in-memory and unsaved — no file exists until the user
    /// saves it from graph settings (or via the close prompt).
    fn create_new_program(&mut self) {
        match Program::new(None, None) {
            Ok(program) => {
                let id = program.app.id.clone();
                self.programs.insert(id.clone(), program);
                self.current_program = Some(id);
            }
            Err(error) => {
                self.error_modal = Some(format!("Failed to create a new graph: {}", error.0));
            }
        }
    }

    /// Opens `path` as a new program tab, or focuses the existing tab
    /// already editing it (two engines auto-saving one file would clobber
    /// each other). Shared by the Libraries panel's open action and the menu
    /// bar's "load" button.
    fn open_or_focus(&mut self, path: PathBuf) {
        let already_open = self
            .programs
            .iter()
            .find(|(_, p)| p.app.save_path.as_deref() == Some(path.as_path()))
            .map(|(id, _)| id.clone());
        if let Some(id) = already_open {
            self.current_program = Some(id);
            return;
        }

        // With tolerant loading in core, failure here means real IO/JSON
        // corruption — tell the user instead of the old silent "double-click
        // does nothing".
        match Program::new(None, Some(path.clone())) {
            Ok(program) => {
                let id = program.app.id.clone();
                self.programs.insert(id.clone(), program);
                self.current_program = Some(id);
            }
            Err(error) => {
                self.error_modal = Some(format!(
                    "Failed to open '{}': {}",
                    path.display(),
                    error.0
                ));
            }
        }
    }

    /// Perform a request raised by the Libraries panel. These need the
    /// programs map (open/create a graph = a new tab; a rename must
    /// re-target open tabs), which the panel itself can't touch.
    fn handle_library_action(&mut self, action: crate::libraries::libraries_state::LibraryAction) {
        use crate::libraries::libraries_state::LibraryAction;
        match action {
            LibraryAction::OpenGraph { path } => self.open_or_focus(path),
            LibraryAction::CreateGraph { path, name } => {
                // A blank tab pointed at the target path: the engine writes
                // the file immediately on SetSavePath (this is the one
                // "new graph" flow that starts saved — the user explicitly
                // created it inside a library), and the library scanner
                // picks it up on its next pass.
                match Program::new(None, None) {
                    Ok(mut program) => {
                        program.set_save_location(path);
                        let id = program.app.id.clone();
                        self.programs.insert(id.clone(), program);
                        self.current_program = Some(id);
                    }
                    Err(error) => {
                        self.error_modal = Some(format!(
                            "Failed to create graph '{}': {}",
                            name, error.0
                        ));
                    }
                }
            }
            LibraryAction::PathRenamed { from, to } => {
                // Any open tab still auto-saving to the old path follows the
                // file, otherwise its next save would resurrect it at the old
                // location. The tab's display name is derived from the file
                // stem, so it updates itself once the path changes — nothing
                // to patch. And for a graph that ISN'T open in a tab there's
                // nothing to do either: `Graph::load` ignores the embedded
                // name and re-derives it from the (now-renamed) file name.
                for program in self.programs.values_mut() {
                    if program.app.save_path.as_deref() == Some(from.as_path()) {
                        program.set_save_location(to.clone());
                    }
                }
            }
            LibraryAction::AddImageNode { path } => {
                // Target the focused program's graph.
                if let Some(id) = self.current_program.clone() {
                    if let Some(program) = self.programs.get_mut(&id) {
                        program.add_image_from_file(path);
                    }
                }
            }
            LibraryAction::BeginImageDrag { path } => {
                // Hand the drag to the focused program; it draws the ghost and
                // drops the node when the drag ends over a graph panel.
                if let Some(id) = self.current_program.clone() {
                    if let Some(program) = self.programs.get_mut(&id) {
                        program.begin_library_image_drag(path);
                    }
                }
            }
            LibraryAction::PreviewImage { path } => {
                // Show the image in the focused program's 2D preview panel.
                if let Some(id) = self.current_program.clone() {
                    if let Some(program) = self.programs.get_mut(&id) {
                        if let Err(err) = program.preview_library_image(path.clone()) {
                            self.libraries.set_error(format!(
                                "couldn't preview '{}': {}",
                                path.display(),
                                err
                            ));
                        }
                    }
                }
            }
        }
    }

    /// Apply a panel-management command. Split/close carry their own target
    /// window/leaf (raised by that panel's corner-button menu), so they act
    /// directly on it — no lookup needed. NewWindow, SaveLayoutAsDefault, and
    /// ResetLayout come from the app settings menu instead: NewWindow always
    /// opens a 2D preview, and SaveLayoutAsDefault/ResetLayout are
    /// main-window-only by design.
    fn handle_panel_action(&mut self, action: PanelAction, work_rect: Rect) {
        match action {
            PanelAction::NewWindow => {
                // New secondary windows always start as a 2D preview.
                let kind = PanelKind::Preview2D;
                let leaf = self.next_leaf_id;
                self.next_leaf_id += 1;
                let id = self.next_window_id;
                self.next_window_id += 1;
                self.secondary_windows.push(SecondaryWindow {
                    id,
                    tree: PanelTree::single(kind, leaf),
                    close_requested: false,
                });
            }
            PanelAction::SplitHorizontal { window, leaf } | PanelAction::SplitVertical { window, leaf } => {
                let direction = if matches!(action, PanelAction::SplitHorizontal { .. }) {
                    SplitDirection::Row
                } else {
                    SplitDirection::Column
                };
                let new_id = self.next_leaf_id;
                self.next_leaf_id += 1;
                if let Some(tree) = self.tree_mut(window) {
                    tree.split(leaf, direction, new_id);
                }
            }
            PanelAction::ClosePanel { window, leaf } => {
                let result = match self.tree_mut(window) {
                    Some(tree) => tree.close(leaf),
                    None => return,
                };
                match result {
                    Ok(()) => {
                        self.prune_all_viewers();
                    }
                    Err(CloseError::IsRoot) => {
                        // Closing a secondary window's last panel closes the
                        // window; on the main window it is a no-op.
                        if let PanelWindowId::Secondary(wid) = window {
                            if let Some(win) =
                                self.secondary_windows.iter_mut().find(|w| w.id == wid)
                            {
                                win.close_requested = true;
                            }
                        }
                    }
                    Err(CloseError::NotFound) => {}
                }
            }
            PanelAction::SaveLayoutAsDefault => {
                if let Some(tree) = &self.main_tree {
                    let mut config = AppConfig::load();
                    config.default_layout = Some(tree.root.clone());
                    config.save();
                }
            }
            PanelAction::ResetLayout => {
                self.main_tree =
                    Some(PanelTree::system_default(work_rect.width(), &mut self.next_leaf_id));
                self.prune_all_viewers();
            }
        }
    }

    /// The panel tree owned by a window, if it still exists.
    fn tree_mut(&mut self, window: PanelWindowId) -> Option<&mut PanelTree> {
        match window {
            PanelWindowId::Main => self.main_tree.as_mut(),
            PanelWindowId::Secondary(wid) => self
                .secondary_windows
                .iter_mut()
                .find(|w| w.id == wid)
                .map(|w| &mut w.tree),
        }
    }

    /// Whether any panel tree (main window or a secondary window) currently
    /// has a Preview2D leaf open.
    fn has_preview_2d_panel(&self) -> bool {
        let is_preview_2d = |leaves: &[(LeafId, PanelKind)]| {
            leaves.iter().any(|(_, kind)| *kind == PanelKind::Preview2D)
        };
        self.main_tree
            .as_ref()
            .is_some_and(|t| is_preview_2d(&t.leaves()))
            || self
                .secondary_windows
                .iter()
                .any(|w| is_preview_2d(&w.tree.leaves()))
    }

    /// Union of live leaf ids across the main tree and all secondary windows.
    fn live_leaf_ids(&self) -> HashSet<LeafId> {
        let mut live = HashSet::new();
        if let Some(tree) = &self.main_tree {
            live.extend(tree.leaves().iter().map(|(id, _)| *id));
        }
        for win in &self.secondary_windows {
            live.extend(win.tree.leaves().iter().map(|(id, _)| *id));
        }
        live
    }

    /// Prune per-leaf viewer state in every program to the currently live
    /// leaves (main tree + all secondary windows).
    fn prune_all_viewers(&mut self) {
        let live = self.live_leaf_ids();
        for program in self.programs.values_mut() {
            program.prune_viewers(&live);
        }
    }
}

fn setup_fonts(ctx: &egui::Context) {
    // Start with the default fonts (we will be adding to them rather than replacing them).
    let mut fonts = egui::FontDefinitions::default();

    egui_phosphor::add_to_fonts(&mut fonts, egui_phosphor::Variant::Regular);

    std::sync::Arc::make_mut(fonts.font_data.get_mut("phosphor").unwrap()).tweak.y_offset_factor = 0.1;

    // Install my own font (maybe supporting non-latin characters).
    // .ttf and .otf files supported.
    fonts.font_data.insert(
        "manrope-light".to_owned(),
        std::sync::Arc::new(egui::FontData::from_static(include_bytes!(
            "../assets/Manrope-Light.ttf"
        ))),
    );

    // Put my font first (highest priority) for proportional text:
    fonts
        .families
        .entry(egui::FontFamily::Proportional)
        .or_default()
        .insert(0, "manrope-light".to_owned());

    // Semibold weight for panel titles ("bold" per the redesign) — a
    // dedicated named family rather than swapping the whole Proportional
    // family, so most text stays on Manrope-Light and only call sites that
    // opt in via `semibold_family()` get the heavier weight.
    fonts.font_data.insert(
        "manrope-semibold".to_owned(),
        std::sync::Arc::new(egui::FontData::from_static(include_bytes!(
            "../assets/Manrope-SemiBold.ttf"
        ))),
    );

    // Clone the (already-finalized) Proportional fallback list so the
    // semibold family still falls back to phosphor icons and the default
    // fonts for glyphs Manrope-SemiBold doesn't cover (non-latin scripts,
    // symbols), same as the regular Proportional family does.
    let mut semibold_fallbacks = fonts
        .families
        .get(&egui::FontFamily::Proportional)
        .cloned()
        .unwrap_or_default();
    semibold_fallbacks.insert(0, "manrope-semibold".to_owned());
    fonts.families.insert(
        egui::FontFamily::Name("semibold".into()),
        semibold_fallbacks,
    );

    // Tell egui to use these fonts:
    ctx.set_fonts(fonts);
}