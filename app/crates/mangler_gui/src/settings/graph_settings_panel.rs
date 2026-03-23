use std::path::PathBuf;
extern crate sanitize_filename;
use eframe::egui::{self, Button};

pub fn show(
    ui: &mut egui::Ui,
    program_name: &mut String,
    program_path: &Option<PathBuf>,
) -> GraphSettingsResponse {
    let mut graph_settings_response = GraphSettingsResponse::new();

    ui.heading("graph settings");

    ui.add_space(20.0);

    ui.label("graph name");
    ui.add_space(8.0);
    if ui.text_edit_singleline(program_name).changed() {
        graph_settings_response.new_name = Some(program_name.clone());
    }

    let mut path = "".to_string();
    if let Some(p) = program_path {
        if let Some(pa) = p.to_str() {
            path = pa.to_string();
        }
    }

    ui.add_space(20.0);
    ui.label("save location");
    ui.add_space(8.0);

    ui.add_enabled_ui(false, |ui| ui.text_edit_singleline(&mut path));
    ui.add_space(8.0);

    //ui.vertical_centered(|ui| {
        if ui.add(Button::new(egui::RichText::new(format!("select location")))).clicked() {
            let options = sanitize_filename::Options {
                truncate: true,  // true by default, truncates to 255 bytes
                windows: true, // default value depends on the OS, removes reserved names like `con` from start of strings on Windows
                replacement: "", // str to replace sanitized chars/strings
            };

            let sanitized =
                sanitize_filename::sanitize_with_options(program_name, options);

            // remove whitespace and append .mangle.json extension
            let starting_file_name = format!("{}.mangle.json", sanitized.replace(' ', "_"));

            if let Some(save_path) = rfd::FileDialog::new()
                .set_file_name(&starting_file_name)
                .add_filter("mangler", &["mangle.json", "json"])
                .save_file()
            {
                graph_settings_response.new_save_path = Some(save_path);
            }
        }
    //});

    ui.add_space(20.0);
    ui.separator();
    ui.add_space(12.0);

    if ui.add(Button::new(egui::RichText::new("auto arrange"))).clicked() {
        graph_settings_response.auto_arrange = true;
    }

    graph_settings_response
}

pub struct GraphSettingsResponse {
    pub new_save_path: Option<PathBuf>,
    pub new_name: Option<String>,
    pub auto_arrange: bool,
}

impl GraphSettingsResponse {
    pub fn new() -> Self {
        Self {
            new_save_path: None,
            new_name: None,
            auto_arrange: false,
        }
    }
}
