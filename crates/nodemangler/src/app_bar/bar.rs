use std::collections::HashMap;

use eframe::egui::{self, Layout};
use epaint::{Color32, Pos2, Rect, Rounding};

use crate::{program::Program, APP_MENU_HEIGHT};

pub fn show(
    ctx: &egui::Context,
    frame: &mut eframe::Frame,
    ui: &mut egui::Ui,
    programs: &HashMap<String, Program>,
    current_program: &Option<String>,
) -> BarResponse {
    let app_rect = ctx.screen_rect();
    let app_menu_rect = Rect::from_two_pos(Pos2::ZERO, Pos2::new(app_rect.max.x, APP_MENU_HEIGHT));

    let rounding = Rounding::none();
    let background_color = Color32::from_gray(20);

    ui.painter().add(egui::Shape::rect_filled(
        app_menu_rect,
        rounding,
        background_color,
    ));

    let input_response = ui.allocate_rect(
        app_menu_rect,
        egui::Sense::drag().union(egui::Sense::click()),
    );

    let bar_response = show_menu(ui, programs, current_program, app_menu_rect);
    show_window_control_menu(ctx, frame, ui);

    if input_response.dragged_by(egui::PointerButton::Primary) {
        frame.drag_window();
    }

    if input_response.double_clicked_by(egui::PointerButton::Primary) {
        if frame.info().window_info.maximized {
            frame.set_maximized(false);
        } else {
            frame.set_maximized(true);
        }
    }

    bar_response
}

pub fn show_menu(
    ui: &mut egui::Ui,
    programs: &HashMap<String, Program>,
    current_program: &Option<String>,
    app_menu_rect: Rect,
) -> BarResponse {
    let mut bar_response = BarResponse::new();

    ui.spacing_mut().item_spacing = egui::vec2(0.0, ui.spacing_mut().item_spacing.y);

    ui.allocate_ui_at_rect(app_menu_rect, |ui| {
        ui.allocate_ui_with_layout(
            app_menu_rect.size(),
            Layout::left_to_right(egui::Align::TOP),
            |ui| {
                ui.horizontal(|ui| {
                    ui.add_space(15.0);

                    egui::Frame::none().inner_margin(8.0).show(ui, |ui| {
                        if ui.add(egui::Button::new("New").frame(false)).clicked() {
                            if let Ok(new_program) = Program::new(None, None) {
                                bar_response.new_program = Some(new_program);
                            }
                        }

                        ui.add_space(10.0);

                        if ui.add(egui::Button::new("Load").frame(false)).clicked() {
                            if let Some(save_path) = rfd::FileDialog::new()
                                .add_filter("mangler", &["mangle"])
                                .pick_file()
                            {
                                if let Ok(new_program) = Program::new(None, Some(save_path)) {
                                    bar_response.new_program = Some(new_program);
                                }
                            }
                        }

                        ui.add_space(10.0);

                        if ui.add(egui::Button::new("Settings").frame(false)).clicked() {}
                    });

                    ui.add_space(20.0);

                    // info about programs
                    // id, name
                    // sorted
                    let mut program_list: Vec<(String, String)> = Vec::new();

                    // sort programs and put into list
                    for (program_id, program) in programs.iter() {
                        program_list.push((program_id.clone(), program.name.clone()));
                    }

                    program_list.sort_by(|a, b| {
                        a.1.partial_cmp(&b.1)
                            .unwrap()
                            .then(a.0.partial_cmp(&b.0).unwrap())
                    });

                    // show programs
                    for (program_id, program_name) in program_list.iter() {
                        //let mut stroke = Stroke::NONE;
                        let mut bg_color = Color32::from_gray(30);

                        if current_program == &Some(program_id.clone()) {
                            //stroke = Stroke::new(2.0, Color32::from_gray(150))
                            bg_color = Color32::from_gray(60);
                        }

                        egui::Frame::none()
                            .fill(bg_color)
                            .inner_margin(8.0)
                            .show(ui, |ui| {
                                let name = program_name.clone();

                                // if *needs_to_save {
                                //     name.push(' ');
                                //     name.push('*');
                                // }

                                if ui.add(egui::Button::new(name).frame(false)).clicked() {
                                    bar_response.current_program = Some(program_id.clone());
                                }

                                if ui.add(egui::Button::new("X").frame(false)).clicked() {
                                    bar_response.program_to_close = Some(program_id.clone());
                                }
                            });

                        ui.add_space(10.0);
                    }
                });
            },
        );
    });

    bar_response
}

pub fn show_window_control_menu(ctx: &egui::Context, frame: &mut eframe::Frame, ui: &mut egui::Ui) {
    let app_rect = ctx.screen_rect();
    let app_menu_rect = Rect::from_two_pos(
        Pos2::new(app_rect.max.x, 0.0),
        Pos2::new(app_rect.max.x, APP_MENU_HEIGHT),
    );

    //let rounding = Rounding::none();

    ui.allocate_ui_at_rect(app_menu_rect, |ui| {
        ui.allocate_ui_with_layout(
            app_menu_rect.size(),
            Layout::right_to_left(egui::Align::Center),
            |ui| {
                ui.horizontal(|ui| {
                    ui.add_space(15.0);

                    if ui.add(egui::Button::new("🗙").frame(false)).clicked() {
                        frame.close();
                    }

                    if frame.info().window_info.maximized {
                        if ui.add(egui::Button::new("🗗").frame(false)).clicked() {
                            frame.set_maximized(false);
                        }
                    } else {
                        if ui.add(egui::Button::new("🗖").frame(false)).clicked() {
                            frame.set_maximized(true);
                        }
                    }

                    if ui.add(egui::Button::new("🗕").frame(false)).clicked() {
                        frame.set_minimized(true);
                    }
                });
            },
        );
    });
}

pub struct BarResponse {
    pub new_program: Option<Program>,
    pub current_program: Option<String>,
    pub program_to_close: Option<String>,
}

impl BarResponse {
    pub fn new() -> Self {
        Self {
            new_program: None,
            current_program: None,
            program_to_close: None,
        }
    }
}
