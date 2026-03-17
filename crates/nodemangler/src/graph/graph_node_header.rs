use eframe::epaint::{Color32, CornerRadius};
use eframe::{egui, emath::Align2};
use egui::{Pos2, Rect};

use crate::{graph_to_view_space, NODE_ROUNDING};
use crate::themes::theme::Theme;



pub fn show_graph_node_header(
    ui: &mut egui::Ui,
    name: &str,
    node_rect: Rect,
    is_editing: bool,
    is_subgraph: bool,
    graph_zoom: f32,
    theme: &Theme,
    is_busy: bool,
) {
    // bg
    if is_busy {
        ui.painter().add(egui::Shape::rect_filled(
            node_rect,
            CornerRadius::same(NODE_ROUNDING as u8),
            theme.get().grid_connection_line,
        ));
    } else {
        ui.painter().add(egui::Shape::rect_filled(
            node_rect,
            CornerRadius::same(NODE_ROUNDING as u8),
            theme.get().node_header_bg,
        ));
    }
    

    // outline
    if is_editing {
        ui.painter().add(egui::Shape::rect_stroke(
            node_rect,
            CornerRadius::same(NODE_ROUNDING as u8),
            egui::Stroke::new(graph_to_view_space(graph_zoom, 3.0), theme.get().node_header_selected_border),
            egui::StrokeKind::Outside,
        ));
    }

    // node name
    ui.painter().text(
        node_rect.center(),
        Align2::CENTER_CENTER,
        name,
        //egui::style::Style::text_styles(),
        egui::FontId::proportional(graph_to_view_space(graph_zoom, 14.0)),
        Color32::from(theme.get().override_text_color),
    );

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
