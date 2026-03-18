use super::graph_node::ConnectionType;
use crate::{
    graph::graph_node::GraphNode, graph_to_view_space, view_to_graph_space,
    view_to_graph_space_pos2, program::NewConnection, themes::theme::Theme,
};
use eframe::{
    egui,
    epaint::{CornerRadius, Rect, Stroke},
};
use egui::epaint::CubicBezierShape;
use egui::Pos2;
use mangler_core::{input::Input, node_settings::NodeSettings, output::Output, value::ValueType};
use std::{collections::HashMap, time::Instant};

const ZOOM_MULTIPLIER: f32 = 0.001;
const ZOOM_BOUNDS: [f32; 2] = [0.15, 5.0];

pub struct GraphEditor {
    pub position: Pos2,
    pub zoom: f32,
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
            zoom: 1.0,
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
        viewing_node_id_index: &Option<(String, usize)>,
        theme: &Theme,
        is_mouse_over_viewer: bool,
    ) -> GraphEditorResponse {
        puffin::profile_scope!("graph panel.show()");
        let mut graph_editor_response = GraphEditorResponse::default();

        let editor_rect = ui.max_rect();
        let editor_bg_response =
            ui.allocate_rect(editor_rect, egui::Sense::drag().union(egui::Sense::hover()));
        //let panel_cursor_position = Pos2::new(cursor_position.x - editor_rect.min.x, cursor_position.y - editor_rect.min.y);

        if editor_rect.contains(cursor_position) && !is_mouse_over_viewer {
            ui.ctx().input(|input_state| {
                // let mouse_x = cursor_position.x - editor_rect.min.x;
                // let mouse_y = cursor_position.y - editor_rect.min.y;
                //println!("{} {}, {:?}", mouse_x, mouse_y, self.position);
                let new_zoom = (self.zoom * (1.0 + input_state.smooth_scroll_delta.y * ZOOM_MULTIPLIER))
                    .min(ZOOM_BOUNDS[1])
                    .max(ZOOM_BOUNDS[0]);

                let old_x = view_to_graph_space(self.zoom, editor_rect.max.x - editor_rect.min.x);
                let new_x = view_to_graph_space(new_zoom, editor_rect.max.x - editor_rect.min.x);
                let old_y = view_to_graph_space(self.zoom, editor_rect.max.y - editor_rect.min.y);
                let new_y = view_to_graph_space(new_zoom, editor_rect.max.y - editor_rect.min.y);

                let mouse_percent_x = cursor_position.x / (editor_rect.max.x - editor_rect.min.x);
                let mouse_percent_y = cursor_position.y / (editor_rect.max.y - editor_rect.min.y);

                self.position.x += view_to_graph_space(
                    new_zoom,
                    mouse_percent_x * graph_to_view_space(new_zoom, new_x - old_x),
                );
                self.position.y += view_to_graph_space(
                    new_zoom,
                    mouse_percent_y * graph_to_view_space(new_zoom, new_y - old_y),
                );

                self.zoom = new_zoom;
            });
        }


        ui.set_clip_rect(editor_rect);

        // bg
        ui.painter().add(egui::Shape::rect_filled(
            editor_rect,
            CornerRadius::ZERO,
            theme.get().grid_bg,
        ));

        self.draw_background_grid(ui, editor_rect, self.position, theme);

        let cursor_inside = editor_rect.contains(cursor_position);

        let mut cursor_primary_went_down = false; // did mouse button go down this frame
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
                //self.position += (cursor_position - last_drag_position) *(1.0 / self.zoom);

                self.position += view_to_graph_space_pos2(
                    self.zoom,
                    cursor_position - last_drag_position.to_vec2(),
                )
                .to_vec2();
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

        // connections
        // collect curves to tell if we clicked on one to delete it
        // curve, input node id, input index
        let mut connection_curves: Vec<(CubicBezierShape, String, usize)> = Vec::with_capacity(self.graph_nodes.len() * 2);
        for (node_id, node) in self.graph_nodes.iter() {
            for (input_index, input) in node.inputs.iter().enumerate() {
                if let Some((output_node_id, output_connection_index)) = &input.connection {
                    if let Some(input_graph_node) = &self.graph_nodes.get(node_id) {
                        if let Some(output_graph_node) = &self.graph_nodes.get(output_node_id) {
                            let input_node_rect =
                                input_graph_node.get_rect(self.position, self.zoom);
                            let output_node_rect =
                                output_graph_node.get_rect(self.position, self.zoom);

                            let curve = self.draw_connection_line(
                                ui,
                                output_graph_node.get_output_position(
                                    *output_connection_index,
                                    output_node_rect,
                                    self.zoom,
                                ),
                                input_graph_node.get_input_position(input_index, input_node_rect, self.zoom),
                                theme,
                                input.is_error,
                            );

                            connection_curves.push((curve, node_id.clone(), input_index));
                        }
                    }
                }
            }
        }

        for (graph_node_id, graph_node) in self.graph_nodes.iter_mut() {
            puffin::profile_scope!("graph panel.graph_nodes.iter()");

            // are we editing node
            let mut is_editing = false;
            if let Some(n) = editing_node_id {
                if n == graph_node_id {
                    is_editing = true;
                }
            }

            // are we viewing node
            let mut is_viewing: Option<usize> = None;
            if let Some((viewing_node_id, viewing_output_index)) = viewing_node_id_index {
                if viewing_node_id == graph_node_id {
                    is_viewing = Some(*viewing_output_index);
                }
            }

            // draw node
            let graph_node_response = graph_node.show(
                ui,
                self.position,
                self.zoom,
                cursor_position,
                is_editing,
                is_viewing,
                self.temp_connection.clone(),
                theme,
            );

            // node moved
            if let Some(new_position) = graph_node_response.new_position {
                graph_editor_response.new_node_position =
                    Some((graph_node_id.clone(), new_position));
            }

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

            if let Some(out_index) = graph_node_response.view_node {
                graph_editor_response.viewing_node_id_index = Some((graph_node_id.clone(), out_index));
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
                    graph_editor_response
                        .nodes_to_delete
                        .push(graph_node_id.clone());
                } else {
                    graph_editor_response.editing_node_id = Some(graph_node_id.clone());
                }
            }

            // right click on node
            // view node
            if graph_node_response.is_right_click {
                graph_editor_response.viewing_node_id_index = Some((graph_node_id.clone(), 0));
            }
        }

        // ------------------------
        // find if it stopped on a connection
        if has_stopped_creating_connection {
            if let Some(temp_connection) = &self.temp_connection {
                // find node with connection at this position
                for (_, other_graph_node) in self.graph_nodes.iter() {
                    let other_node = &self.graph_nodes[&other_graph_node.id];
                    let other_node_rect = other_graph_node.get_rect(self.position, self.zoom);

                    match temp_connection.from_connection_type {
                        ConnectionType::Input => {
                            for output_index in 0..other_node.outputs.len() {
                                if other_graph_node
                                    .get_output_rect(output_index, other_node_rect, self.zoom)
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
                                    .get_input_rect(input_index, other_node_rect, self.zoom)
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
                    self.draw_connection_line(ui, cursor_position, temp_connection.from_position, theme, false)
                }
                ConnectionType::Output => {
                    self.draw_connection_line(ui, temp_connection.from_position, cursor_position, theme, false)
                }
            };
        }

        // ------------------------
        // mouse

        if editor_bg_response.clicked_by(egui::PointerButton::Primary) && !is_cursor_over_node {
            graph_editor_response.clear_editing_node = true;
        } else if editor_bg_response.clicked_by(egui::PointerButton::Secondary)
            && !is_cursor_over_node
        {
            graph_editor_response.clear_viewing_node = true;
        } else if editor_bg_response.drag_started_by(egui::PointerButton::Primary)
            && !is_cursor_over_node
        {
            self.start_dragging();
        } else if editor_bg_response.drag_stopped_by(egui::PointerButton::Primary) {
            self.stop_dragging();
        }

        // if cursor_primary_went_down && !is_cursor_over_node {
        //     self.start_dragging();
        // }

        if self.last_node_click.is_some() {
            graph_editor_response.request_redraw = true;
        }

        // deleting connections
        ui.input(|i| {
            if i.modifiers.command && cursor_primary_went_down {
                for (curve, input_node_id, input_index) in connection_curves.iter() {
                    if curve.visual_bounding_rect().contains(cursor_position) {
                        let distance =
                            distance_to_cubic_bezier_curve(cursor_position, curve.points);
                        if distance < 15.0 {
                            graph_editor_response
                                .connections_to_delete
                                .push((input_node_id.clone(), *input_index));
                        }
                    }
                }
            }
        });

        //self.draw_top_border(ui, editor_rect, theme);

        self.previous_cursor_primary_down = Some(cursor_primary_down);

        graph_editor_response
    }

    // pub fn draw_top_border(&self, ui: &mut egui::Ui, rect: Rect, theme: &Theme) {
    //     let size = 2.0;
    //     let stroke = Stroke::new(size, egui::Color32::from(theme.panel_border_lines));

    //     let points: Vec<Pos2> = vec![
    //         Pos2::new(rect.left(), rect.top() + (size * 0.5)),
    //         Pos2::new(rect.right(), rect.top() + (size * 0.5)),
    //     ];

    //     ui.painter().add(egui::Shape::line(points, stroke));
    // }

    pub fn draw_background_grid(&self, ui: &mut egui::Ui, editor_rect: Rect, graph_position: Pos2, theme: &Theme) {
        let stroke = Stroke::new(1.0, theme.get().grid_lines);
        let grid_size: f32 = 50.0;

        let mut x = graph_to_view_space(self.zoom, graph_position.x % grid_size);
        let mut y = graph_to_view_space(self.zoom, graph_position.y % grid_size);

        while x <= editor_rect.max.x {
            ui.painter().line_segment(
                [Pos2::new(x, editor_rect.min.y), Pos2::new(x, editor_rect.max.y)],
                stroke,
            );
            x += graph_to_view_space(self.zoom, grid_size);
        }

        while y <= editor_rect.max.y {
            ui.painter().line_segment(
                [Pos2::new(editor_rect.min.x, y), Pos2::new(editor_rect.max.x, y)],
                stroke,
            );
            y += graph_to_view_space(self.zoom, grid_size);
        }
    }

    // returns curve shape to detect clickin on curve
    pub fn draw_connection_line(
        &self,
        ui: &mut egui::Ui,
        from: Pos2,
        to: Pos2,
        theme: &Theme,
        is_error: bool,
    ) -> CubicBezierShape {
        let offset_max = 150.0;

        let mut color = egui::Color32::from(theme.get().grid_connection_line);

        if is_error {
            color = egui::Color32::from(theme.get().grid_connection_dot_error);
        }

        let stroke = egui::Stroke::new(theme.get().grid_connection_line_width, color);

        let distance = from.distance(to);
        let offset = (distance / 2.0).min(offset_max);

        let points = [
            from,
            Pos2::new(from.x + offset, from.y),
            Pos2::new(to.x - offset, to.y),
            to,
        ];

        let curve_shape = CubicBezierShape {
            points,
            closed: false,
            fill: egui::Color32::from_black_alpha(0),
            stroke: stroke.into(),
        };

        ui.painter().add(egui::Shape::CubicBezier(curve_shape.clone()));

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
        settings: NodeSettings,
        inputs: Vec<Input>,
        outputs: Vec<Output>,
        position_graph_space: Pos2,
        is_subgraph: bool,
    ) {
        //let inverse_zoom = 1.0 / self.zoom;
        //let position = Pos2::new(position_graph_space.x, position_graph_space.y);

        let node = GraphNode::new(
            node_id.clone(),
            position_graph_space,
            settings,
            inputs,
            outputs,
            is_subgraph,
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
    pub from_value_type: ValueType,
    /// Whether the source input accepts any type (for pass-through nodes like select).
    pub from_accepts_any_type: bool,
}

pub struct GraphEditorResponse {
    pub new_connection: Option<NewConnection>,
    #[allow(dead_code)]
    pub is_left_click_node_id: Option<String>, // if a node was clicked on return it's id
    #[allow(dead_code)]
    pub is_right_click_node_id: Option<String>,
    pub request_redraw: bool,
    pub editing_node_id: Option<String>,
    pub viewing_node_id_index: Option<(String, usize)>,   // node id, output index
    pub clear_editing_node: bool,
    pub clear_viewing_node: bool,
    pub nodes_to_delete: Vec<String>,
    pub connections_to_delete: Vec<(String, usize)>, // node id, input index
    pub new_node_position: Option<(String, Pos2)>,
}

impl GraphEditorResponse {
    fn default() -> GraphEditorResponse {
        GraphEditorResponse {
            new_connection: None,
            is_left_click_node_id: None,
            is_right_click_node_id: None,
            request_redraw: false,
            editing_node_id: None,
            viewing_node_id_index: None,
            nodes_to_delete: Vec::new(),
            connections_to_delete: Vec::new(),
            clear_editing_node: false, // should editing node be cleared.  clicked on graph bg
            clear_viewing_node: false,
            new_node_position: None,
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
        let step = 0.1;
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

        let x = uuu * points[0].x
            + 3.0 * uu * t * points[1].x
            + 3.0 * u * tt * points[2].x
            + ttt * points[3].x;

        let y = uuu * points[0].y
            + 3.0 * uu * t * points[1].y
            + 3.0 * u * tt * points[2].y
            + ttt * points[3].y;

        Pos2 { x, y }
    }
}
