use std::path::PathBuf;
extern crate sanitize_filename;
use eframe::egui::{self, Button, Label, RichText, TextEdit};

use crate::{
    settings::section::{section_label, section_rule},
    themes::theme::Theme,
};

pub fn show(
    ui: &mut egui::Ui,
    program_name: &mut String,
    program_path: &Option<PathBuf>,
    theme: &Theme,
) -> GraphSettingsResponse {
    let mut graph_settings_response = GraphSettingsResponse::new();

    // Title: small semibold label, matching the node settings panel's
    // header. No close control here — the graph settings panel is the
    // default state of the settings panel (shown when no node is selected),
    // so there's nothing to dismiss back to.
    // `.strong()` resolves to `widgets.active.fg_stroke` (rose in dark_green)
    // rather than a weight change — use the dedicated semibold font family
    // instead, with no explicit color override.
    ui.label(RichText::new("graph settings").size(15.0).family(crate::themes::theme::semibold_family()));

    // --- graph section ---
    section_rule(ui, theme);
    section_label(ui, "graph");

    ui.horizontal(|ui| {
        // Fixed-width label column, echoing the node settings panel's
        // name/value table layout even though this is a single hand-built row.
        // `ui.add_sized` centers its content within the given size, which
        // misaligned this label against the "graph" section label above it
        // (left-aligned); `allocate_ui_with_layout` with a left-to-right
        // layout gives a fixed-width cell without re-centering the text.
        ui.allocate_ui_with_layout(
            egui::vec2(52.0, ui.spacing().interact_size.y),
            egui::Layout::left_to_right(egui::Align::Center),
            |ui| {
                ui.label(RichText::new("name").color(theme.get().text_faint));
            },
        );
        let remaining_width = ui.available_width();
        if ui.add(TextEdit::singleline(program_name).desired_width(remaining_width)).changed() {
            graph_settings_response.new_name = Some(program_name.clone());
        }
    });

    // --- save location section ---
    section_rule(ui, theme);
    section_label(ui, "save location");

    let mut path = "".to_string();
    if let Some(p) = program_path {
        if let Some(pa) = p.to_str() {
            path = pa.to_string();
        }
    }

    // The path used to be a disabled (greyed-out) TextEdit, which looked
    // editable but wasn't and couldn't wrap long paths. A plain wrapping
    // label reads as what it is: informational text, not a field.
    ui.add(Label::new(RichText::new(&path).monospace().color(theme.get().text_faint)).wrap());
    ui.add_space(8.0);

    //ui.vertical_centered(|ui| {
        if ui.add(Button::new(egui::RichText::new("select location"))).clicked() {
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

    // --- layout section ---
    section_rule(ui, theme);
    section_label(ui, "layout");

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
