use std::path::PathBuf;
use eframe::egui::{self, Button, Label, RichText, TextEdit};
use mangler_core::naming;

use crate::{
    settings::section::{section_label, section_rule},
    themes::theme::Theme,
};

pub fn show(
    ui: &mut egui::Ui,
    name_buffer: &mut String,
    display_name: &str,
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
        let response = ui.add(
            TextEdit::singleline(name_buffer).desired_width(remaining_width),
        );
        // The graph's name IS its file name, so a name change renames the
        // file on disk — an expensive, potentially-failing op we don't want
        // to fire on every keystroke. Commit only when the field loses focus
        // (click-away or Enter).
        if response.lost_focus() {
            graph_settings_response.new_name = Some(name_buffer.clone());
        } else if !response.has_focus() {
            // While not being edited, keep the buffer in sync with the
            // authoritative name derived from the file path, so an external
            // rename (or a failed rename that kept the old name) is reflected.
            if name_buffer != display_name {
                *name_buffer = display_name.to_string();
            }
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
        if ui
            .add(Button::new(egui::RichText::new("select location")))
            .on_hover_text("saves a copy to the new location; the old file is not deleted")
            .clicked()
        {
            let starting_file_name = naming::graph_file_name(display_name);

            // rfd matches extensions against the final dot-component only,
            // so "json" alone covers both "x.json" and "x.mangler.json" — a
            // "mangle.json" filter token would never match anything.
            if let Some(save_path) = rfd::FileDialog::new()
                .set_file_name(&starting_file_name)
                .add_filter("NodeMangler graph", &["json"])
                .save_file()
            {
                // Plain-.json saves must become impossible: force the
                // canonical extension onto whatever the OS dialog returned,
                // regardless of what the user typed as the file name.
                let file_name = save_path
                    .file_name()
                    .and_then(|f| f.to_str())
                    .unwrap_or_default();
                let save_path = if file_name.ends_with(naming::GRAPH_EXTENSION) {
                    save_path
                } else {
                    // Strip a single trailing plain ".json" (if any) before
                    // appending the canonical extension, so a plain-.json
                    // choice becomes "<name>.mangler.json" rather than
                    // "<name>.json.mangler.json".
                    let stem = file_name.strip_suffix(".json").unwrap_or(file_name);
                    save_path.with_file_name(format!("{stem}{}", naming::GRAPH_EXTENSION))
                };

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
