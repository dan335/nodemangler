use crate::graph::graph_node::InputOutputResponse;
use eframe::{
    egui,
    epaint::{Color32, Pos2, Rect, Shape},
};

use super::{graph_editor::TempConnection, graph_node::ConnectionType};
use mangler::output::Output;

const COLOR: Color32 = Color32::from_gray(150);
const COLOR_HOVER: Color32 = Color32::from_gray(200);
const COLOR_DISABLED: Color32 = Color32::from_gray(50);
const COLOR_TEXT: Color32 = Color32::from_gray(200);

pub fn draw_graph_output(
    node_id: &String,
    output: &Output,
    output_value_name: &String,
    output_position: Pos2,
    input_rect: Rect,
    index: usize,
    _rect: Rect,
    ui: &mut egui::Ui,
    show_type: bool,
    temp_connection: Option<TempConnection>,
) -> InputOutputResponse {
    puffin::profile_scope!("graph node.draw_graph_output()");
    let mut response = InputOutputResponse::new();
    let mut color = COLOR;
    let output_response =
        ui.allocate_rect(input_rect, egui::Sense::drag().union(egui::Sense::hover()));

    if let Some(temp) = temp_connection {
        // if we're dragging from this node
        if node_id == &temp.from_node_id {
            if temp.from_connection_type == ConnectionType::Input
                || temp.from_connection_index != index
            {
                response.is_disabled = true;
            }
        } else {
            if temp.from_connection_type == ConnectionType::Output
                || !output
                    .value
                    .valid_conversions()
                    .contains(&temp.from_value_type)
            {
                response.is_disabled = true;
            }
        }
    }

    // highlight when hovering
    if response.is_disabled {
        color = COLOR_DISABLED;
    } else if output_response.hovered() {
        color = COLOR_HOVER;
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
        let txt = format!("{} - {}", output.name, output_value_name);
        ui.painter().text(
            Pos2::new(output_position.x + 10.0, output_position.y),
            egui::Align2::LEFT_CENTER,
            txt,
            egui::FontId::proportional(12.0),
            COLOR_TEXT,
        );
    }

    response
}

pub fn draw_graph_output_highlighted(output_position: Pos2, ui: &mut egui::Ui) {
    puffin::profile_scope!("graph node.draw_graph_output_highlighted()");
    let color = Color32::from_rgb(222, 214, 90);
    ui.painter().add(Shape::circle_stroke(
        output_position,
        6.0,
        egui::Stroke::new(4.0, color),
    ));
}
