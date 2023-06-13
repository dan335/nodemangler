use crate::{graph::graph_node::InputOutputResponse, theme::Theme, graph_to_view_space};
use eframe::{
    egui,
    epaint::{Pos2, Rect, Shape},
};
use mangler::input::Input;

use super::{graph_editor::TempConnection, graph_node::ConnectionType};


pub fn draw_graph_input(
    node_id: &String,
    input: &Input,
    input_position: Pos2,
    input_rect: Rect,
    index: usize,
    _node_rect: Rect,
    ui: &mut egui::Ui,
    show_names: bool,
    temp_connection: Option<TempConnection>,
    theme: &Theme,
    graph_zoom: f32,
) -> InputOutputResponse {
    let mut response = InputOutputResponse::new();
    let mut color = theme.get().grid_connection_dot;
    let input_response =
        ui.allocate_rect(input_rect, egui::Sense::drag().union(egui::Sense::hover()));

    if let Some(temp) = temp_connection {
        // if we're dragging from this node
        if node_id == &temp.from_node_id {
            if temp.from_connection_type == ConnectionType::Output
                || temp.from_connection_index != index
            {
                response.is_disabled = true;
            }
        } else {
            if temp.from_connection_type == ConnectionType::Input
                || !input
                    .value
                    .valid_conversions()
                    .contains(&temp.from_value_type)
            {
                response.is_disabled = true;
            }
        }
    }

    response.is_cursor_over = input_response.hovered();

    if response.is_disabled {
        color = theme.get().grid_connection_dot_disabled;
    } else if input_response.hovered() {
        color = theme.get().grid_connection_dot_hover;
    }

    let shape = Shape::circle_filled(input_position, 5.0, color);
    ui.painter().add(shape);

    if input_response.drag_started() {
        response.has_started_creating_connection = true;
        response.connection_from_position = input_position;
    } else if input_response.drag_released() {
        response.has_stopped_creating_connection = true;
        response.connection_to_position = input_position;
    }

    if show_names || response.is_cursor_over {
        let font_id = egui::FontId::proportional(graph_to_view_space(graph_zoom, 12.0));
        let color = theme.get().override_text_color;
        let pos = Pos2::new(input_position.x - 10.0, input_position.y);

        let galley = ui.painter().layout_no_wrap(input.name.clone(), font_id.clone(), color);
        
        // bg
        ui.painter().rect_filled(Rect::from_min_size(Pos2::new(pos.x - galley.rect.width(), pos.y - (galley.rect.height() * 0.5)), galley.rect.size()), egui::Rounding::same(1.0), theme.get().grid_bg);

        // text
        ui.painter().text(
            pos,
            egui::Align2::RIGHT_CENTER,
            input.name.clone(),
            font_id,
            color,
        );

        // ui.painter().text(
        //     Pos2::new(input_position.x - 10.0, input_position.y),
        //     Align2::RIGHT_CENTER,
        //     input.name.clone(),
        //     FontId::proportional(12.0),
        //     Color32::from(theme.get().override_text_color),
        // );
    }

    response
}
