//! Shared layout helpers for the settings panels (node settings + graph
//! settings): a full-width hairline rule and a small lowercase section label.
//! Pulled out of `node_settings_panel.rs` so `graph_settings_panel.rs` can
//! use the exact same section chrome without duplicating it.

use eframe::egui;

use crate::themes::theme::Theme;

/// Draw a full-width 1px hairline rule in `theme.settings_section_rule`,
/// with the vertical rhythm from the design doc: ~16px of space above the
/// rule (pushing it away from the previous section's content) and ~14px
/// below it before the next section label.
///
/// Implemented as a manually painted rect rather than `ui.separator()`
/// because `ui.separator()` draws in the widget "noninteractive" stroke
/// color, which isn't themed for this purpose — we need the dedicated
/// `settings_section_rule` color instead.
pub(crate) fn section_rule(ui: &mut egui::Ui, theme: &Theme) {
    ui.add_space(16.0);

    let width = ui.available_width();
    let (rect, _response) = ui.allocate_exact_size(egui::vec2(width, 1.0), egui::Sense::hover());
    if ui.is_rect_visible(rect) {
        ui.painter().rect_filled(rect, 0.0, theme.get().settings_section_rule);
    }

    ui.add_space(14.0);
}

/// Draw a lowercase section label in the normal (non-faint) text color. Sits
/// ~8px above the section's content, per the design. Size bumped from 12px
/// to 14px per user request — normal weight and color are unchanged.
pub(crate) fn section_label(ui: &mut egui::Ui, text: &str) {
    ui.label(egui::RichText::new(text).size(14.0));
    ui.add_space(8.0);
}
