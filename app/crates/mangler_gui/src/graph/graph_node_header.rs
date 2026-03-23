use eframe::epaint::{Color32, CornerRadius};
use eframe::{egui, emath::Align2};
use egui::{Pos2, Rect};

use crate::themes::theme::Theme;
use crate::{graph_to_view_space, NODE_ROUNDING};

pub fn show_graph_node_header(
    ui: &mut egui::Ui,
    name: &str,
    custom_name: Option<&str>,
    node_rect: Rect,
    is_editing: bool,
    is_subgraph: bool,
    graph_zoom: f32,
    theme: &Theme,
    is_busy: bool,
    is_enabled: bool,
) {
    /// Dim a color by reducing its alpha to indicate a disabled state.
    fn dim(color: Color32) -> Color32 {
        Color32::from_rgba_unmultiplied(color.r(), color.g(), color.b(), color.a() / 3)
    }

    // bg
    let bg_color = if is_busy {
        theme.get().grid_connection_line
    } else {
        theme.get().node_header_bg
    };

    ui.painter().add(egui::Shape::rect_filled(
        node_rect,
        CornerRadius::same(NODE_ROUNDING as u8),
        if is_enabled { bg_color } else { dim(bg_color) },
    ));

    // outline
    if is_editing {
        ui.painter().add(egui::Shape::rect_stroke(
            node_rect,
            CornerRadius::same(NODE_ROUNDING as u8),
            egui::Stroke::new(
                graph_to_view_space(graph_zoom, 3.0),
                theme.get().node_header_selected_border,
            ),
            egui::StrokeKind::Outside,
        ));
    }

    // node name
    let text_color = Color32::from(theme.get().override_text_color);
    if let Some(custom) = custom_name {
        // Two-row mode: custom name on top, operation type below.
        let top_y = node_rect.top() + node_rect.height() * 0.4;
        let bottom_y = node_rect.top() + node_rect.height() * 0.7;

        // Custom name (top row)
        ui.painter().text(
            Pos2::new(node_rect.center().x, top_y),
            Align2::CENTER_CENTER,
            custom,
            egui::FontId::proportional(graph_to_view_space(graph_zoom, 14.0)),
            if is_enabled {
                text_color
            } else {
                dim(text_color)
            },
        );

        // Operation type (bottom row)
        ui.painter().text(
            Pos2::new(node_rect.center().x, bottom_y),
            Align2::CENTER_CENTER,
            name,
            egui::FontId::proportional(graph_to_view_space(graph_zoom, 11.0)),
            if is_enabled {
                dim(text_color)
            } else {
                dim(dim(text_color))
            },
        );
    } else {
        // Single-row mode: operation name centered.
        ui.painter().text(
            node_rect.center(),
            Align2::CENTER_CENTER,
            name,
            egui::FontId::proportional(graph_to_view_space(graph_zoom, 14.0)),
            if is_enabled {
                text_color
            } else {
                dim(text_color)
            },
        );
    }

    // "disabled" label above the node
    if !is_enabled {
        ui.painter().text(
            Pos2::new(
                node_rect.center().x,
                node_rect.top() - graph_to_view_space(graph_zoom, 16.0),
            ),
            Align2::CENTER_BOTTOM,
            "disabled",
            egui::FontId::proportional(graph_to_view_space(graph_zoom, 11.0)),
            dim(text_color),
        );
    }

    // subgraph
    if is_subgraph {
        ui.painter().text(
            Pos2::new(node_rect.center().x, node_rect.top() - 20.0),
            Align2::CENTER_TOP,
            "subgraph",
            egui::FontId::default(),
            Color32::from(theme.get().override_text_color),
        );
    }
}
