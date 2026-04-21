use std::collections::HashMap;

use eframe::egui::{self, Layout};
use epaint::{CornerRadius, Pos2, Rect};

use crate::{program::Program, themes::theme::Theme, APP_MENU_HEIGHT};

pub struct AppMenu;

impl AppMenu {
    pub fn new() -> Self {
        Self
    }

    pub fn show(
        &mut self,
        ctx: &egui::Context,
        ui: &mut egui::Ui,
        programs: &HashMap<String, Program>,
        current_program: &Option<String>,
        current_theme: &Theme,
        view_in_separate_window: &mut bool,
    ) -> BarResponse {
        // save current theme
        // show that we know which to highlight

        // draw background
        let app_menu_rect = Rect::from_two_pos(
            Pos2::ZERO,
            Pos2::new(ctx.content_rect().max.x, APP_MENU_HEIGHT),
        );
        ui.painter().add(egui::Shape::rect_filled(
            app_menu_rect,
            CornerRadius::ZERO,
            current_theme.get().menu_bar,
        ));

        let bar_response = self.show_menu(
            ui,
            programs,
            current_program,
            app_menu_rect,
            current_theme,
            view_in_separate_window,
        );

        bar_response
    }

    pub fn show_menu(
        &self,
        ui: &mut egui::Ui,
        programs: &HashMap<String, Program>,
        current_program: &Option<String>,
        app_menu_rect: Rect,
        current_theme: &Theme,
        view_in_separate_window: &mut bool,
    ) -> BarResponse {
        let mut bar_response = BarResponse::new();

        //ui.spacing_mut().item_spacing = egui::vec2(0.0, ui.spacing_mut().item_spacing.y);

        ui.scope_builder(egui::UiBuilder::new().max_rect(app_menu_rect), |ui| {
            ui.allocate_ui_with_layout(
                app_menu_rect.size(),
                Layout::left_to_right(egui::Align::TOP),
                |ui| {
                    ui.horizontal(|ui| {
                        ui.add_space(15.0);

                        egui::Frame::NONE.inner_margin(8.0).show(ui, |ui| {
                            if ui.button("new").clicked() {
                                if let Ok(new_program) = Program::new(None, None) {
                                    bar_response.new_program = Some(new_program);
                                }
                            }

                            //ui.add_space(10.0);

                            if ui.button("load").clicked() {
                                if let Some(save_path) = rfd::FileDialog::new()
                                    .add_filter("mangler", &["mangle.json", "json"])
                                    .pick_file()
                                {
                                    if let Ok(new_program) = Program::new(None, Some(save_path)) {
                                        bar_response.new_program = Some(new_program);
                                    }
                                }
                            }

                            //ui.add_space(10.0);

                            ui.menu_button("settings", |ui| {
                                //egui::Frame::NONE.inner_margin(8.0).show(ui, |ui| {
                                //ui.spacing_mut().item_spacing.y = 8.0;

                                ui.menu_button("theme", |ui| {
                                    //egui::Frame::NONE.inner_margin(8.0).show(ui, |ui| {
                                    for theme in Theme::list().iter() {
                                        if theme == current_theme {
                                            ui.button(theme.name()).highlight();
                                        } else {
                                            if ui.button(theme.name()).clicked() {
                                                bar_response.theme_changed_to = Some(theme.clone());
                                            }
                                        }
                                    }
                                    //});
                                });
                                ui.checkbox(view_in_separate_window, "viewer in separate window");
                                //});
                            });
                        });

                        ui.add_space(20.0);

                        // info about programs
                        // id, name
                        // sorted
                        let mut program_list: Vec<(String, String)> = Vec::new();

                        // sort programs and put into list
                        for (program_id, program) in programs.iter() {
                            program_list.push((program_id.clone(), program.app.name.clone()));
                        }

                        program_list.sort_by(|a, b| {
                            a.1.partial_cmp(&b.1)
                                .unwrap()
                                .then(a.0.partial_cmp(&b.0).unwrap())
                        });

                        // show programs
                        for (program_id, program_name) in program_list.iter() {
                            let r = egui::Frame::NONE.inner_margin(8.0).show(ui, |ui| {
                                let name = program_name.clone();

                                if current_program == &Some(program_id.clone()) {
                                    ui.label(name);
                                } else {
                                    if ui.button(name).clicked() {
                                        bar_response.current_program = Some(program_id.clone());
                                    }
                                }

                                if ui.button("X").clicked() {
                                    bar_response.program_to_close = Some(program_id.clone());
                                }
                            });

                            if current_program == &Some(program_id.clone()) {
                                ui.painter().add(egui::Shape::rect_filled(
                                    egui::Rect::from_min_max(
                                        Pos2::new(r.response.rect.left(), APP_MENU_HEIGHT - 3.0),
                                        Pos2::new(r.response.rect.right(), APP_MENU_HEIGHT),
                                    ),
                                    CornerRadius::ZERO,
                                    current_theme.get().menu_bar_button_selected,
                                ));
                            }

                            ui.add_space(10.0);
                        }
                    });
                },
            );
        });

        bar_response
    }
}

// pub fn show(
//     ctx: &egui::Context,
//     frame: &mut eframe::Frame,
//     ui: &mut egui::Ui,
//     programs: &HashMap<String, Program>,
//     current_program: &Option<String>,
//     theme: &Theme,
// ) -> BarResponse {
//     let app_rect = ctx.screen_rect();
//     let app_menu_rect = Rect::from_two_pos(Pos2::ZERO, Pos2::new(app_rect.max.x, APP_MENU_HEIGHT));

//     let rounding = CornerRadius::ZERO;

//     ui.painter().add(egui::Shape::rect_filled(
//         app_menu_rect,
//         rounding,
//         theme.get().menu_bar,
//     ));

//     let mut bar_response = show_menu(ui, programs, current_program, app_menu_rect, theme);

//     bar_response
// }

// pub fn show_window_control_menu(ctx: &egui::Context, frame: &mut eframe::Frame, ui: &mut egui::Ui, theme: &Theme) -> Option<Theme> {
//     let mut new_theme = None;

//     let app_rect = ctx.screen_rect();
//     let app_menu_rect = Rect::from_two_pos(
//         Pos2::new(app_rect.max.x, 0.0),
//         Pos2::new(app_rect.max.x, APP_MENU_HEIGHT),
//     );

//     //let rounding = CornerRadius::ZERO;

//     ui.scope_builder(egui::UiBuilder::new().max_rect(app_menu_rect), |ui| {
//         ui.allocate_ui_with_layout(
//             app_menu_rect.size(),
//             Layout::right_to_left(egui::Align::Center),
//             |ui| {
//                 ui.horizontal(|ui| {

//                     ui.add_space(15.0);

//                     if ui.add(egui::Button::new("🗙").frame(false)).clicked() {
//                         frame.close();
//                     }

//                     if frame.info().window_info.maximized {
//                         if ui.add(egui::Button::new("🗗").frame(false)).clicked() {
//                             frame.set_maximized(false);
//                         }
//                     } else if ui.add(egui::Button::new("🗖").frame(false)).clicked() {
//                         frame.set_maximized(true);
//                     }

//                     if ui.add(egui::Button::new("🗕").frame(false)).clicked() {
//                         frame.set_minimized(true);
//                     }

//                     ui.add_space(25.0);

//                     if ui.add(egui::Button::new("theme").frame(false)).clicked() {
//                         let theme = if ui.visuals().dark_mode { Theme::Light } else { Theme::DarkGreen };
//                         set_theme(ctx, theme.clone());
//                         new_theme = Some(theme);
//                     }
//                 });
//             },
//         );
//     });

//     new_theme
// }

pub struct BarResponse {
    pub new_program: Option<Program>,
    pub current_program: Option<String>,
    pub program_to_close: Option<String>,
    pub theme_changed_to: Option<Theme>,
}

impl BarResponse {
    pub fn new() -> Self {
        Self {
            new_program: None,
            current_program: None,
            program_to_close: None,
            theme_changed_to: None,
        }
    }
}
