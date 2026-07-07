use crate::{graph::graph_node::InputOutputResponse, graph_to_view_space, themes::theme::Theme};
use eframe::{
    egui,
    epaint::{Pos2, Rect, Shape},
};
use epaint::CornerRadius;

use super::{graph_editor::TempConnection, graph_node::ConnectionType};
use mangler_core::output::Output;

pub fn draw_graph_output(
    node_id: &String,
    output: &Output,
    output_value_name: &String,
    output_position: Pos2,
    input_rect: Rect,
    release_rect: Rect,
    index: usize,
    _rect: Rect,
    ui: &mut egui::Ui,
    show_type: bool,
    temp_connection: Option<&TempConnection>,
    theme: &Theme,
    graph_zoom: f32,
    cursor_position: Pos2,
) -> InputOutputResponse {
    puffin::profile_scope!("graph node.draw_graph_output()");
    let mut response = InputOutputResponse::new();
    let mut color = theme.get().grid_connection_dot;
    let output_response =
        ui.allocate_rect(input_rect, egui::Sense::drag().union(egui::Sense::hover()).union(egui::Sense::click()));

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
                || (!temp.from_accepts_any_type && !temp.from_value_type
                    .valid_conversions()
                    .contains(&output.value.value_type()))
            {
                response.is_disabled = true;
            }
        }
    }

    if output_response.clicked_by(egui::PointerButton::Secondary) {
        response.view_output = Some(index);
    }

    // While a connection is being dragged out of an INPUT, this output is a live
    // drop target if the cursor sits anywhere inside its enlarged, reaches-right
    // `release_rect`. Tested via cursor geometry (not `hovered()`) because egui
    // keeps hover on the dragged source dot for the whole drag.
    let is_drop_target = matches!(
        temp_connection,
        Some(t) if t.from_connection_type == ConnectionType::Input
    ) && !response.is_disabled
        && release_rect.contains(cursor_position);

    // highlight when hovering
    if response.is_disabled {
        color = theme.get().grid_connection_dot_disabled;
    } else if output_response.hovered() || is_drop_target {
        color = theme.get().grid_connection_dot_hover;
    }

    // draw bg — grow the dot while it is the active drop target so the user can
    // see which output the connection will land on before releasing.
    let dot_radius = if is_drop_target { 8.0 } else { 5.0 };
    let shape = Shape::circle_filled(output_position, graph_to_view_space(graph_zoom, dot_radius), color);
    response.is_cursor_over = output_response.hovered();
    ui.painter().add(shape);

    // Outline indicates this output is exposed for subgraph composition.
    // Uses the theme's selected-border color so it matches the visual
    // vocabulary already established for "this thing is highlighted."
    if output.is_exposed {
        ui.painter().add(Shape::circle_stroke(
            output_position,
            graph_to_view_space(graph_zoom, 7.0),
            egui::Stroke::new(
                graph_to_view_space(graph_zoom, 1.5),
                theme.get().node_header_selected_border,
            ),
        ));
    }

    // creating connections
    if output_response.drag_started() {
        response.has_started_creating_connection = true;
        response.connection_from_position = output_position;
    } else if output_response.drag_stopped() {
        response.has_stopped_creating_connection = true;
        response.connection_to_position = output_position;
    }

    // show name and type when hovering
    if show_type || response.is_cursor_over || is_drop_target {
        puffin::profile_scope!("graph node.show type when hovering");

        let txt = format!("{} ({})", output.name, output_value_name);
        let font_id = egui::FontId::proportional(crate::graph_to_view_space(graph_zoom, 12.0));
        let color = theme.get().override_text_color;
        // Offset the label from the dot by its radius + a fixed 5px gap, so the
        // gap stays constant even when the dot grows to a drop target (radius 8).
        // A flat 10px offset would let the enlarged dot crowd the name.
        let label_offset = dot_radius + 5.0;
        let pos = Pos2::new(output_position.x + graph_to_view_space(graph_zoom, label_offset), output_position.y);

        let galley = ui.painter().layout_no_wrap(txt.clone(), font_id.clone(), color);
        
        // bg
        ui.painter().rect_filled(Rect::from_min_size(Pos2::new(pos.x, pos.y - (galley.rect.height() * 0.5)), galley.rect.size()), CornerRadius::same(1), theme.get().grid_bg);

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

pub fn draw_graph_output_highlighted(output_position: Pos2, ui: &mut egui::Ui, theme: &Theme, graph_zoom: f32) {
    ui.painter().add(
        Shape::circle_filled(
            Pos2::new(output_position.x, output_position.y),
            graph_to_view_space(graph_zoom, 5.5),
            theme.get().node_header_selected_border,
            // egui::Stroke::new(
            //     graph_to_view_space(graph_zoom, 3.0),
            //     theme.get().node_header_selected_border
            // )
        )
    );
}
