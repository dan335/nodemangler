use crate::graph::graph_input::draw_graph_input;
use crate::graph::graph_node_header::show_graph_node_header;
use crate::graph::graph_node_info::show_graph_node_info;
use crate::graph::graph_output::draw_graph_output;
use crate::themes::theme::Theme;
use crate::{graph_to_view_space_pos2, view_to_graph_space_pos2, graph_to_view_space, NODE_SIZE};
use eframe::egui;
use egui::{Pos2, Rect, Vec2};
use mangler_core::input::Input;
use mangler_core::node_settings::NodeSettings;
use mangler_core::output::Output;
use std::fmt::Debug;
use std::time::Duration;

use super::graph_editor::TempConnection;
use super::graph_node_thumbnail::GraphNodeThumbnail;
use super::graph_output::draw_graph_output_highlighted;

#[derive(Clone)]
pub struct GraphNode {
    pub id: String,
    pub position: egui::Pos2,
    pub settings: NodeSettings,
    pub inputs: Vec<Input>,
    pub outputs: Vec<Output>,
    pub time: Option<Duration>,
    pub is_dragging: bool,
    pub last_drag_position: Option<Pos2>,
    pub thumbnail: Option<GraphNodeThumbnail>,
    pub is_subgraph: bool,
    pub is_busy: bool,
    pub is_error: bool,
    pub error_message: Option<String>,
    pub is_enabled: bool,
}

impl GraphNode {
    pub fn new(
        id: String,
        position: Pos2,
        settings: NodeSettings,
        inputs: Vec<Input>,
        outputs: Vec<Output>,
        is_subgraph: bool,
    ) -> GraphNode {
        GraphNode {
            id,
            position,
            settings,
            is_dragging: false,
            last_drag_position: None,
            thumbnail: None,
            inputs,
            outputs,
            time: None,
            is_subgraph,
            is_busy: false,
            is_error: false,
            error_message: None,
            is_enabled: true,
        }
    }

    pub fn get_rect(&self, graph_position: Pos2, graph_zoom: f32) -> Rect {
        let node_view_pos = graph_to_view_space_pos2(graph_zoom, self.position);
        let graph_view_pos = graph_to_view_space_pos2(graph_zoom, graph_position);

        let graph_pos = Pos2::new(
            graph_view_pos.x + node_view_pos.x,
            graph_view_pos.y + node_view_pos.y,
        );
        //println!("graph pos node {:?}", graph_pos);
        //let view_pos = graph_to_view_space_pos2(graph_zoom, graph_pos);
        let view_size = graph_to_view_space_pos2(graph_zoom, NODE_SIZE.to_pos2());
        Rect::from_center_size(graph_pos, view_size.to_vec2())
    }

    pub fn show(
        &mut self,
        ui: &mut egui::Ui,
        graph_position: Pos2,
        graph_zoom: f32,
        panel_cursor_position: Pos2,
        is_editing: bool,
        is_viewing: Option<usize>,
        temp_connection: Option<TempConnection>,
        theme: &Theme,
    ) -> GraphNodeResponse {
        puffin::profile_scope!("graph node.show()");
        let mut graph_node_response = GraphNodeResponse::default();

        if self.is_dragging {
            if let Some(last_drag_position) = self.last_drag_position {
                self.position += view_to_graph_space_pos2(
                    graph_zoom,
                    panel_cursor_position - last_drag_position.to_vec2(),
                )
                .to_vec2();
                graph_node_response.new_position = Some(self.position);
            }

            self.last_drag_position = Some(panel_cursor_position);
        }

        let node_rect = self.get_rect(graph_position, graph_zoom);

        let bg_response = ui.allocate_rect(
            node_rect,
            egui::Sense::click()
                .union(egui::Sense::drag())
                .union(egui::Sense::hover()),
        );

        if bg_response.clicked_by(egui::PointerButton::Primary) {
            self.stop_dragging();
            graph_node_response.is_left_click = true;
        } else if bg_response.clicked_by(egui::PointerButton::Secondary) {
            self.stop_dragging();
            graph_node_response.is_right_click = true;
        } else if bg_response.drag_started_by(egui::PointerButton::Primary) {
            self.start_dragging();
        } else if bg_response.drag_stopped_by(egui::PointerButton::Primary) {
            self.stop_dragging();
        }

        graph_node_response.is_cursor_inside = bg_response.hovered();

        show_graph_node_header(
            ui,
            &self.settings.name,
            node_rect,
            is_editing,
            self.is_subgraph,
            graph_zoom,
            theme,
            self.is_busy,
            self.is_enabled,
        );

        show_graph_node_info(ui, self.time, node_rect, graph_zoom, theme);

        if let Some(thumbnail) = &self.thumbnail {
            thumbnail.show(ui, self.get_rect(graph_position, graph_zoom).center_bottom(), graph_zoom, theme);
        }

        // ------------
        // inputs
        for (index, input) in self.inputs.iter().enumerate() {
            puffin::profile_scope!("graph node.inputs.iter()");
            // draw input
            let input_output_response = draw_graph_input(
                &self.id,
                input,
                self.get_input_position(index, node_rect, graph_zoom),
                self.get_input_rect(index, node_rect, graph_zoom),
                index,
                node_rect,
                ui,
                bg_response.hovered(),
                temp_connection.as_ref(),
                theme,
                graph_zoom,
            );

            if input_output_response.has_started_creating_connection {
                graph_node_response.temp_connection = Some(TempConnection {
                    from_position: input_output_response.connection_from_position,
                    from_node_id: self.id.clone(),
                    from_connection_index: index,
                    from_connection_type: ConnectionType::Input,
                    from_value_type: input.value.value_type(),
                    from_accepts_any_type: input.accepts_any_type,
                });
            }

            if !input_output_response.is_disabled
                && input_output_response.has_stopped_creating_connection
            {
                graph_node_response.has_stopped_creating_connection = true;
                graph_node_response.connection_to_position =
                    input_output_response.connection_to_position;
            }

            if !input_output_response.is_disabled && input_output_response.is_cursor_over {
                graph_node_response.is_cursor_inside = true;
            }
        }

        // outputs
        for (index, output) in self.outputs.iter().enumerate() {
            puffin::profile_scope!("graph node.outputs.iter()");
            let input_output_response = draw_graph_output(
                &self.id,
                &output,
                &output.value.value_type().value_name(),
                self.get_output_position(index, node_rect, graph_zoom),
                self.get_output_rect(index, node_rect, graph_zoom),
                index,
                node_rect,
                ui,
                bg_response.hovered(),
                temp_connection.as_ref(),
                theme,
                graph_zoom
            );

            if let Some(view_output_index) = input_output_response.view_output {
                graph_node_response.view_node = Some(view_output_index);
            }

            // started dragging from connection
            // create temp connection object
            if input_output_response.has_started_creating_connection {
                graph_node_response.temp_connection = Some(TempConnection {
                    from_position: input_output_response.connection_from_position,
                    from_node_id: self.id.clone(),
                    from_connection_index: index,
                    from_connection_type: ConnectionType::Output,
                    from_value_type: output.value.value_type(),
                    from_accepts_any_type: false,
                });
            }

            if !input_output_response.is_disabled
                && input_output_response.has_stopped_creating_connection
            {
                graph_node_response.has_stopped_creating_connection = true;
                graph_node_response.connection_to_position =
                    input_output_response.connection_to_position;
            }

            if let Some(viewing_index) = is_viewing {
                if viewing_index == index {
                    draw_graph_output_highlighted(self.get_output_position(index, node_rect, graph_zoom), ui, theme, graph_zoom);
                }
            }

            // if is_viewing && index == 0 {
            //     draw_graph_output_highlighted(self.get_output_position(index, node_rect, graph_zoom), ui, theme, graph_zoom);
            // }

            if !input_output_response.is_disabled && input_output_response.is_cursor_over {
                graph_node_response.is_cursor_inside = true;
            }
        }

        graph_node_response
    }

    fn start_dragging(&mut self) {
        self.is_dragging = true;
    }

    fn stop_dragging(&mut self) {
        self.is_dragging = false;
        self.last_drag_position = None;
    }

    pub fn get_input_position(&self, index: usize, node_rect: Rect, graph_zoom: f32) -> Pos2 {
        Pos2::new(
            node_rect.left() - graph_to_view_space(graph_zoom, 14.0),
            node_rect.top() + graph_to_view_space(graph_zoom, 12.0) + graph_to_view_space(graph_zoom, 20.0) * index as f32,
        )
    }

    pub fn get_output_position(&self, index: usize, node_rect: Rect, graph_zoom: f32) -> Pos2 {
        Pos2::new(
            node_rect.right() + graph_to_view_space(graph_zoom, 14.0),
            node_rect.top() + graph_to_view_space(graph_zoom, 12.0) + graph_to_view_space(graph_zoom, 20.0) * index as f32,
        )
    }

    pub fn get_input_rect(&self, index: usize, node_rect: Rect, graph_zoom: f32) -> Rect {
        puffin::profile_scope!("graph node.get_input_rect()");
        Rect::from_center_size(
            self.get_input_position(index, node_rect, graph_zoom),
            Vec2::new(12.0, 12.0),
        )
    }

    pub fn get_output_rect(&self, index: usize, node_rect: Rect, graph_zoom: f32) -> Rect {
        puffin::profile_scope!("graph node.get_output_rect()");
        Rect::from_center_size(
            self.get_output_position(index, node_rect, graph_zoom),
            Vec2::new(12.0, 12.0),
        )
    }

    pub fn set_input_connection(
        &mut self,
        input_index: usize,
        output_id: String,
        output_index: usize,
    ) {
        puffin::profile_scope!("graph node.set_input_connection()");
        self.inputs[input_index].connection = Some((output_id, output_index));
    }

    pub fn clear_input_connection(&mut self, input_index: usize) {
        self.inputs[input_index].connection = None;
    }

    pub fn set_output_connection(
        &mut self,
        output_index: usize,
        input_id: String,
        input_index: usize,
    ) {
        puffin::profile_scope!("graph node.set_output_connection()");
        if self.outputs[output_index].connection.is_some() {
            self.outputs[output_index]
                .connection
                .as_mut()
                .unwrap()
                .push((input_id, input_index));
        } else {
            self.outputs[output_index].connection = Some(vec![(input_id, input_index)]);
        }
    }

    /// Remove a specific downstream connection from an output.
    /// Identifies the connection by the target node ID and input index.
    /// Sets the connection field to `None` if no connections remain.
    pub fn clear_output_connection(
        &mut self,
        output_index: usize,
        input_node_id: &str,
        input_index: usize,
    ) {
        if let Some(c) = self.outputs.get_mut(output_index) {
            if let Some(d) = c.connection.as_mut() {
                d.retain(|(id, idx)| !(id == input_node_id && *idx == input_index));
                if d.is_empty() {
                    c.connection = None;
                }
            }
        }
    }
}

#[cfg(test)]
#[path = "graph_node_tests.rs"]
mod tests;

#[derive(Debug)]
pub struct GraphNodeResponse {
    pub temp_connection: Option<TempConnection>,
    pub has_stopped_creating_connection: bool,
    pub connection_to_position: Pos2,
    pub view_node: Option<usize>,   // usize = output index to view
    pub is_right_click: bool,
    pub is_left_click: bool,
    pub is_cursor_inside: bool,
    pub new_position: Option<Pos2>,
}

impl GraphNodeResponse {
    pub fn default() -> GraphNodeResponse {
        GraphNodeResponse {
            temp_connection: None,
            has_stopped_creating_connection: false,
            connection_to_position: Pos2::ZERO,
            view_node: None,
            is_right_click: false,
            is_left_click: false,
            is_cursor_inside: false,
            new_position: None,
        }
    }
}

pub struct InputOutputResponse {
    pub has_started_creating_connection: bool,
    pub connection_from_position: Pos2,
    pub has_stopped_creating_connection: bool,
    pub connection_to_position: Pos2,
    pub is_cursor_over: bool,
    pub is_disabled: bool,
    pub view_output: Option<usize>, // clicked on output to view it
}

impl InputOutputResponse {
    pub fn new() -> InputOutputResponse {
        InputOutputResponse {
            has_started_creating_connection: false,
            connection_from_position: Pos2::ZERO,
            has_stopped_creating_connection: false,
            connection_to_position: Pos2::ZERO,
            is_cursor_over: false,
            is_disabled: false,
            view_output: None,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum ConnectionType {
    Input,
    Output,
}
