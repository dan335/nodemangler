use eframe::egui::{self, output};
use egui::Pos2;
use mangler::{nodes::{node_settings::NodeSettings, node::Node}};
use std::collections::HashMap;

use crate::{graph::graph_node::GraphNode, NewConnection};

use super::graph_node::ConnectionType;

pub struct GraphEditor {
    position: Pos2,
    is_dragging: bool,
    last_drag_position: Option<Pos2>,
    pub graph_nodes: HashMap<String, GraphNode>,
    temp_connection: Option<TempConnection>,
    
}

impl GraphEditor {
    pub fn new() -> GraphEditor {
        GraphEditor {
            position: Pos2::ZERO,
            is_dragging: false,
            last_drag_position: None,
            graph_nodes: HashMap::default(),
            temp_connection: None,
        }
    }

    pub fn show(&mut self, ui: &mut egui::Ui, cursor_position: Pos2, nodes: &HashMap<String, Node>, cursor_primary_down: bool) -> GraphEditorResponse {
        let mut graph_editor_response = GraphEditorResponse::default();

        let editor_rect = ui.max_rect();
        ui.allocate_rect(editor_rect, egui::Sense::hover());

        ui.set_clip_rect(editor_rect);

        let cursor_inside = editor_rect.contains(cursor_position);

        let bg_response = ui.allocate_rect(
            editor_rect,
            egui::Sense::click().union(egui::Sense::drag()),
        );

        if bg_response.clicked() {
            // clicked on bg
        } else if bg_response.drag_started() {
            self.start_dragging();
        } else if bg_response.drag_released() {
            self.stop_dragging();
        }

        if self.is_dragging && !cursor_inside {
            self.stop_dragging();
        }

        if self.is_dragging {
            if let Some(last_drag_position) = self.last_drag_position {
                self.position += cursor_position - last_drag_position;
            }

            self.last_drag_position = Some(cursor_position);
        }

        // draw nodes
        let mut has_stopped_creating_connection = false;
        let mut connection_to_position = Pos2::ZERO;

        for (_, graph_node) in self.graph_nodes.iter_mut() {
            let graph_node_response = graph_node.show(ui, self.position, cursor_position, &nodes[&graph_node.id]);

            // new temp connection
            if let Some(temp_connection) = graph_node_response.temp_connection {
                self.temp_connection = Some(temp_connection.clone());
            }

            // new connection
            if graph_node_response.has_stopped_creating_connection {
                has_stopped_creating_connection = true;
                connection_to_position = graph_node_response.connection_to_position;
            }
        }

        // find if it stopped on a connection
        if has_stopped_creating_connection {
            if let Some(temp_connection) = &self.temp_connection {
                // find node with connection at this position
                for (_, other_graph_node) in self.graph_nodes.iter() {
                    let other_node = &nodes[&other_graph_node.id];

                    match temp_connection.from_connection_type {
                        ConnectionType::Input => {
                            for output_index in 0..other_node.outputs.len() {
                                if other_graph_node.get_output_rect(output_index, self.position).contains(cursor_position) {
                                    graph_editor_response.new_connection = Some(NewConnection::new(temp_connection.from_node_id.clone(), temp_connection.from_connection_index, other_node.id.clone(), output_index));
                                }
                            }
                        },
                        ConnectionType::Output => {
                            for input_index in 0..other_node.inputs.len() {
                                if other_graph_node.get_input_rect(input_index, self.position).contains(cursor_position) {
                                    graph_editor_response.new_connection = Some(NewConnection { input_node_id: other_node.id.clone(), input_connection_index: input_index, output_node_id: temp_connection.from_node_id.clone(), output_connection_index: temp_connection.from_connection_index })
                                }
                            }
                        },
                    }                    
                }
            }
        }

        if self.temp_connection.is_some() && !cursor_primary_down {
            self.temp_connection = None;
        }

        // temp connection being created
        if let Some(temp_connection) = &self.temp_connection {
            let mut points: Vec<Pos2> = Vec::with_capacity(2);
            points.push(temp_connection.from_position);
            points.push(cursor_position);
            let stroke = egui::Stroke::new(1.0, egui::Color32::from_gray(150));
            ui.painter().add(egui::Shape::line(points, stroke));
        }

        // connections
        for (node_id, node) in nodes.iter() {
            for input in node.inputs.iter() {
                if let Some(connection_id) = &input.connection {
                    let other_node = &nodes[connection_id];
                    
                }
            }
        }

        graph_editor_response
    }

    fn start_dragging(&mut self) {
        self.is_dragging = true;
    }

    fn stop_dragging(&mut self) {
        self.is_dragging = false;
            self.last_drag_position = None;
    }

    pub fn add_node(&mut self, node_id: String, node_settings: NodeSettings, position: Pos2) {
        let node = GraphNode::new(node_id.clone(), position - self.position.to_vec2(), node_settings);
        self.graph_nodes.insert(node_id, node);
    }

    
}



// connection that is being created
// goes to cursor_position
#[derive(Debug, Clone)]
pub struct TempConnection {
    pub from_position: Pos2,
    pub from_node_id: String,
    pub from_connection_index: usize,
    pub from_connection_type: ConnectionType,
}


pub struct GraphEditorResponse {
    pub new_connection: Option<NewConnection>,
}

impl GraphEditorResponse {
    fn default() -> GraphEditorResponse {
        GraphEditorResponse {
            new_connection: None,
        }
    }
}


