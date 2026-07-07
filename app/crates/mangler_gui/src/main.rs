#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")] // hide console window on Windows in release

use eframe::egui::{self};
use epaint::Vec2;
use themes::theme::Theme;
mod app;
mod app_menu;
mod config;
mod graph;
mod libraries;
mod node_menu;
mod panels;
mod pan_zoom;
mod program;
mod settings;
mod themes;
mod view_window;
use egui::Pos2;

pub const PROFILE: bool = false;
pub const DEFAULT_WINDOW_WIDTH: f32 = 1280.0;
pub const DEFAULT_WINDOW_HEIGHT: f32 = 800.0;
pub const APP_MENU_HEIGHT: f32 = 40.0;
pub const NODE_MENU_WIDTH: f32 = 250.0;
pub const SETTINGS_PANEL_WIDTH: f32 = 300.0;
pub const DEFAULT_THEME: Theme = Theme::DarkGreen;
pub const NODE_SIZE: Vec2 = Vec2::new(150.0, 40.0);
pub const NODE_ROUNDING: f32 = 2.0;

#[tokio::main]
async fn main() -> Result<(), eframe::Error> {
    puffin::set_scopes_on(PROFILE);

    let options = eframe::NativeOptions {
        // 4x MSAA on the main framebuffer so the 3D viewer gets geometry AA.
        multisampling: 4,
        // The 3D viewer (view_window/gl_renderer.rs) enables GL_DEPTH_TEST and
        // relies on a real depth buffer to sort mesh triangles. eframe's default
        // is 0 depth bits; without this the depth test only "worked" because the
        // Windows GL driver happened to pick a depth-capable framebuffer config.
        // Request 24-bit explicitly so behavior doesn't depend on driver defaults.
        depth_buffer: 24,
        viewport: egui::ViewportBuilder::default()
            .with_maximized(true)
            .with_resizable(true)
            .with_decorations(true)
            .with_icon(load_icon()),
        ..Default::default()
    };

    eframe::run_native(
        "NodeMangler",
        options,
        Box::new(|cc| Ok(Box::<app::App>::new(app::App::new(cc)))),
    )
}

fn load_icon() -> egui::IconData {
    // Embedded so the binary works standalone, without the source tree.
    let bytes = include_bytes!("../assets/mangler_icon.png");
    let image = image::load_from_memory(bytes)
        .expect("Failed to decode embedded icon")
        .into_rgba8();
    let (width, height) = image.dimensions();

    egui::IconData {
        rgba: image.into_raw(),
        width,
        height,
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
pub struct ManglerError(pub String);
