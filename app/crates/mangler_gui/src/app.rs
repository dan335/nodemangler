use crate::{
    app_menu::app_menu::AppMenu,
    config::AppConfig,
    panels::{
        panel_kind::PanelKind,
        panel_tree::{CloseError, LeafId, PanelTree, SplitDirection},
        panel_view::{self, PanelAction, PanelFocus, PanelWindowId},
        panel_windows::{self, SecondaryWindow},
    },
    themes::theme::{set_theme, Theme},
    APP_MENU_HEIGHT,
};
use eframe::egui;
use epaint::{pos2, CornerRadius, Rect};
use crate::program::Program;
use std::collections::{HashMap, HashSet};

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
    /// Last-hovered panel — the target for split/close actions.
    focused: Option<PanelFocus>,
    /// Secondary OS windows, each hosting its own panel tree (session-only).
    secondary_windows: Vec<SecondaryWindow>,
    /// Monotonic allocator for secondary window ids.
    next_window_id: u64,
}

impl eframe::App for App {
    fn ui(&mut self, outer_ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        if PROFILE {
            puffin::profile_function!();
            puffin::GlobalProfiler::lock().new_frame(); // call once per frame!
            // puffin_egui::profiler_window(ctx); // disabled: puffin_egui not compatible with egui 0.33
        }

        let ctx = outer_ui.ctx().clone();
        egui::CentralPanel::default().show_inside(outer_ui, |ui| {
            // bg
            ui.painter().add(egui::Shape::rect_filled(
                ui.max_rect(),
                CornerRadius::ZERO,
                self.theme.get().panel_fill,
            ));

            let bar_response = self.app_menu.show(&ctx, ui, &self.programs, &self.current_program, &self.theme);

            if let Some(new_program) = bar_response.new_program {
                let program_id = new_program.app.id.clone();
                self.programs.insert(new_program.app.id.clone(), new_program);
                self.current_program = Some(program_id);
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

            if let Some(current_program) = &self.current_program {
                let has_preview_2d_panel = self.has_preview_2d_panel();
                if let Some(program) = self.programs.get_mut(current_program) {
                    program.has_preview_2d_panel = has_preview_2d_panel;
                    program.update(&ctx, ui);
                    let resp = panel_view::render_tree(
                        ui,
                        self.main_tree.as_mut().unwrap(),
                        work_rect,
                        PanelWindowId::Main,
                        &mut self.focused,
                        program,
                        &self.theme,
                    );
                    program.show_overlays(&ctx, ui, &self.theme, &resp.graph_rects, work_rect);

                    // Secondary windows render the same current program. These
                    // App fields are all disjoint from `program`, so borrow
                    // them directly (no method calls on `self` here).
                    for win in &mut self.secondary_windows {
                        panel_windows::show_secondary_window(
                            &ctx,
                            win,
                            &mut self.focused,
                            program,
                            &self.theme,
                        );
                    }
                }
            }

            // Reap secondary windows whose titlebar close button was pressed.
            if self.secondary_windows.iter().any(|w| w.close_requested) {
                // Drop focus if it points into a window that is going away.
                if let Some(PanelFocus {
                    window: PanelWindowId::Secondary(wid),
                    ..
                }) = self.focused
                {
                    if self
                        .secondary_windows
                        .iter()
                        .any(|w| w.id == wid && w.close_requested)
                    {
                        self.focused = None;
                    }
                }
                self.secondary_windows.retain(|w| !w.close_requested);
                // Prune viewers to the leaves that remain live.
                self.prune_all_viewers();
            }

            if let Some(program_id_to_close) = bar_response.program_to_close {
                if let Some(program) = self.programs.remove(&program_id_to_close) {
                    program.close();
                    if self.current_program == Some(program_id_to_close) {
                        self.current_program = None;

                        if let Some(next_program_id) = self.programs.keys().next() {
                            self.current_program = Some(next_program_id.clone());
                        }
                    }
                }
            }
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
}

impl App {
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        setup_fonts(&cc.egui_ctx);

        // Load persistent config.
        let config = AppConfig::load();

        // Restore theme from config, or use default.
        let theme = config.theme.as_deref()
            .and_then(Theme::from_name)
            .unwrap_or(crate::DEFAULT_THEME);
        set_theme(&cc.egui_ctx, theme.clone());

        let mut programs = HashMap::new();
        let mut current_program: Option<String> = None;

        if let Ok(program) = Program::new(None, None) {
            current_program = Some(program.app.id.clone());
            programs.insert(program.app.id.clone(), program);
        }

        Self {
            app_menu: AppMenu::new(),
            programs: programs,
            current_program: current_program,
            theme: theme,
            main_tree: None,
            next_leaf_id: 0,
            focused: None,
            secondary_windows: Vec::new(),
            next_window_id: 0,
        }
    }

    /// Apply a panel-management command from the settings menu. Split/close
    /// act on the focused panel — which may live in a secondary window —
    /// falling back to the main tree's first leaf. SaveLayoutAsDefault and
    /// ResetLayout are main-window-only by design.
    fn handle_panel_action(&mut self, action: PanelAction, work_rect: Rect) {
        match action {
            PanelAction::NewWindow => {
                // Initial kind = focused panel's kind if it still exists, else
                // a 2D preview.
                let kind = self.focused_kind().unwrap_or(PanelKind::Preview2D);
                let leaf = self.next_leaf_id;
                self.next_leaf_id += 1;
                let id = self.next_window_id;
                self.next_window_id += 1;
                self.secondary_windows.push(SecondaryWindow {
                    id,
                    tree: PanelTree::single(kind, leaf),
                    close_requested: false,
                });
                self.focused = Some(PanelFocus {
                    window: PanelWindowId::Secondary(id),
                    leaf,
                });
            }
            PanelAction::SplitHorizontal | PanelAction::SplitVertical => {
                let direction = if matches!(action, PanelAction::SplitHorizontal) {
                    SplitDirection::Row
                } else {
                    SplitDirection::Column
                };
                let Some((window, target)) = self.resolve_target() else {
                    return;
                };
                let new_id = self.next_leaf_id;
                self.next_leaf_id += 1;
                if let Some(tree) = self.tree_mut(window) {
                    if tree.split(target, direction, new_id) {
                        self.focused = Some(PanelFocus {
                            window,
                            leaf: target,
                        });
                    }
                }
            }
            PanelAction::ClosePanel => {
                let Some((window, target)) = self.resolve_target() else {
                    return;
                };
                let result = match self.tree_mut(window) {
                    Some(tree) => tree.close(target),
                    None => return,
                };
                match result {
                    Ok(()) => {
                        self.focused = None;
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
                self.focused = None;
                self.prune_all_viewers();
            }
        }
    }

    /// The window + leaf a split/close action targets: the focused panel if it
    /// still exists, else the main tree's first leaf.
    fn resolve_target(&self) -> Option<(PanelWindowId, LeafId)> {
        if let Some(focus) = self.focused {
            let exists = match focus.window {
                PanelWindowId::Main => self
                    .main_tree
                    .as_ref()
                    .is_some_and(|t| t.contains(focus.leaf)),
                PanelWindowId::Secondary(wid) => self
                    .secondary_windows
                    .iter()
                    .any(|w| w.id == wid && w.tree.contains(focus.leaf)),
            };
            if exists {
                return Some((focus.window, focus.leaf));
            }
        }
        self.main_tree
            .as_ref()
            .map(|t| (PanelWindowId::Main, t.first_leaf()))
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

    /// The kind of the focused leaf, if the focused panel still exists.
    fn focused_kind(&self) -> Option<PanelKind> {
        let focus = self.focused?;
        let leaves = match focus.window {
            PanelWindowId::Main => self.main_tree.as_ref()?.leaves(),
            PanelWindowId::Secondary(wid) => {
                self.secondary_windows.iter().find(|w| w.id == wid)?.tree.leaves()
            }
        };
        leaves
            .into_iter()
            .find(|(id, _)| *id == focus.leaf)
            .map(|(_, kind)| kind)
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

    // Tell egui to use these fonts:
    ctx.set_fonts(fonts);
}