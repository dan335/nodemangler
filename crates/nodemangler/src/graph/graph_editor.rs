use eframe::{egui::{self}};
use egui::Pos2;
use mangler::{nodes::{node_settings::NodeSettings, node::Node}};
use std::{collections::HashMap, time::{Instant, Duration}};
use egui::epaint::{CubicBezierShape};
use crate::{graph::graph_node::GraphNode, NewConnection};
use super::graph_node::ConnectionType;

const DOUBLE_CLICK_DURATION: Duration = Duration::from_millis(500);

pub struct GraphEditor {
    position: Pos2,
    is_dragging: bool,
    last_drag_position: Option<Pos2>,
    pub graph_nodes: HashMap<String, GraphNode>,
    temp_connection: Option<TempConnection>,

    // if a node was clicked on when was it clicked and what is it's node_id
    // used to check for click or double click
    last_node_click: Option<(Instant, String)>,    
}

impl GraphEditor {
    pub fn new() -> GraphEditor {
        GraphEditor {
            position: Pos2::ZERO,
            is_dragging: false,
            last_drag_position: None,
            graph_nodes: HashMap::default(),
            temp_connection: None,
            last_node_click: None,
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

        // clicking on nodes
        // check if click is less than DOUBLE_CLICK_DURATION
        // but no other click has occured
        if let Some(last_node_click) = &self.last_node_click {
            if last_node_click.0.elapsed() > DOUBLE_CLICK_DURATION {
                // click occured
                graph_editor_response.editing_node_id = Some(last_node_click.1.clone());
                self.last_node_click = None;
            }
        }

        // draw nodes
        let mut has_stopped_creating_connection = false;
        //let mut connection_to_position = Pos2::ZERO;

        for (graph_node_id, graph_node) in self.graph_nodes.iter_mut() {
            let graph_node_response = graph_node.show(ui, self.position, cursor_position, &nodes[&graph_node.id]);

            // new temp connection
            if let Some(temp_connection) = graph_node_response.temp_connection {
                self.temp_connection = Some(temp_connection.clone());
            }

            // new connection
            if graph_node_response.has_stopped_creating_connection {
                has_stopped_creating_connection = true;
                //connection_to_position = graph_node_response.connection_to_position;
            }

            // click on node
            if graph_node_response.is_click {
                //graph_editor_response.is_click_node_id = Some(graph_node_id.clone());
                if let Some(last_node_click) = &self.last_node_click {
                    if &last_node_click.1 == graph_node_id {
                        // check for double click
                        if last_node_click.0.elapsed() <= DOUBLE_CLICK_DURATION {
                            // double click
                            graph_editor_response.viewing_node_id = Some(graph_node_id.clone());
                            self.last_node_click = None;
                        } else {
                            // previous click is old
                            // save click
                            self.last_node_click = Some((Instant::now(), graph_node_id.clone()));
                        }
                    } else {
                        // different node was clicked on
                        // save click
                        self.last_node_click = Some((Instant::now(), graph_node_id.clone()));
                    }
                    
                } else {
                    // no previous click
                    // save click
                    self.last_node_click = Some((Instant::now(), graph_node_id.clone()));
                }
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
            match temp_connection.from_connection_type {
                ConnectionType::Input => self.draw_connection_line(ui, cursor_position, temp_connection.from_position),
                ConnectionType::Output => self.draw_connection_line(ui, temp_connection.from_position, cursor_position),
            }
        }

        // connections
        for (node_id, node) in nodes.iter() {
            for (input_index, input) in node.inputs.iter().enumerate() {
                if let Some((output_node_id, output_connection_index)) = &input.connection {
                    let input_graph_node = &self.graph_nodes[node_id];
                    let output_graph_node = &self.graph_nodes[output_node_id];

                    self.draw_connection_line(ui, output_graph_node.get_output_position(output_connection_index.clone(), self.position), input_graph_node.get_input_position(input_index, self.position));
                }
            }
        }

        if self.last_node_click.is_some() {
            graph_editor_response.request_redraw = true;
        }

        graph_editor_response
    }

    pub fn draw_connection_line(&self, ui: &mut egui::Ui, from: Pos2, to: Pos2) {
        let offset_max = 150.0;
        let color = egui::Color32::from_gray(150);
        let stroke = egui::Stroke::new(2.0, color);

        let distance = from.distance(to);
        let offset = (distance / 2.0).min(offset_max);

        let points = [
            from,
            Pos2::new(from.x + offset, from.y),
            Pos2::new(to.x - offset, to.y),
            to,
            ];

        //let curve_shape = CubicBezierShape::from_points_stroke(points, false, color, stroke);
        let curve_shape = CubicBezierShape {
            points,
            closed: false,
            fill: egui::Color32::from_black_alpha(0),
            stroke,
        };

        ui.painter().add(egui::Shape::CubicBezier(curve_shape));
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

    pub fn edit_node(&mut self, node_id: String) {

    }

    pub fn view_node(&mut self, node_id: String) {

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
    pub is_click_node_id: Option<String>,   // if a node was clicked on return it's id
    pub request_redraw: bool,
    pub editing_node_id: Option<String>,
    pub viewing_node_id: Option<String>,
}

impl GraphEditorResponse {
    fn default() -> GraphEditorResponse {
        GraphEditorResponse {
            new_connection: None,
            is_click_node_id: None,
            request_redraw: false,
            editing_node_id: None,
            viewing_node_id: None,
        }
    }
}


