use eframe::epaint::{Color32, Rounding};
use eframe::{egui, emath::Align2};
use egui::{Pos2, Rect};

use crate::{graph_to_view_space_pos2, graph_to_view_space};

const ROUNDING: f32 = 2.0;
const COLOR_BG: Color32 = egui::Color32::from_gray(70);
const COLOR_TEXT: Color32 = egui::Color32::from_gray(220);
const COLOR_BORDER_EDITING: Color32 = egui::Color32::from_gray(200);

pub fn show_graph_node_header(
    ui: &mut egui::Ui,
    name: String,
    node_rect: Rect,
    is_editing: bool,
    is_subgraph: bool,
    graph_zoom: f32,
) {
    // bg
    ui.painter().add(egui::Shape::rect_filled(
        node_rect,
        Rounding::same(ROUNDING),
        COLOR_BG,
    ));

    // outline
    if is_editing {
        ui.painter().add(egui::Shape::rect_stroke(
            node_rect,
            ROUNDING,
            egui::Stroke::new(2.0, COLOR_BORDER_EDITING),
        ));
    }

    // node name
    ui.painter().text(
        node_rect.center(),
        Align2::CENTER_CENTER,
        name,
        //egui::style::Style::text_styles(),
        egui::FontId::proportional(graph_to_view_space(graph_zoom, 12.0)),
        egui::Color32::from_gray(220),
    );

    // subgraph
    if is_subgraph {
        ui.painter().text(
            Pos2::new(node_rect.center().x, node_rect.top() - 20.0),
            Align2::CENTER_TOP,
            "subgraph",
            egui::FontId::default(),
            COLOR_TEXT,
        );
    }
}
