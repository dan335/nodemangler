#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")] // hide console window on Windows in release

use eframe::egui::{self};
use epaint::Vec2;
use themes::theme::Theme;
use std::path::Path;
mod app_menu;
mod graph;
mod node_menu;
mod program;
mod settings;
mod themes;
mod title_bar;
mod view_window;
mod app;
use egui::Pos2;

pub const PROFILE: bool = false;
pub const DEFAULT_WINDOW_WIDTH: f32 = 1280.0;
pub const DEFAULT_WINDOW_HEIGHT: f32 = 800.0;
pub const APP_MENU_HEIGHT: f32 = 35.0;
pub const NODE_MENU_WIDTH: f32 = 250.0;
pub const SETTINGS_PANEL_WIDTH: f32 = 250.0;
pub const DEFAULT_THEME: Theme = Theme::DarkGreen;
pub const NODE_SIZE: Vec2 = Vec2::new(150.0, 40.0);
pub const NODE_ROUNDING: f32 = 2.0;

#[tokio::main]
async fn main() -> Result<(), eframe::Error> {
    puffin::set_scopes_on(PROFILE);

    let icon_path = Path::new(env!("CARGO_MANIFEST_DIR")).join("assets/mangler_icon.png");

    let options = eframe::NativeOptions {
        //initial_window_size: Some(egui::vec2(DEFAULT_WINDOW_WIDTH, DEFAULT_WINDOW_HEIGHT)),
        icon_data: Some(load_icon(icon_path.to_str().unwrap())),
        maximized: true,
        //fullscreen: true,
        //drag_and_drop_support: true,
        resizable: true,
        decorated: true,
        ..Default::default()
    };

    eframe::run_native(
        "Mangler",
        options,
        Box::new(|cc| {
            Box::<app::App>::new(app::App::new(cc))
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



// generic error
pub struct ManglerError(String);