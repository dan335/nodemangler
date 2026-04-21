use crate::{app_menu::app_menu::AppMenu, config::AppConfig, themes::theme::{Theme, set_theme}};
use eframe::egui;
use epaint::CornerRadius;
use crate::program::Program;
use std::collections::HashMap;

pub const PROFILE: bool = false;
// pub const DEFAULT_WINDOW_WIDTH: f32 = 1280.0;
// pub const DEFAULT_WINDOW_HEIGHT: f32 = 800.0;
// pub const APP_MENU_HEIGHT: f32 = 35.0;


pub struct App {
    app_menu: AppMenu,
    programs: HashMap<String, Program>,
    current_program: Option<String>,
    theme: Theme,
    view_in_separate_window: bool,
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

            let bar_response = self.app_menu.show(&ctx, ui, &self.programs, &self.current_program, &self.theme, &mut self.view_in_separate_window);

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

            if let Some(current_program) = &self.current_program {
                if let Some(program) = self.programs.get_mut(current_program) {
                    program.show(&ctx, ui, &self.theme, self.view_in_separate_window);
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
            view_in_separate_window: true,
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