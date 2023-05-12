use eframe::{epaint::{Pos2, Rect, Shape, Color32}, egui};
use mangler::input::Input;
use crate::graph::graph_node::InputOutputResponse;

pub fn draw_graph_input(input: &Input, input_position: Pos2, input_rect: Rect, index: usize, node_rect: Rect, ui: &mut egui::Ui) -> InputOutputResponse {
    let mut response = InputOutputResponse::new();
    let mut color = Color32::from_gray(150);
    let input_response = ui.allocate_rect(input_rect, egui::Sense::drag().union(egui::Sense::hover()));

    if input_response.hovered() {
        color = Color32::from_gray(200);
    }

    ui.painter().add(Shape::circle_filled(input_position, 5.0, color));

    if input_response.drag_started() {
        response.has_started_creating_connection = true;
        response.connection_from_position = input_position;
    } else if input_response.drag_released() {
        response.has_stopped_creating_connection = true;
        response.connection_to_position = input_position;
    }

    response
}