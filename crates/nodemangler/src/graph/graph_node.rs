use crate::graph::graph_input::draw_graph_input;
use crate::graph::graph_node_header::show_graph_node_header;
use crate::graph::graph_node_info::show_graph_node_info;
use crate::graph::graph_node_thumbnail::show_graph_node_thumbnail;
use crate::graph::graph_output::draw_graph_output;
use crate::{graph_to_view_space_pos2, view_to_graph_space_pos2};
use eframe::egui;
use egui::{Pos2, Rect, Vec2};
use mangler::input::Input;
use mangler::node_settings::NodeSettings;
use mangler::output::Output;
use std::fmt::Debug;
use std::time::Duration;

use super::graph_editor::TempConnection;
use super::graph_output::draw_graph_output_highlighted;

pub const NODE_SIZE: Vec2 = Vec2::new(128.0, 40.0);
//pub const THUMBNAIL_SIZE: [u32; 2] = [128, 128];

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
    pub thumbnail: Option<egui::TextureHandle>,
    pub is_subgraph: bool,
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
        is_viewing: bool,
        temp_connection: Option<TempConnection>,
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
        } else if bg_response.drag_released_by(egui::PointerButton::Primary) {
            self.stop_dragging();
        }

        graph_node_response.is_cursor_inside = bg_response.hovered();

        show_graph_node_header(
            ui,
            self.settings.name.clone(),
            node_rect,
            is_editing,
            self.is_subgraph,
        );

        show_graph_node_info(ui, self.time, node_rect, &self.outputs);

        show_graph_node_thumbnail(
            ui,
            &self.outputs,
            self.thumbnail.clone(),
            self.get_rect(graph_position, graph_zoom).center_bottom(),
            graph_zoom,
        );

        // ------------
        // inputs
        for (index, input) in self.inputs.iter().enumerate() {
            puffin::profile_scope!("graph node.inputs.iter()");
            // draw input
            let input_output_response = draw_graph_input(
                &self.id,
                input,
                self.get_input_position(index, node_rect),
                self.get_input_rect(index, node_rect),
                index,
                node_rect,
                ui,
                bg_response.hovered(),
                temp_connection.clone(),
            );

            if input_output_response.has_started_creating_connection {
                graph_node_response.temp_connection = Some(TempConnection {
                    from_position: input_output_response.connection_from_position,
                    from_node_id: self.id.clone(),
                    from_connection_index: index,
                    from_connection_type: ConnectionType::Input,
                    from_value_type: input.value.value_type(),
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
                &output.value.value_name(),
                self.get_output_position(index, node_rect),
                self.get_output_rect(index, node_rect),
                index,
                node_rect,
                ui,
                bg_response.hovered(),
                temp_connection.clone(),
            );

            // started dragging from connection
            // create temp connection object
            if input_output_response.has_started_creating_connection {
                graph_node_response.temp_connection = Some(TempConnection {
                    from_position: input_output_response.connection_from_position,
                    from_node_id: self.id.clone(),
                    from_connection_index: index,
                    from_connection_type: ConnectionType::Output,
                    from_value_type: output.value.value_type(),
                });
            }

            if !input_output_response.is_disabled
                && input_output_response.has_stopped_creating_connection
            {
                graph_node_response.has_stopped_creating_connection = true;
                graph_node_response.connection_to_position =
                    input_output_response.connection_to_position;
            }

            if is_viewing && index == 0 {
                draw_graph_output_highlighted(self.get_output_position(index, node_rect), ui);
            }

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

    pub fn get_input_position(&self, index: usize, node_rect: Rect) -> Pos2 {
        puffin::profile_scope!("graph node.get_input_position()");
        Pos2::new(
            node_rect.left() - 14.0,
            node_rect.top() + 12.0 + 20.0 * index as f32,
        )
    }

    pub fn get_output_position(&self, index: usize, node_rect: Rect) -> Pos2 {
        puffin::profile_scope!("graph node.get_output_position()");
        Pos2::new(
            node_rect.right() + 14.0,
            node_rect.top() + 12.0 + 20.0 * index as f32,
        )
    }

    pub fn get_input_rect(&self, index: usize, node_rect: Rect) -> Rect {
        puffin::profile_scope!("graph node.get_input_rect()");
        Rect::from_center_size(
            self.get_input_position(index, node_rect),
            Vec2::new(12.0, 12.0),
        )
    }

    pub fn get_output_rect(&self, index: usize, node_rect: Rect) -> Rect {
        puffin::profile_scope!("graph node.get_output_rect()");
        Rect::from_center_size(
            self.get_output_position(index, node_rect),
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
}

#[derive(Debug)]
pub struct GraphNodeResponse {
    pub temp_connection: Option<TempConnection>,
    pub has_stopped_creating_connection: bool,
    pub connection_to_position: Pos2,
    pub edit_node: bool,
    pub view_node: bool,
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
            edit_node: false,
            view_node: false,
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
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum ConnectionType {
    Input,
    Output,
}
