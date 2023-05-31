#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")] // hide console window on Windows in release

use app_bar::bar;
use eframe::egui;
use program::Program;
use std::collections::HashMap;
use std::path::Path;
mod app_bar;
mod graph;
mod menu;
mod program;
mod settings;
mod title_bar;
mod view;
use egui::Pos2;

pub const PROFILE: bool = false;
pub const DEFAULT_WINDOW_WIDTH: f32 = 1280.0;
pub const DEFAULT_WINDOW_HEIGHT: f32 = 800.0;
pub const APP_MENU_HEIGHT: f32 = 35.0;
//const ICON: &[u8; 2869] = include_bytes!("..\\assets\\mangler_icon.png");

#[tokio::main]
async fn main() -> Result<(), eframe::Error> {
    puffin::set_scopes_on(PROFILE);

    let icon_path = Path::new(env!("CARGO_MANIFEST_DIR")).join("assets/mangler_icon.png");

    let options = eframe::NativeOptions {
        initial_window_size: Some(egui::vec2(DEFAULT_WINDOW_WIDTH, DEFAULT_WINDOW_HEIGHT)),
        icon_data: Some(load_icon(icon_path.to_str().unwrap())),
        //maximized: true,
        drag_and_drop_support: true,
        resizable: true,
        decorated: false,
        ..Default::default()
    };

    let my_app = ManglerApp::new();

    eframe::run_native(
        "Mangler",
        options,
        Box::new(|_cc| {
            //let frame = cc.egui_ctx.clone();
            Box::<ManglerApp>::new(my_app)
        }),
    )
}

// do this without image crate?
fn load_icon(path: &str) -> eframe::IconData {
    let (icon_rgba, icon_width, icon_height) = {
        let image = image::open(path)
            .expect("Failed to open icon path")
            .into_rgba8();
        let (width, height) = image.dimensions();
        let rgba = image.into_raw();
        (rgba, width, height)
    };

    eframe::IconData {
        rgba: icon_rgba,
        width: icon_width,
        height: icon_height,
    }
}

struct ManglerApp {
    programs: HashMap<String, Program>,
    current_program: Option<String>,
}

impl eframe::App for ManglerApp {
    fn update(&mut self, ctx: &egui::Context, frame: &mut eframe::Frame) {
        if PROFILE {
            puffin::profile_function!();
            puffin::GlobalProfiler::lock().new_frame(); // call once per frame!

            puffin_egui::profiler_window(ctx);
        }

        egui::CentralPanel::default().show(ctx, |ui| {
            let bar_response = bar::show(ctx, frame, ui, &self.programs, &self.current_program);

            if let Some(new_program) = bar_response.new_program {
                let program_id = new_program.id.clone();
                self.programs.insert(new_program.id.clone(), new_program);
                self.current_program = Some(program_id);
            }

            if let Some(current_program) = bar_response.current_program {
                self.current_program = Some(current_program);
            }

            if let Some(current_program) = &self.current_program {
                if let Some(program) = self.programs.get_mut(current_program) {
                    program.show(ctx, frame, ui);
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

        // for (_program_id, program) in self.programs.iter_mut() {
        //     if program.needs_to_save {
        //         program.save_to_file();
        //     }
        // }
    }
}

impl ManglerApp {
    pub fn new() -> Self {
        Self {
            programs: HashMap::new(),
            current_program: None,
        }
    }
}

#[derive(Debug)]
pub struct NewConnection {
    input_node_id: String,
    input_connection_index: usize,
    output_node_id: String,
    output_connection_index: usize,
}

impl NewConnection {
    pub fn new(
        input_node_id: String,
        input_connection_index: usize,
        output_node_id: String,
        output_connection_index: usize,
    ) -> NewConnection {
        NewConnection {
            input_node_id,
            input_connection_index,
            output_node_id,
            output_connection_index,
        }
    }
}

pub fn view_to_graph_space(zoom: f32, n: f32) -> f32 {
    n * zoom
}

pub fn view_to_graph_space_pos2(zoom: f32, n: Pos2) -> Pos2 {
    Pos2::new(
        view_to_graph_space(zoom, n.x),
        view_to_graph_space(zoom, n.y),
    )
}

pub fn graph_to_view_space(zoom: f32, n: f32) -> f32 {
    n / zoom
}

pub fn graph_to_view_space_pos2(zoom: f32, n: Pos2) -> Pos2 {
    Pos2::new(
        graph_to_view_space(zoom, n.x),
        graph_to_view_space(zoom, n.y),
    )
}
