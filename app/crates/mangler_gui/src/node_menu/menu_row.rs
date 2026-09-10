//! Painting and interaction for one Node List row.
//!
//! Rows are hand-painted rather than built from `egui::Button` because the
//! button's `.frame(false)` gave no visible hover at all, and because the row
//! must keep an identical height in every state — the same reason the node
//! search popup avoids `selectable_label` (see `node_search_popup.rs`).

use eframe::egui;
use epaint::{vec2, Rect};

use crate::themes::theme::Theme;

use super::menu_item::{RowKind, RowSpec};

pub const ROW_HEIGHT: f32 = 24.0;
pub const INDENT_STEP: f32 = 14.0;
/// Left inset of a row's glyph column. Small because the panel frame
/// already insets the whole body.
pub const ROW_PAD: f32 = 4.0;
/// Width reserved at the left of a row for the caret / drag grip.
const GLYPH_WIDTH: f32 = 16.0;
/// Below this much room, the category path in search mode is dropped rather
/// than squeezing the node's own name.
const SECONDARY_MIN_WIDTH: f32 = 80.0;

/// Horizontal offset of a row's glyph column at nesting `level`.
pub fn indent_px(level: usize) -> f32 {
    ROW_PAD + level as f32 * INDENT_STEP
}

/// What the user did to a row this frame.
#[derive(Debug, PartialEq, Eq)]
pub enum RowEvent {
    None,
    ToggleCategory,
    Add,
    DragStart,
}

/// Draws one row and reports the interaction.
pub fn show_row(ui: &mut egui::Ui, spec: &RowSpec, theme: &Theme) -> RowEvent {
    let colors = theme.get();
    let is_category = matches!(spec.kind, RowKind::Category { .. });

    // Categories only ever toggle; operation and subgraph rows must sense both,
    // since click adds and drag places.
    let sense = if is_category {
        egui::Sense::click()
    } else {
        egui::Sense::click_and_drag()
    };
    let (rect, response) =
        ui.allocate_exact_size(vec2(ui.available_width(), ROW_HEIGHT), sense);

    if ui.is_rect_visible(rect) {
        paint_row(ui, spec, &response, rect, &colors, is_category);
    }

    let response = if is_category {
        response.on_hover_cursor(egui::CursorIcon::PointingHand)
    } else {
        response.on_hover_cursor(egui::CursorIcon::Grab)
    };
    let response = add_tooltip(response, spec);

    if is_category {
        if response.clicked() {
            return RowEvent::ToggleCategory;
        }
    } else if response.drag_started() {
        return RowEvent::DragStart;
    } else if response.clicked() {
        return RowEvent::Add;
    }

    RowEvent::None
}

fn paint_row(
    ui: &egui::Ui,
    spec: &RowSpec,
    response: &egui::Response,
    rect: Rect,
    colors: &crate::themes::theme::ThemeValues,
    is_category: bool,
) {
    // The default weak hover fill is invisible in several of the themes (the
    // reason `strengthen_menu_hover` exists in the Libraries panel), so paint
    // the strong one directly.
    if response.is_pointer_button_down_on() {
        ui.painter()
            .rect_filled(rect, 3.0, colors.widgets_active_bg_fill);
    } else if response.hovered() {
        ui.painter()
            .rect_filled(rect, 3.0, colors.widgets_hovered_bg_fill);
    }

    let font = egui::FontId::proportional(15.0);
    let glyph_x = rect.left() + indent_px(spec.level);

    // Categories always show their caret; operation rows only show the grip on
    // hover. Painting (not layout) is what changes, so the row never reflows.
    let glyph = match spec.kind {
        RowKind::Category { collapsed } => Some(if collapsed {
            crate::icons::CARET_RIGHT
        } else {
            crate::icons::CARET_DOWN
        }),
        _ if response.hovered() => Some(crate::icons::DOTS_SIX_VERTICAL),
        _ => None,
    };
    if let Some(glyph) = glyph {
        ui.painter().text(
            egui::pos2(glyph_x, rect.center().y),
            egui::Align2::LEFT_CENTER,
            glyph,
            font.clone(),
            colors.text_faint,
        );
    }

    let label_x = glyph_x + GLYPH_WIDTH;
    let label_color = if is_category {
        colors.text_faint
    } else {
        colors.override_text_color
    };

    // Right-aligned category path, in search mode only and only when there is
    // room for it without eating into the node's own name.
    let mut label_limit = rect.right() - ROW_PAD - label_x;
    if let Some(secondary) = spec.secondary {
        let galley = ui.painter().layout_no_wrap(
            secondary.to_string(),
            egui::FontId::proportional(12.0),
            colors.text_faint,
        );
        let width = galley.size().x;
        if label_limit - width > SECONDARY_MIN_WIDTH {
            let pos = egui::pos2(
                rect.right() - ROW_PAD - width,
                rect.center().y - galley.size().y * 0.5,
            );
            ui.painter().galley(pos, galley, colors.text_faint);
            label_limit -= width + ROW_PAD;
        }
    }

    // Truncate rather than wrap: the row is a fixed height, so a wrapped label
    // would spill into its neighbours in a narrow panel.
    let mut job = egui::text::LayoutJob::single_section(
        spec.label.to_string(),
        egui::TextFormat::simple(font, label_color),
    );
    job.wrap.max_width = label_limit.max(0.0);
    job.wrap.max_rows = 1;
    job.wrap.overflow_character = Some('…');
    let galley = ui.painter().layout_job(job);
    ui.painter().galley(
        egui::pos2(label_x, rect.center().y - galley.size().y * 0.5),
        galley,
        label_color,
    );
}

/// Description + help + a one-line gesture reminder, built only for the row
/// actually hovered. Suppressed mid-drag, where a tooltip trailing the ghost
/// node is just noise.
fn add_tooltip(response: egui::Response, spec: &RowSpec) -> egui::Response {
    if spec.description.is_empty() || response.dragged() {
        return response;
    }
    let description = spec.description;
    let help = spec.help;
    response.on_hover_ui(|ui| {
        ui.set_max_width(320.0);
        ui.label(description);
        if !help.is_empty() {
            ui.add_space(4.0);
            ui.label(egui::RichText::new(help).weak().size(12.0));
        }
        ui.add_space(4.0);
        ui.label(
            egui::RichText::new("click to add  ·  drag to place")
                .weak()
                .size(11.0),
        );
    })
}
