use eframe::{epaint::{Pos2, Rect, Shape, Color32}, egui};
use mangler::output::Output;
use crate::graph::graph_node::InputOutputResponse;

const OUTPUT_BG_COLOR: Color32 = Color32::from_gray(150);
const OUTPUT_BG_COLOR_HOVER: Color32 = Color32::from_gray(200);
const OUTPUT_TEXT: Color32 = Color32::from_gray(200);

pub fn draw_graph_output(output_name: &String, output_value_name: &String, output_position: Pos2, input_rect: Rect, _index: usize, _rect: Rect, ui: &mut egui::Ui, show_type: bool) -> InputOutputResponse {
    puffin::profile_scope!("graph node.draw_graph_output()");
    let mut response = InputOutputResponse::new();
    let mut color = OUTPUT_BG_COLOR;
    let output_response = ui.allocate_rect(input_rect, egui::Sense::drag().union(egui::Sense::hover()));

    // highlight when hovering
    if output_response.hovered() {
        color = OUTPUT_BG_COLOR_HOVER;
    }

    // draw bg
    let shape = Shape::circle_filled(output_position, 5.0, color);
    response.is_cursor_over = output_response.hovered();
    ui.painter().add(shape);

    // creating connections
    if output_response.drag_started() {
        response.has_started_creating_connection = true;
        response.connection_from_position = output_position;
    } else if output_response.drag_released() {
        response.has_stopped_creating_connection = true;
        response.connection_to_position = output_position;
    }

    // show type when hovering
    if show_type || response.is_cursor_over {
        puffin::profile_scope!("graph node.show type when hovering");
        let txt = format!("{} - {}", output_name, output_value_name);
        ui.painter().text(Pos2::new(output_position.x + 10.0, output_position.y), egui::Align2::LEFT_CENTER, txt, egui::FontId::proportional(12.0), OUTPUT_TEXT);
    }

    response
}

pub fn draw_graph_output_highlighted(output_position: Pos2, ui: &mut egui::Ui) {
    puffin::profile_scope!("graph node.draw_graph_output_highlighted()");
    let color = Color32::from_rgb(222, 214, 90);
    ui.painter().add(Shape::circle_stroke(output_position, 6.0, egui::Stroke::new(4.0, color)));
}