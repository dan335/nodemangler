use std::path::PathBuf;
extern crate sanitize_filename;
use eframe::egui;
use epaint::vec2;

pub fn show(
    ui: &mut egui::Ui,
    program_name: &mut String,
    program_path: &Option<PathBuf>,
) -> GraphSettingsResponse {
    let mut graph_settings_response = GraphSettingsResponse::new();

    ui.label("Graph Name");
    ui.text_edit_singleline(program_name);

    let mut path = "".to_string();
    if let Some(p) = program_path {
        if let Some(pa) = p.to_str() {
            path = pa.to_string();
        }
    }

    ui.add_space(20.0);

    ui.label("Save Location");
    ui.horizontal(|ui| {
        ui.allocate_ui(vec2(ui.available_width() - 20.0, ui.available_height()), |ui| {
            ui.add_enabled_ui(false, |ui| {
                ui.text_edit_singleline(&mut path)
            });
        
            
        });

        if ui.button("🗀").clicked() {
            let options = sanitize_filename::Options {
                truncate: true, // true by default, truncates to 255 bytes
                windows: true, // default value depends on the OS, removes reserved names like `con` from start of strings on Windows
                replacement: "" // str to replace sanitized chars/strings
            };

            let mut starting_file_name = sanitize_filename::sanitize_with_options(program_name, options);

            // remove whitespace
            starting_file_name = starting_file_name.replace(" ", "_");

            if let Some(save_path) = rfd::FileDialog::new().set_file_name(&starting_file_name).add_filter("mangler", &["mangle"]).save_file() {
                graph_settings_response.new_save_path = Some(save_path);
            }
        }
    });
    
    graph_settings_response
}


pub struct GraphSettingsResponse {
    pub new_save_path: Option<PathBuf>,
}

impl GraphSettingsResponse {
    pub fn new() -> Self {
        Self {
            new_save_path: None,
        }
    }
}