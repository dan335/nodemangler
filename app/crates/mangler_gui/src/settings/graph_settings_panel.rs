use std::path::PathBuf;
use eframe::egui::{self, Button, Label, RichText, TextEdit};

use crate::{
    file_dialog::FileDialogRequest,
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

    // --- graph section: name + location together ---
    // A graph's name IS its file name, so the two aren't independent
    // settings: for a saved graph the name field renames the file in place,
    // and the location line shows where that file lives. For an unsaved
    // graph the name is a pending, GUI-side value that becomes the file
    // stem at first save.
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
        // For a saved graph a name change renames the file on disk — an
        // expensive, potentially-failing op we don't want to fire on every
        // keystroke. Commit only when the field loses focus (click-away or
        // Enter). The unsaved case follows the same rule for consistency.
        if response.lost_focus() {
            graph_settings_response.new_name = Some(name_buffer.clone());
        } else if !response.has_focus() {
            // While not being edited, keep the buffer in sync with the
            // authoritative name (file stem, or the pending name while
            // unsaved), so an external rename (or a failed rename that kept
            // the old name) is reflected.
            if name_buffer != display_name {
                *name_buffer = display_name.to_string();
            }
        }
    });

    match program_path {
        Some(path) => {
            // The path used to be a disabled (greyed-out) TextEdit, which
            // looked editable but wasn't and couldn't wrap long paths. A
            // plain wrapping label reads as what it is: informational text.
            let path_text = path.to_str().unwrap_or_default();
            ui.add(Label::new(RichText::new(path_text).monospace().color(theme.get().text_faint)).wrap());
            ui.add_space(8.0);

            if ui
                .add(Button::new(egui::RichText::new("save a copy as")))
                .on_hover_text("saves a copy to the new location; the old file is not deleted")
                .clicked()
            {
                graph_settings_response.file_dialog_request =
                    Some(FileDialogRequest::SaveGraph {
                        default_dir: path.parent().map(PathBuf::from),
                        default_stem: display_name.to_owned(),
                    });
            }
        }
        None => {
            ui.add(Label::new(
                RichText::new("not saved").italics().color(theme.get().text_faint),
            ));
            ui.add_space(8.0);

            if ui
                .add(Button::new(egui::RichText::new("save graph")))
                .on_hover_text("choose where to save; auto-save keeps it up to date afterwards")
                .clicked()
            {
                // Seed the dialog with the default library so a first save
                // lands where the Libraries panel can see it. Loaded lazily
                // inside the click (not per-frame); ensure_default_library
                // is idempotent and recreates the folder if it was deleted.
                let mut config = crate::config::AppConfig::load();
                let default_dir = config.ensure_default_library();
                config.save();

                graph_settings_response.file_dialog_request =
                    Some(FileDialogRequest::SaveGraph {
                        default_dir,
                        default_stem: display_name.to_owned(),
                    });
            }
        }
    }

    // --- layout section ---
    section_rule(ui, theme);
    section_label(ui, "layout");

    if ui.add(Button::new(egui::RichText::new("auto arrange"))).clicked() {
        graph_settings_response.auto_arrange = true;
    }

    graph_settings_response
}

pub struct GraphSettingsResponse {
    /// A file dialog the user asked for. `Program` bubbles this to `App`,
    /// which owns the dialog; the pick returns via `FileDialogIntent`.
    pub file_dialog_request: Option<FileDialogRequest>,
    pub new_name: Option<String>,
    pub auto_arrange: bool,
}

impl GraphSettingsResponse {
    pub fn new() -> Self {
        Self {
            file_dialog_request: None,
            new_name: None,
            auto_arrange: false,
        }
    }
}
