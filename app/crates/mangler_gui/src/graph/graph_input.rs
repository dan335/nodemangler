use crate::{graph::graph_node::InputOutputResponse, graph_to_view_space, themes::theme::Theme};
use eframe::{
    egui,
    epaint::{Pos2, Rect, Shape},
};
use mangler_core::input::Input;

use super::{graph_editor::TempConnection, graph_node::ConnectionType};


pub fn draw_graph_input(
    node_id: &String,
    input: &Input,
    input_position: Pos2,
    input_rect: Rect,
    release_rect: Rect,
    index: usize,
    _node_rect: Rect,
    ui: &mut egui::Ui,
    show_names: bool,
    temp_connection: Option<&TempConnection>,
    theme: &Theme,
    graph_zoom: f32,
    cursor_position: Pos2,
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
                || (!input.accepts_any_type && !temp.from_value_type
                    .valid_conversions()
                    .contains(&input.value.value_type()))
            {
                response.is_disabled = true;
            }
        }
    }

    response.is_cursor_over = input_response.hovered();

    // While a connection is being dragged out of an OUTPUT, this input is a live
    // drop target if the cursor sits anywhere inside its enlarged, reaches-left
    // `release_rect`. We test cursor geometry directly rather than `hovered()`
    // because egui keeps hover routed to the dragged source dot for the whole
    // drag, so the target dot never reports hover. Only valid (non-disabled)
    // targets light up, matching what a release will actually connect to.
    let is_drop_target = matches!(
        temp_connection,
        Some(t) if t.from_connection_type == ConnectionType::Output
    ) && !response.is_disabled
        && release_rect.contains(cursor_position);

    if input.is_error {
        color = theme.get().grid_connection_dot_error;
    } else if response.is_disabled {
        color = theme.get().grid_connection_dot_disabled;
    } else if input_response.hovered() || is_drop_target {
        color = theme.get().grid_connection_dot_hover;
    }

    // Grow the dot while it is the active drop target so the user can see, before
    // releasing, exactly which input the connection will land on.
    let dot_radius = if is_drop_target { 8.0 } else { 5.0 };
    let shape = Shape::circle_filled(input_position, graph_to_view_space(graph_zoom, dot_radius), color);
    ui.painter().add(shape);

    // Outline indicates this input is exposed for subgraph composition.
    // Uses the theme's selected-border color so it matches the visual
    // vocabulary already established for "this thing is highlighted."
    if input.is_exposed {
        ui.painter().add(Shape::circle_stroke(
            input_position,
            graph_to_view_space(graph_zoom, 7.0),
            egui::Stroke::new(
                graph_to_view_space(graph_zoom, 1.5),
                theme.get().node_header_selected_border,
            ),
        ));
    }

    if input_response.drag_started() {
        response.has_started_creating_connection = true;
        response.connection_from_position = input_position;
    } else if input_response.drag_stopped() {
        response.has_stopped_creating_connection = true;
        response.connection_to_position = input_position;
    }

    // Offset the label from the dot by its radius + a fixed 5px gap, so the gap
    // stays constant even when the dot grows to a drop target (radius 8). With a
    // flat 10px offset the enlarged dot would crowd the name (only ~2px clear).
    let label_offset = dot_radius + 5.0;
    let pos = Pos2::new(input_position.x - graph_to_view_space(graph_zoom, label_offset), input_position.y);
    let font_id = egui::FontId::proportional(graph_to_view_space(graph_zoom, 12.0));
    let color = theme.get().override_text_color;

    if show_names || response.is_cursor_over || is_drop_target {
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
        // Build the list of type labels to display
        let type_labels: Vec<String> = if input.accepts_any_type {
            vec!["any".to_string()]
        } else {
            let conversions = input.value.value_type().valid_conversions_from();
            if conversions.is_empty() {
                vec!["none".to_string()]
            } else {
                conversions.iter().map(|t| t.value_name()).collect()
            }
        };

        for (i, txt) in type_labels.iter().enumerate() {
            let txt_pos = Pos2::new(pos.x, graph_to_view_space(graph_zoom, 25.0) + pos.y + graph_to_view_space(graph_zoom, 15.0) * i as f32);
            let galley = ui.painter().layout_no_wrap(txt.clone(), font_id.clone(), theme.get().text_faint);

            // bg
            ui.painter().rect_filled(Rect::from_min_size(Pos2::new(txt_pos.x - galley.rect.width(), txt_pos.y - (galley.rect.height() * 0.5)), galley.rect.size()), egui::CornerRadius::same(1), theme.get().grid_bg);

            // text
            ui.painter().text(
                txt_pos,
                egui::Align2::RIGHT_CENTER,
                txt.clone(),
                font_id.clone(),
                theme.get().text_faint,
            );
        }
    }

    response
}
