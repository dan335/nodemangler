use crate::{graph::graph_node::InputOutputResponse, graph_to_view_space, themes::theme::Theme};
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
    temp_connection: Option<&TempConnection>,
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
                || !temp.from_value_type
                    .valid_conversions()
                    .contains(&input.value.value_type())
            {
                response.is_disabled = true;
            }
        }
    }

    response.is_cursor_over = input_response.hovered();

    if input.is_error {
        color = theme.get().grid_connection_dot_error;
    } else if response.is_disabled {
        color = theme.get().grid_connection_dot_disabled;
    } else if input_response.hovered() {
        color = theme.get().grid_connection_dot_hover;
    }

    let shape = Shape::circle_filled(input_position, graph_to_view_space(graph_zoom, 5.0), color);
    ui.painter().add(shape);

    if input_response.drag_started() {
        response.has_started_creating_connection = true;
        response.connection_from_position = input_position;
    } else if input_response.drag_stopped() {
        response.has_stopped_creating_connection = true;
        response.connection_to_position = input_position;
    }

    let pos = Pos2::new(input_position.x - graph_to_view_space(graph_zoom, 10.0), input_position.y);
    let font_id = egui::FontId::proportional(graph_to_view_space(graph_zoom, 12.0));
    let color = theme.get().override_text_color;

    if show_names || response.is_cursor_over {
        let galley = ui.painter().layout_no_wrap(input.name.clone(), font_id.clone(), color);
        
        // bg
        ui.painter().rect_filled(Rect::from_min_size(Pos2::new(pos.x - galley.rect.width(), pos.y - (galley.rect.height() * 0.5)), galley.rect.size()), egui::CornerRadius::same(1), theme.get().grid_bg);

        // text
        ui.painter().text(
            pos,
            egui::Align2::RIGHT_CENTER,
            input.name.clone(),
            font_id.clone(),
            color,
        );
    }

    if response.is_cursor_over {
        let valid_conversions = input.value.value_type().valid_conversions_from();

        if valid_conversions.len() == 0 {
            let txt = "none".to_string();
            let txt_pos = Pos2::new(pos.x, graph_to_view_space(graph_zoom, 25.0) + pos.y);

            let galley = ui.painter().layout_no_wrap(txt.clone(), font_id.clone(), color);

            // bg
            ui.painter().rect_filled(Rect::from_min_size(Pos2::new(txt_pos.x - galley.rect.width(), txt_pos.y - (galley.rect.height() * 0.5)), galley.rect.size()), egui::CornerRadius::same(1), theme.get().grid_bg);

            // text
            ui.painter().text(
                txt_pos,
                egui::Align2::RIGHT_CENTER,
                txt,
                font_id.clone(),
                color,
            );
        } else {
            for (index, valid_type) in input.value.value_type().valid_conversions_from().iter().enumerate() {
                let txt = valid_type.value_name();
                let txt_pos = Pos2::new(pos.x, graph_to_view_space(graph_zoom, 25.0) + pos.y + graph_to_view_space(graph_zoom, 15.0) * index as f32);
    
                let galley = ui.painter().layout_no_wrap(txt.clone(), font_id.clone(), theme.get().text_faint);
    
                // bg
                ui.painter().rect_filled(Rect::from_min_size(Pos2::new(txt_pos.x - galley.rect.width(), txt_pos.y - (galley.rect.height() * 0.5)), galley.rect.size()), egui::CornerRadius::same(1), theme.get().grid_bg);
    
                // text
                ui.painter().text(
                    txt_pos,
                    egui::Align2::RIGHT_CENTER,
                    txt,
                    font_id.clone(),
                    theme.get().text_faint,
                );
            }
        }
        
    }

    response
}
