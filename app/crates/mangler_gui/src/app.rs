use crate::{
    app_menu::app_menu::AppMenu,
    config::AppConfig,
    panels::{
        panel_tree::{LeafId, PanelTree, SplitDirection},
        panel_view::{self, PanelAction, PanelFocus, PanelWindowId},
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
                if let Some(program) = self.programs.get_mut(current_program) {
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
                }
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
        }
    }

    /// Apply a panel-management command from the settings menu to the main
    /// window's tree, targeting the focused panel (falling back to the first
    /// leaf) and pruning per-leaf viewer state when leaves disappear.
    fn handle_panel_action(&mut self, action: PanelAction, work_rect: Rect) {
        let Some(tree) = self.main_tree.as_mut() else {
            return;
        };

        // Resolve the target leaf: the focused panel if it still exists in the
        // main window, else the first leaf.
        let target = self
            .focused
            .filter(|f| f.window == PanelWindowId::Main && tree.contains(f.leaf))
            .map(|f| f.leaf)
            .unwrap_or_else(|| tree.first_leaf());

        match action {
            PanelAction::NewWindow => {
                // TODO(panel-system Phase 5): create a secondary OS window panel.
            }
            PanelAction::SplitHorizontal | PanelAction::SplitVertical => {
                let direction = if matches!(action, PanelAction::SplitHorizontal) {
                    SplitDirection::Row
                } else {
                    SplitDirection::Column
                };
                let new_id = self.next_leaf_id;
                self.next_leaf_id += 1;
                tree.split(target, direction, new_id);
                self.focused = Some(PanelFocus {
                    window: PanelWindowId::Main,
                    leaf: target,
                });
            }
            PanelAction::ClosePanel => {
                // `Err(IsRoot)` (last panel) and `Err(NotFound)` are no-ops.
                if tree.close(target).is_ok() {
                    self.focused = None;
                    let live: HashSet<LeafId> =
                        tree.leaves().iter().map(|(id, _)| *id).collect();
                    for program in self.programs.values_mut() {
                        program.prune_viewers(&live);
                    }
                }
            }
            PanelAction::SaveLayoutAsDefault => {
                let mut config = AppConfig::load();
                config.default_layout = Some(tree.root.clone());
                config.save();
            }
            PanelAction::ResetLayout => {
                *tree = PanelTree::system_default(work_rect.width(), &mut self.next_leaf_id);
                self.focused = None;
                let live: HashSet<LeafId> = tree.leaves().iter().map(|(id, _)| *id).collect();
                for program in self.programs.values_mut() {
                    program.prune_viewers(&live);
                }
            }
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