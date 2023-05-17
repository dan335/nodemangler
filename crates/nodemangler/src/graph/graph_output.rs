use eframe::{epaint::{Pos2, Rect, Shape, Color32}, egui};
use mangler::output::Output;
use crate::graph::graph_node::InputOutputResponse;

pub fn draw_graph_output(_output: &Output, output_position: Pos2, input_rect: Rect, _index: usize, _rect: Rect, ui: &mut egui::Ui) -> InputOutputResponse {
    let mut response = InputOutputResponse::new();
    let mut color = Color32::from_gray(150);
    let output_response = ui.allocate_rect(input_rect, egui::Sense::drag().union(egui::Sense::hover()));

    if output_response.hovered() {
        color = Color32::from_gray(200);
    }

    ui.painter().add(Shape::circle_filled(output_position, 5.0, color));

    if output_response.drag_started() {
        response.has_started_creating_connection = true;
        response.connection_from_position = output_position;
    } else if output_response.drag_released() {
        response.has_stopped_creating_connection = true;
        response.connection_to_position = output_position;
    }

    response
}

pub fn draw_graph_output_highlighted(output_position: Pos2, input_rect: Rect, _index: usize, _rect: Rect, ui: &mut egui::Ui) {
    let mut color = Color32::from_rgb(222, 214, 90);
    ui.painter().add(Shape::circle_stroke(output_position, 6.0, egui::Stroke::new(4.0, color)));
}