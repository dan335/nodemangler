use eframe::epaint::{Color32, Rounding};
use eframe::{egui, emath::Align2};
use egui::{Pos2, Rect};

use crate::theme::Theme;
use crate::{graph_to_view_space_pos2, graph_to_view_space};

const ROUNDING: f32 = 2.0;

pub fn show_graph_node_header(
    ui: &mut egui::Ui,
    name: String,
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
            Rounding::same(ROUNDING),
            theme.get().grid_connection_line,
        ));
    } else {
        ui.painter().add(egui::Shape::rect_filled(
            node_rect,
            Rounding::same(ROUNDING),
            theme.get().node_header_bg,
        ));
    }
    

    // outline
    if is_editing {
        ui.painter().add(egui::Shape::rect_stroke(
            node_rect,
            ROUNDING,
            egui::Stroke::new(1.0, theme.get().node_header_selected_border),
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
