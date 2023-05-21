use super::graph_node::ConnectionType;
use crate::{graph::graph_node::GraphNode, NewConnection, view};
use eframe::egui::{self};
use egui::epaint::CubicBezierShape;
use egui::Pos2;
use mangler::nodes::{node::Node, node_settings::NodeSettings, operation::ConnectionSettings};
use std::{
    collections::HashMap,
    time::{Duration, Instant}, println,
};

const DOUBLE_CLICK_DURATION: Duration = Duration::from_millis(500);

pub struct GraphEditor {
    position: Pos2,
    is_dragging: bool,
    last_drag_position: Option<Pos2>,
    pub graph_nodes: HashMap<String, GraphNode>,
    temp_connection: Option<TempConnection>,
    previous_cursor_primary_down: Option<bool>,

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
            previous_cursor_primary_down: None,
        }
    }

    pub fn show(
        &mut self,
        ui: &mut egui::Ui,
        cursor_position: Pos2,
        cursor_primary_down: bool,
        editing_node_id: &Option<String>,
        viewing_node_id: &Option<String>,
    ) -> GraphEditorResponse {
        let mut graph_editor_response = GraphEditorResponse::default();

        let editor_rect = ui.max_rect();
        ui.allocate_rect(editor_rect, egui::Sense::hover());

        ui.set_clip_rect(editor_rect);

        let cursor_inside = editor_rect.contains(cursor_position);
        let mut cursor_primary_went_down = false;   // did mouse button go down this frame
        let mut cursor_primary_went_up = false; // did mous button go up this rame
        let mut is_cursor_over_node = false;

        if let Some(previous_cursor_primary_down) = self.previous_cursor_primary_down {
            if previous_cursor_primary_down && !cursor_primary_down {
                cursor_primary_went_up = true;
            }
            if !previous_cursor_primary_down && cursor_primary_down {
                cursor_primary_went_down = true;
            }
        }

        // mouse
        if cursor_primary_went_up {
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
        // if let Some(last_node_click) = &self.last_node_click {
        //     if last_node_click.0.elapsed() > DOUBLE_CLICK_DURATION {
        //         // click occured
        //         graph_editor_response.editing_node_id = Some(last_node_click.1.clone());
        //         self.last_node_click = None;
        //     }
        // }

        // draw nodes
        let mut has_stopped_creating_connection = false;
        //let mut connection_to_position = Pos2::ZERO;

        for (graph_node_id, graph_node) in self.graph_nodes.iter_mut() {
            
            // are we editing node
            let mut is_editing = false;
            if let Some(n) = editing_node_id {
                if n == graph_node_id {
                    is_editing = true;
                }
            }

            // are we viewing node
            let mut is_viewing = false;
            if let Some(n) = viewing_node_id {
                if n == graph_node_id {
                    is_viewing = true;
                }
            }

            // draw node
            let graph_node_response =
                graph_node.show(ui, self.position, cursor_position, is_editing, is_viewing);
                
            // mouse over it?
            if graph_node_response.is_cursor_inside {
                is_cursor_over_node = true;
            }

            // new temp connection
            if let Some(temp_connection) = graph_node_response.temp_connection {
                self.temp_connection = Some(temp_connection.clone());
            }

            // new connection
            if graph_node_response.has_stopped_creating_connection {
                has_stopped_creating_connection = true;
            }

            // click on node
            // edit or delete node
            if graph_node_response.is_left_click {
                let mut is_command_down = false;
                ui.input(|i| {
                    if i.modifiers.command {
                        is_command_down = true;
                    }
                });

                if is_command_down {
                    // delete node
                    graph_editor_response.nodes_to_delete.push(graph_node_id.clone());
                } else {
                    graph_editor_response.editing_node_id = Some(graph_node_id.clone());
                }
            }

            // right click on node
            // view node
            if graph_node_response.is_right_click {
                graph_editor_response.viewing_node_id = Some(graph_node_id.clone());
            }
        }

        

        // ------------------------
        // find if it stopped on a connection
        if has_stopped_creating_connection {
            if let Some(temp_connection) = &self.temp_connection {
                // find node with connection at this position
                for (_, other_graph_node) in self.graph_nodes.iter() {
                    let other_node = &self.graph_nodes[&other_graph_node.id];
                    let other_node_rect = other_graph_node.get_rect(self.position);

                    match temp_connection.from_connection_type {
                        ConnectionType::Input => {
                            for output_index in 0..other_node.outputs.len() {
                                if other_graph_node
                                    .get_output_rect(output_index,other_node_rect)
                                    .contains(cursor_position)
                                {
                                    graph_editor_response.new_connection =
                                        Some(NewConnection::new(
                                            temp_connection.from_node_id.clone(),
                                            temp_connection.from_connection_index,
                                            other_node.id.clone(),
                                            output_index,
                                        ));
                                }
                            }
                        }
                        ConnectionType::Output => {
                            for input_index in 0..other_node.inputs.len() {
                                if other_graph_node
                                    .get_input_rect(input_index, other_node_rect)
                                    .contains(cursor_position)
                                {
                                    graph_editor_response.new_connection = Some(NewConnection {
                                        input_node_id: other_node.id.clone(),
                                        input_connection_index: input_index,
                                        output_node_id: temp_connection.from_node_id.clone(),
                                        output_connection_index: temp_connection
                                            .from_connection_index,
                                    })
                                }
                            }
                        }
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
                ConnectionType::Input => {
                    self.draw_connection_line(ui, cursor_position, temp_connection.from_position)
                }
                ConnectionType::Output => {
                    self.draw_connection_line(ui, temp_connection.from_position, cursor_position)
                }
            };
        }

        // connections
        // collect curves to tell if we clicked on one to delete it
        // curve, input node id, input index
        let mut connection_curves: Vec<(CubicBezierShape, String, usize)> = Vec::new();
        for (node_id, node) in self.graph_nodes.iter() {
            for (input_index, input) in node.inputs.iter().enumerate() {
                if let Some((output_node_id, output_connection_index)) = &input.connection {
                    let input_graph_node = &self.graph_nodes[node_id];
                    let output_graph_node = &self.graph_nodes[output_node_id];

                    let input_node_rect = input_graph_node.get_rect(self.position);
                    let output_node_rect = output_graph_node.get_rect(self.position);

                    let curve = self.draw_connection_line(
                        ui,
                        output_graph_node
                            .get_output_position(*output_connection_index, output_node_rect),
                        input_graph_node.get_input_position(input_index, input_node_rect),
                    );

                    connection_curves.push((curve, node_id.clone(), input_index));
                }
            }
        }

        // ------------------------
        // mouse
        if cursor_primary_went_down && !is_cursor_over_node {
            self.start_dragging();
        }

        if self.last_node_click.is_some() {
            graph_editor_response.request_redraw = true;
        }

        // deleting connections
        ui.input(|i| {
            if i.modifiers.command {
                if cursor_primary_went_down {
                    for (curve, input_node_id, input_index) in connection_curves.iter() {
                        if curve.visual_bounding_rect().contains(cursor_position) {
                            let distance = distance_to_cubic_bezier_curve(cursor_position, curve.points);
                            if distance < 6.0 {
                                graph_editor_response.connections_to_delete.push((input_node_id.clone(), input_index.clone()));
                            }
                        }
                    }
                }
            }
        });

        self.previous_cursor_primary_down = Some(cursor_primary_down);

        graph_editor_response
    }

    // returns curve shape to detect clickin on curve
    pub fn draw_connection_line(&self, ui: &mut egui::Ui, from: Pos2, to: Pos2) -> CubicBezierShape {
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

        curve_shape
    }

    fn start_dragging(&mut self) {
        self.is_dragging = true;
    }

    fn stop_dragging(&mut self) {
        self.is_dragging = false;
        self.last_drag_position = None;
    }

    pub fn add_node(
        &mut self,
        node_id: String,
        node_settings: NodeSettings,
        input_settings: Vec<ConnectionSettings>,
        output_settings: Vec<ConnectionSettings>,
        position: Pos2,
    ) {
        let node = GraphNode::new(
            node_id.clone(),
            position - self.position.to_vec2(),
            node_settings,
            input_settings,
            output_settings,
        );

        self.graph_nodes.insert(node_id, node);
    }

    pub fn remove_node(&mut self, node_id: &String) {
        self.graph_nodes.remove(node_id);
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
    pub is_left_click_node_id: Option<String>, // if a node was clicked on return it's id
    pub is_right_click_node_id: Option<String>,
    pub request_redraw: bool,
    pub editing_node_id: Option<String>,
    pub viewing_node_id: Option<String>,
    pub nodes_to_delete: Vec<String>,
    pub connections_to_delete: Vec<(String, usize)>,    // node id, input index
}

impl GraphEditorResponse {
    fn default() -> GraphEditorResponse {
        GraphEditorResponse {
            new_connection: None,
            is_left_click_node_id: None,
            is_right_click_node_id: None,
            request_redraw: false,
            editing_node_id: None,
            viewing_node_id: None,
            nodes_to_delete: Vec::new(),
            connections_to_delete: Vec::new(),
        }
    }
}


fn distance_to_cubic_bezier_curve(point: Pos2, points: [Pos2; 4]) -> f32 {
    let t = nearest_t(point, points);
    let curve_point = point_at(t, points);
    let dx = curve_point.x - point.x;
    let dy = curve_point.y - point.y;
    return (dx * dx + dy * dy).sqrt();

    fn nearest_t(point: Pos2, points: [Pos2; 4]) -> f32 {
        // Find the nearest t value by iterating and comparing distances
        let mut t = 0.0;
        let mut step = 0.1;
        let mut min_distance = f32::MAX;

        while t <= 1.0 {
            let curve_point = point_at(t, points);
            let dx = curve_point.x - point.x;
            let dy = curve_point.y - point.y;
            let distance = dx * dx + dy * dy;

            if distance < min_distance {
                min_distance = distance;
            } else {
                // If the distance starts increasing, we can stop iterating
                break;
            }

            t += step;
        }

        t - step
    }

    fn point_at(t: f32, points: [Pos2; 4]) -> Pos2 {
        let u = 1.0 - t;
        let tt = t * t;
        let uu = u * u;
        let uuu = uu * u;
        let ttt = tt * t;

        let x =
            uuu * points[0].x
            + 3.0 * uu * t * points[1].x
            + 3.0 * u * tt * points[2].x
            + ttt * points[3].x;

        let y =
            uuu * points[0].y
            + 3.0 * uu * t * points[1].y
            + 3.0 * u * tt * points[2].y
            + ttt * points[3].y;

        Pos2 { x, y }
    }
}