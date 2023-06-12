use crate::{graph::graph_node::InputOutputResponse, theme::Theme};
use eframe::{
    egui,
    epaint::{Color32, Pos2, Rect, Shape},
};
use epaint::Rounding;

use super::{graph_editor::TempConnection, graph_node::ConnectionType};
use mangler::output::Output;

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
    theme: &Theme,
    graph_zoom: f32,
) -> InputOutputResponse {
    puffin::profile_scope!("graph node.draw_graph_output()");
    let mut response = InputOutputResponse::new();
    let mut color = theme.get().grid_connection_dot;
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
        color = theme.get().grid_connection_dot_disabled;
    } else if output_response.hovered() {
        color = theme.get().grid_connection_dot_hover;
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

    // show name and type when hovering
    if show_type || response.is_cursor_over {
        puffin::profile_scope!("graph node.show type when hovering");

        let txt = format!("{} ({})", output.name, output_value_name);
        let font_id = egui::FontId::proportional(crate::graph_to_view_space(graph_zoom, 12.0));
        let color = theme.get().override_text_color;
        let pos = Pos2::new(output_position.x + 10.0, output_position.y);

        let galley = ui.painter().layout_no_wrap(txt.clone(), font_id.clone(), color);
        
        // bg
        ui.painter().rect_filled(Rect::from_min_size(Pos2::new(pos.x, pos.y - (galley.rect.height() * 0.5)), galley.rect.size()), Rounding::same(1.0), theme.get().grid_bg);

        // text
        ui.painter().text(
            pos,
            egui::Align2::LEFT_CENTER,
            txt,
            font_id,
            color,
        );
    }

    response
}

pub fn draw_graph_output_highlighted(output_position: Pos2, ui: &mut egui::Ui, theme: &Theme) {
    ui.painter().add(Shape::circle_stroke(
        output_position,
        6.0,
        egui::Stroke::new(1.0, theme.get().node_header_selected_border),
    ));
}
