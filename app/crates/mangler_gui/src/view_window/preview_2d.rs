//! 2D preview content: renders a single node output into an arbitrary `Ui`.
//!
//! Free functions (no owning panel struct) so the caller keeps the per-leaf
//! [`ImageViewer`] pan/zoom state. The value-dispatch logic mirrors the
//! non-tab content path of the old `ViewPanel::show_content`.

use eframe::egui::{self, RichText};
use epaint::{Stroke, StrokeKind};

use crate::{graph::graph_node::GraphNode, themes::theme::Theme};

use super::{color_viewer::ColorViewer, curve_overlay, image_viewer::ImageViewer, text_viewer::TextViewer};

/// Render the given node output into `ui` using the supplied per-leaf image
/// viewer for pan/zoom state. Pointer input is read from `ui`'s own context,
/// so this works identically in secondary OS windows.
pub fn show(
    ui: &mut egui::Ui,
    viewer: &mut ImageViewer,
    graph_node: &GraphNode,
    output_index: usize,
    theme: &Theme,
) {
    let Some(output) = graph_node.outputs.get(output_index) else {
        show_empty(ui, theme);
        return;
    };

    // Images fill the whole panel; a header would overlap the pan/zoom canvas
    // awkwardly, so it is drawn only for the non-image value viewers.
    if let mangler_core::value::Value::Image { data, change_id } = &output.value {
        viewer.show(
            ui,
            graph_node.id.clone(),
            output_index,
            change_id.clone(),
            data,
            false, // node outputs re-run constantly; don't reset the user's pan/zoom
            theme,
        );
        return;
    }

    // Faint one-line header for the value viewers.
    ui.label(
        RichText::new(format!("{} · {}", graph_node.settings.name, output.name))
            .color(theme.get().text_faint)
            .small(),
    );
    ui.add_space(4.0);

    match &output.value {
        mangler_core::value::Value::Bool(value) => TextViewer::show(ui, value.to_string()),
        mangler_core::value::Value::Integer(value) => TextViewer::show(ui, value.to_string()),
        mangler_core::value::Value::Decimal(value) => TextViewer::show(ui, format!("{:?}", value)),
        mangler_core::value::Value::Text(value) => TextViewer::show(ui, value.to_string()),
        mangler_core::value::Value::Color(value) => ColorViewer::show(ui, *value),
        mangler_core::value::Value::Path(path) => {
            TextViewer::show(ui, path.to_str().unwrap_or("none").to_string())
        }
        mangler_core::value::Value::FilterType(value) => TextViewer::show(ui, format!("{:?}", value)),
        mangler_core::value::Value::ColorFormat(value) => {
            TextViewer::show(ui, format!("{:?}", value))
        }
        mangler_core::value::Value::ImageType(value) => TextViewer::show(ui, format!("{:?}", value)),
        mangler_core::value::Value::Trigger => TextViewer::show(ui, "trigger".to_string()),
        mangler_core::value::Value::NoiseWorleyDistanceFunction(value) => {
            TextViewer::show(ui, format!("{:?}", value))
        }
        mangler_core::value::Value::ColorSpace(value) => TextViewer::show(ui, format!("{:?}", value)),
        mangler_core::value::Value::BlendMode(value) => TextViewer::show(ui, format!("{:?}", value)),
        mangler_core::value::Value::EdgeMode(value) => TextViewer::show(ui, format!("{:?}", value)),
        mangler_core::value::Value::TextHAlign(value) => TextViewer::show(ui, format!("{:?}", value)),
        mangler_core::value::Value::TextVAlign(value) => TextViewer::show(ui, format!("{:?}", value)),
        mangler_core::value::Value::ExportPreset(value) => TextViewer::show(ui, format!("{:?}", value)),
        // Painted read-only canvas: a letterboxed square (matching the overlay
        // editor's fallback rect, so a viewed-and-edited curve lines up) with a
        // themed background/border and the curve drawn on top.
        mangler_core::value::Value::Curve(value) => {
            let colors = theme.get();
            let canvas = curve_overlay::fallback_canvas_rect(ui.max_rect());
            ui.painter().rect_filled(canvas, 4.0, colors.grid_bg);
            ui.painter().rect_stroke(
                canvas,
                4.0,
                Stroke::new(1.0, colors.grid_lines),
                StrokeKind::Inside,
            );
            curve_overlay::draw_curve(
                ui.painter(),
                canvas,
                value,
                Stroke::new(2.0, colors.grid_connection_line),
                theme,
            );
        }
        mangler_core::value::Value::Image { .. } => unreachable!(),
    }
}

/// Placeholder shown when no node output is being viewed in this panel.
pub fn show_empty(ui: &mut egui::Ui, theme: &Theme) {
    let rect = ui.max_rect();
    ui.painter().text(
        rect.center(),
        egui::Align2::CENTER_CENTER,
        "right-click a node output to view",
        egui::FontId::proportional(13.0),
        theme.get().text_faint,
    );
}

/// Placeholder shown when a curve is being edited but nothing (or a non-image)
/// is viewed to trace over. Frames the fallback canvas the overlay will draw
/// on (same rect), so the [0,1]² drawing area is visible.
pub fn show_curve_hint(ui: &mut egui::Ui, theme: &Theme) {
    let rect = ui.max_rect();
    let colors = theme.get();
    let canvas = curve_overlay::fallback_canvas_rect(rect);
    ui.painter().rect_filled(canvas, 4.0, colors.grid_bg);
    ui.painter().rect_stroke(
        canvas,
        4.0,
        Stroke::new(1.0, colors.grid_lines),
        StrokeKind::Inside,
    );
    ui.painter().text(
        rect.center_bottom() - egui::vec2(0.0, 14.0),
        egui::Align2::CENTER_BOTTOM,
        "drawing curve — right-click a node output to trace over it",
        egui::FontId::proportional(13.0),
        colors.text_faint,
    );
}
