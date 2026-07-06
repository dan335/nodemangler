//! 2D preview content: renders a single node output into an arbitrary `Ui`.
//!
//! Free functions (no owning panel struct) so the caller keeps the per-leaf
//! [`ImageViewer`] pan/zoom state. The value-dispatch logic mirrors the
//! non-tab content path of the old `ViewPanel::show_content`.

use eframe::egui::{self, Pos2, RichText};

use crate::{graph::graph_node::GraphNode, themes::theme::Theme};

use super::{color_viewer::ColorViewer, image_viewer::ImageViewer, text_viewer::TextViewer};

/// Render the given node output into `ui` using the supplied per-leaf image
/// viewer for pan/zoom state.
pub fn show(
    ui: &mut egui::Ui,
    viewer: &mut ImageViewer,
    graph_node: &GraphNode,
    output_index: usize,
    cursor_position: Pos2,
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
            cursor_position,
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
