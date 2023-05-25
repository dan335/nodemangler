#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")] // hide console window on Windows in release

use eframe::egui::{self, Layout};
use epaint::{Rect, Rounding, Color32, Stroke};
use std::collections::HashMap;
use mangler::{
    SetNodeInputMessage, get_id,
};
use program::Program;
use std::path::Path;
mod graph;
mod menu;
mod settings;
mod title_bar;
mod view;
mod program;
use egui::{Pos2};

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
        maximized: true,
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
            self.show_app_menu(ctx, ui);
            
            if let Some(current_program) = &self.current_program {
                if let Some(program) = self.programs.get_mut(current_program) {
                    program.show(ctx, frame, ui);
                }
            }
        });
    }
}

impl ManglerApp {
    pub fn new() -> Self {
        Self {
            programs: HashMap::new(),
            current_program: None,
        }
    } 

    pub fn show_app_menu(&mut self, ctx: &egui::Context, ui: &mut egui::Ui) {
        let app_rect = ctx.screen_rect();
        let app_menu_rect = Rect::from_two_pos(Pos2::ZERO, Pos2::new(app_rect.max.x, APP_MENU_HEIGHT));

        let rounding = Rounding::none();
        let background_color = Color32::from_gray(20);
        
        ui.painter().add(egui::Shape::rect_filled(
            app_menu_rect,
            rounding,
            background_color,
        ));

        ui.allocate_ui_with_layout(app_menu_rect.size(), Layout::left_to_right(egui::Align::Min), |ui| {
            ui.horizontal(|ui| {
                if ui.add(egui::Button::new("New")).clicked() {
                    let id = get_id();
                    self.programs.insert(id.clone(), Program::new(id.clone()));
                    self.current_program = Some(id);
                }
        
                if ui.add(egui::Button::new("Load")).clicked() {
                    println!("New");
                }
    
                for (program_id, program) in self.programs.iter() {
                    let mut stroke = Stroke::NONE;

                    if self.current_program == Some(program_id.clone()) {
                        stroke = Stroke::new(2.0, Color32::from_gray(150))
                    }

                    if ui.add(egui::Button::new(&program.name).stroke(stroke)).clicked() {
                        self.current_program = Some(program_id.clone());
                    }
                }
            });
        });

        
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
    Pos2::new(view_to_graph_space(zoom, n.x), view_to_graph_space(zoom, n.y))
}

pub fn graph_to_view_space(zoom: f32, n: f32) -> f32 {
    n / zoom
}

pub fn graph_to_view_space_pos2(zoom: f32, n: Pos2) -> Pos2 {
    Pos2::new(graph_to_view_space(zoom, n.x), graph_to_view_space(zoom, n.y))
}