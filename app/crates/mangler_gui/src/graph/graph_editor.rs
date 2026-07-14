use super::graph_node::ConnectionType;
use crate::{
    graph::graph_node::GraphNode, graph_to_view_space,
    pan_zoom::{self, PanZoomController},
    program::NewConnection, themes::theme::Theme, view_to_graph_space,
};
use eframe::{
    egui,
    epaint::{CornerRadius, Rect, Stroke},
};
use egui::epaint::CubicBezierShape;
use egui::Pos2;
use mangler_core::{
    input::Input, node_settings::NodeSettings, operations::Operation, output::Output, value::ValueType,
    AddNodeType,
};
use std::collections::{HashMap, HashSet};

pub struct GraphEditor {
    pub graph_nodes: HashMap<String, GraphNode>,
    temp_connection: Option<TempConnection>,

    /// Set of currently selected node IDs (for multi-selection and copy/paste).
    pub selected_node_ids: HashSet<String>,
}

/// Per-panel view transform for a graph panel: every Graph-kind panel leaf
/// owns one, so panels pan/zoom independently while sharing the graph itself.
pub struct GraphCamera {
    /// Pan offset in graph space (view = (graph + position) / zoom).
    pub position: Pos2,
    pub zoom: f32,
    /// Drag-to-pan state machine (shared implementation with the 2D preview).
    pub pan_zoom: PanZoomController,
    /// When true, center the view on graph origin (0,0) on the next frame.
    pub needs_center: bool,
}

impl GraphCamera {
    pub fn new() -> GraphCamera {
        GraphCamera {
            position: Pos2::ZERO,
            zoom: 1.0,
            pan_zoom: PanZoomController::new(),
            needs_center: true,
        }
    }
}

impl GraphEditor {
    pub fn new() -> GraphEditor {
        GraphEditor {
            graph_nodes: HashMap::default(),
            temp_connection: None,
            selected_node_ids: HashSet::new(),
        }
    }

    /// Wipe all editor state: nodes, selection, any in-progress (drag)
    /// connection, and click tracking. Used when the engine replaces the
    /// graph wholesale (`GraphChangedMessage::GraphCleared`, e.g. resolving
    /// a file conflict by reloading from disk) — the fresh `LoadedNode`
    /// stream that follows assumes a clean slate.
    pub fn clear(&mut self) {
        self.graph_nodes.clear();
        self.temp_connection = None;
        self.selected_node_ids.clear();
    }

    /// Frames the camera on a set of nodes: the current selection if any, else
    /// the whole graph. No-op on an empty graph. Instant snap (no animation),
    /// matching the 2D/3D viewers. Camera is pure GUI state, so nothing is sent
    /// to the engine.
    fn focus_camera(&self, camera: &mut GraphCamera, editor_rect: Rect) {
        // Choose targets: selected nodes if any, otherwise every node.
        let target_ids: Vec<&String> = if self.selected_node_ids.is_empty() {
            self.graph_nodes.keys().collect()
        } else {
            self.selected_node_ids.iter().collect()
        };

        // Union each target's graph-space rect into one bounding box.
        let mut bbox: Option<Rect> = None;
        for id in target_ids {
            if let Some(node) = self.graph_nodes.get(id) {
                let node_rect = node.graph_space_rect();
                bbox = Some(match bbox {
                    Some(b) => b.union(node_rect),
                    None => node_rect,
                });
            }
        }
        let bbox = match bbox {
            Some(b) => b,
            None => return, // empty graph / nothing to frame
        };

        // Guard against a degenerate panel size (avoids NaN in the ratios).
        if editor_rect.width() <= 0.0 || editor_rect.height() <= 0.0 {
            return;
        }

        // Fit zoom: larger zoom = smaller on screen (screen = graph / zoom), so
        // pick the axis where content is most oversized, add margin for padding,
        // clamp to the shared bounds, and cap at 1.0 so a tiny selection never
        // zooms in past 100%.
        let margin = 1.15;
        let zoom = ((bbox.width() / editor_rect.width())
            .max(bbox.height() / editor_rect.height())
            * margin)
            .clamp(pan_zoom::ZOOM_BOUNDS[0], pan_zoom::ZOOM_BOUNDS[1])
            .max(1.0);

        // Pan so the bbox center lands on the panel center. Solving
        // (bbox.center + position) / zoom = editor_rect.center for position.
        let p = editor_rect.center();
        let c = bbox.center();
        camera.zoom = zoom;
        camera.position = Pos2::new(p.x * zoom - c.x, p.y * zoom - c.y);
        camera.needs_center = false;
    }

    pub fn show(
        &mut self,
        ui: &mut egui::Ui,
        camera: &mut GraphCamera,
        editing_node_id: &Option<String>,
        viewing_node_id_index: &Option<(String, usize)>,
        theme: &Theme,
        is_popup_open: bool,
    ) -> GraphEditorResponse {
        puffin::profile_scope!("graph panel.show()");
        let mut graph_editor_response = GraphEditorResponse::default();

        let editor_rect = ui.max_rect();

        // Pointer state from this ui's own (per-viewport) input, so graph
        // panels hosted in secondary OS windows track their window's pointer
        // rather than the main window's.
        let cursor_position = pan_zoom::viewport_cursor(ui);
        let cursor_primary_down: bool = ui.ctx().input(|i| i.pointer.primary_down());

        // Center the view on graph origin (0,0) on the first frame.
        if camera.needs_center {
            let center = editor_rect.center();
            camera.position = Pos2::new(
                view_to_graph_space(camera.zoom, center.x),
                view_to_graph_space(camera.zoom, center.y),
            );
            camera.needs_center = false;
        }

        let editor_bg_response =
            ui.allocate_rect(editor_rect, egui::Sense::click().union(egui::Sense::drag()).union(egui::Sense::hover()));
        //let panel_cursor_position = Pos2::new(cursor_position.x - editor_rect.min.x, cursor_position.y - editor_rect.min.y);

        // Scroll-to-zoom about the cursor (shared with the 2D preview).
        if editor_rect.contains(cursor_position) && !is_popup_open {
            pan_zoom::zoom_about_cursor(
                ui,
                &mut camera.position,
                &mut camera.zoom,
                cursor_position,
                pan_zoom::ZOOM_BOUNDS,
            );
        }

        // "F" to focus the camera on the selected nodes (or the whole graph if
        // nothing is selected), Maya-style. Only the hovered panel reframes, and
        // never while the node search is open or a text field has keyboard focus.
        // `text_edit_focused()` (not `egui_wants_keyboard_input()`, which is true
        // whenever any widget has focus, including a selected node) so reframing
        // still works with a node selected.
        let typing = ui.ctx().text_edit_focused();
        if editor_rect.contains(cursor_position)
            && !is_popup_open
            && !typing
            && ui.ctx().input(|i| i.key_pressed(egui::Key::F))
        {
            self.focus_camera(camera, editor_rect);
        }


        ui.set_clip_rect(editor_rect);

        // bg
        ui.painter().add(egui::Shape::rect_filled(
            editor_rect,
            CornerRadius::ZERO,
            theme.get().grid_bg,
        ));

        self.draw_background_grid(ui, editor_rect, camera.position, camera.zoom, theme);

        let cursor_inside = editor_rect.contains(cursor_position);

        let mut is_cursor_over_node = false;

        // Drag-to-pan (shared state machine with the 2D preview); starts via
        // the background response further down, once node hits are known.
        let pointer_events = camera.pan_zoom.update(
            &mut camera.position,
            camera.zoom,
            cursor_position,
            cursor_inside,
            cursor_primary_down,
        );
        let cursor_primary_went_down = pointer_events.primary_went_down;

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
        // If a selected node is dragged, we apply the same delta to all other selected nodes after the loop.
        let mut multi_drag: Option<(String, egui::Vec2)> = None;
        //let mut connection_to_position = Pos2::ZERO;

        // connections
        // collect curves to tell if we clicked on one to delete it
        // curve, input node id, input index
        let mut connection_curves: Vec<(CubicBezierShape, String, usize)> = Vec::with_capacity(self.graph_nodes.len() * 2);
        for (node_id, node) in self.graph_nodes.iter() {
            for (input_index, input) in node.inputs.iter().enumerate() {
                if let Some((output_node_id, output_connection_index)) = &input.connection {
                    // `node` (from the outer iter()) already *is*
                    // `self.graph_nodes.get(node_id)` — no need to look it up
                    // again under a different name.
                    if let Some(output_graph_node) = &self.graph_nodes.get(output_node_id) {
                        let input_node_rect =
                            node.get_rect(camera.position, camera.zoom);
                        let output_node_rect =
                            output_graph_node.get_rect(camera.position, camera.zoom);

                        let curve = self.draw_connection_line(
                            ui,
                            output_graph_node.get_output_position(
                                *output_connection_index,
                                output_node_rect,
                                camera.zoom,
                            ),
                            node.get_input_position(input_index, input_node_rect, camera.zoom),
                            theme,
                            input.is_error,
                        );

                        connection_curves.push((curve, node_id.clone(), input_index));
                    }
                }
            }
        }

        for (graph_node_id, graph_node) in self.graph_nodes.iter_mut() {
            puffin::profile_scope!("graph panel.graph_nodes.iter()");

            // are we editing or selected
            let is_editing = editing_node_id.as_ref() == Some(graph_node_id);
            let is_selected = self.selected_node_ids.contains(graph_node_id.as_str());

            // are we viewing node
            let mut is_viewing: Option<usize> = None;
            if let Some((viewing_node_id, viewing_output_index)) = viewing_node_id_index {
                if viewing_node_id == graph_node_id {
                    is_viewing = Some(*viewing_output_index);
                }
            }

            // draw node (highlight if editing or selected)
            let graph_node_response = graph_node.show(
                ui,
                camera.position,
                camera.zoom,
                cursor_position,
                is_editing || is_selected,
                is_viewing,
                self.temp_connection.clone(),
                theme,
            );

            // node moved — record delta to apply to other selected nodes after the loop
            if let Some(delta) = graph_node_response.drag_delta {
                graph_editor_response.new_node_positions.push((graph_node_id.clone(), graph_node.position));
                if self.selected_node_ids.contains(graph_node_id.as_str()) {
                    multi_drag = Some((graph_node_id.clone(), delta));
                }
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
                let (is_command_down, is_shift_down) = ui.input(|i| {
                    (i.modifiers.command, i.modifiers.shift)
                });

                if is_command_down {
                    // delete node
                    graph_editor_response
                        .nodes_to_delete
                        .push(graph_node_id.clone());
                } else if is_shift_down {
                    // shift+click: toggle node in multi-selection
                    if self.selected_node_ids.contains(graph_node_id.as_str()) {
                        self.selected_node_ids.remove(graph_node_id.as_str());
                    } else {
                        self.selected_node_ids.insert(graph_node_id.clone());
                    }

                } else {
                    // plain click: select only this node, edit it
                    self.selected_node_ids.clear();
                    self.selected_node_ids.insert(graph_node_id.clone());
                    graph_editor_response.editing_node_id = Some(graph_node_id.clone());

                }
            }

            // right click on node
            // view node — unless it's a material export node, in which case the
            // right-click instead binds all of its input connections onto the 3D
            // preview panels' material channels (see view_material_node below).
            if graph_node_response.is_right_click {
                // node_type can be None on some load paths (e.g. subgraph inner
                // nodes reconstructed without their AddNodeType), so also fall
                // back to matching on the node's display name.
                let is_material_node = matches!(
                    &graph_node.node_type,
                    Some(AddNodeType::Operation(Operation::OpImageOutputMaterial))
                ) || graph_node.settings.name == "material";

                if is_material_node {
                    graph_editor_response.view_material_node = Some(graph_node_id.clone());
                } else {
                    graph_editor_response.viewing_node_id_index = Some((graph_node_id.clone(), 0));
                }
            }
        }

        // Apply drag delta to all other selected nodes (multi-node drag).
        if let Some((dragged_node_id, delta)) = multi_drag {
            let moved = self.apply_multi_drag(&dragged_node_id, delta);
            graph_editor_response.new_node_positions.extend(moved);
        }

        // ------------------------
        // find if it stopped on a connection
        if has_stopped_creating_connection {
            if let Some(temp_connection) = &self.temp_connection {
                // Find the dot to connect to. The release zones are large and
                // asymmetric (they reach out into the gutter beside the node),
                // so more than one can contain the cursor at once — pick the one
                // whose dot is nearest the cursor. Only valid targets (different
                // node, compatible type) are considered, so the roomy zones can
                // never land a connection the dot coloring says is illegal.
                let mut best: Option<(f32, NewConnection)> = None;
                for (_, other_graph_node) in self.graph_nodes.iter() {
                    // A node can never connect to itself.
                    if other_graph_node.id == temp_connection.from_node_id {
                        continue;
                    }
                    let other_node_rect = other_graph_node.get_rect(camera.position, camera.zoom);

                    match temp_connection.from_connection_type {
                        // Dragging from an input → looking for a compatible output.
                        ConnectionType::Input => {
                            for (output_index, output) in other_graph_node.outputs.iter().enumerate() {
                                let compatible = temp_connection.from_accepts_any_type
                                    || temp_connection
                                        .from_value_type
                                        .valid_conversions()
                                        .contains(&output.value.value_type());
                                if !compatible {
                                    continue;
                                }
                                let dot = other_graph_node
                                    .get_output_position(output_index, other_node_rect, camera.zoom);
                                if other_graph_node
                                    .get_output_release_rect(output_index, other_node_rect, camera.zoom)
                                    .contains(cursor_position)
                                {
                                    let dist = dot.distance_sq(cursor_position);
                                    if best.as_ref().map_or(true, |(d, _)| dist < *d) {
                                        best = Some((dist, NewConnection::new(
                                            temp_connection.from_node_id.clone(),
                                            temp_connection.from_connection_index,
                                            other_graph_node.id.clone(),
                                            output_index,
                                        )));
                                    }
                                }
                            }
                        }
                        // Dragging from an output → looking for a compatible input.
                        ConnectionType::Output => {
                            for (input_index, input) in other_graph_node.inputs.iter().enumerate() {
                                // Hidden inputs draw no dot, so they can't be a drop target.
                                if input.hide_in_graph {
                                    continue;
                                }
                                let compatible = input.accepts_any_type
                                    || temp_connection
                                        .from_value_type
                                        .valid_conversions()
                                        .contains(&input.value.value_type());
                                if !compatible {
                                    continue;
                                }
                                let dot = other_graph_node
                                    .get_input_position(input_index, other_node_rect, camera.zoom);
                                if other_graph_node
                                    .get_input_release_rect(input_index, other_node_rect, camera.zoom)
                                    .contains(cursor_position)
                                {
                                    let dist = dot.distance_sq(cursor_position);
                                    if best.as_ref().map_or(true, |(d, _)| dist < *d) {
                                        best = Some((dist, NewConnection {
                                            input_node_id: other_graph_node.id.clone(),
                                            input_connection_index: input_index,
                                            output_node_id: temp_connection.from_node_id.clone(),
                                            output_connection_index: temp_connection
                                                .from_connection_index,
                                        }));
                                    }
                                }
                            }
                        }
                    }
                }

                if let Some((_, connection)) = best {
                    graph_editor_response.new_connection = Some(connection);
                }
            }
        }

        // Only the viewport holding the pointer may decide the connection was
        // dropped: a window without the pointer sees `primary_down == false`
        // for a drag happening in another window and would drop it (and open
        // the search popup) spuriously.
        let viewport_has_pointer = ui.ctx().pointer_latest_pos().is_some();
        if self.temp_connection.is_some() && !cursor_primary_down && viewport_has_pointer {
            // If no new connection was made, signal that the connection was dropped
            if graph_editor_response.new_connection.is_none() {
                graph_editor_response.dropped_connection = self.temp_connection.clone();
            }
            self.temp_connection = None;
        }

        // temp connection being created — drawn only in the window the cursor
        // is in (other windows would draw a line to their offscreen fallback
        // cursor).
        if let Some(temp_connection) = self.temp_connection.as_ref().filter(|_| viewport_has_pointer) {
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
            self.selected_node_ids.clear();
        } else if editor_bg_response.clicked_by(egui::PointerButton::Secondary)
            && !is_cursor_over_node
        {
            graph_editor_response.clear_viewing_node = true;
        } else if editor_bg_response.drag_started_by(egui::PointerButton::Primary)
            && !is_cursor_over_node
        {
            camera.pan_zoom.start_dragging();
        } else if editor_bg_response.drag_stopped_by(egui::PointerButton::Primary) {
            camera.pan_zoom.stop_dragging();
        }

        // if cursor_primary_went_down && !is_cursor_over_node {
        //     self.start_dragging();
        // }

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

    pub fn draw_background_grid(&self, ui: &mut egui::Ui, editor_rect: Rect, graph_position: Pos2, zoom: f32, theme: &Theme) {
        let stroke = Stroke::new(1.0, theme.get().grid_lines);
        let grid_size: f32 = 50.0;

        let mut x = graph_to_view_space(zoom, graph_position.x % grid_size);
        let mut y = graph_to_view_space(zoom, graph_position.y % grid_size);

        while x <= editor_rect.max.x {
            ui.painter().line_segment(
                [Pos2::new(x, editor_rect.min.y), Pos2::new(x, editor_rect.max.y)],
                stroke,
            );
            x += graph_to_view_space(zoom, grid_size);
        }

        while y <= editor_rect.max.y {
            ui.painter().line_segment(
                [Pos2::new(editor_rect.min.x, y), Pos2::new(editor_rect.max.x, y)],
                stroke,
            );
            y += graph_to_view_space(zoom, grid_size);
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

    /// Apply a drag delta to all selected nodes except the one already dragged.
    /// Returns (node_id, new_position) for each node that was moved.
    pub fn apply_multi_drag(&mut self, dragged_node_id: &str, delta: egui::Vec2) -> Vec<(String, Pos2)> {
        let mut moved = Vec::new();
        for other_id in self.selected_node_ids.iter() {
            if other_id.as_str() != dragged_node_id {
                if let Some(other_node) = self.graph_nodes.get_mut(other_id) {
                    other_node.position += delta;
                    moved.push((other_id.clone(), other_node.position));
                }
            }
        }
        moved
    }

    pub fn add_node(
        &mut self,
        node_id: String,
        settings: NodeSettings,
        inputs: Vec<Input>,
        outputs: Vec<Output>,
        position_graph_space: Pos2,
        is_subgraph: bool,
        node_type: Option<AddNodeType>,
        is_enabled: bool,
        custom_name: Option<String>,
    ) {
        let node = GraphNode::new(
            node_id.clone(),
            position_graph_space,
            settings,
            inputs,
            outputs,
            is_subgraph,
            node_type,
            is_enabled,
            custom_name,
        );

        self.graph_nodes.insert(node_id, node);
    }

    pub fn remove_node(&mut self, node_id: &String) {
        self.graph_nodes.remove(node_id);
    }

    /// Automatically layout nodes if they all share the same position (e.g. all at origin).
    ///
    /// Checks whether all nodes overlap and, if so, runs the auto-arrange layout.
    /// Returns a list of (node_id, new_position) pairs for nodes that were moved.
    pub fn auto_layout_if_needed(&mut self) -> Vec<(String, Pos2)> {
        if self.graph_nodes.len() < 2 {
            return Vec::new();
        }

        // Check if all nodes are at the same position (or very close).
        let positions: Vec<Pos2> = self.graph_nodes.values().map(|n| n.position).collect();
        let first = positions[0];
        let all_overlapping = positions.iter().all(|p| {
            (p.x - first.x).abs() < 1.0 && (p.y - first.y).abs() < 1.0
        });

        if !all_overlapping {
            return Vec::new();
        }

        self.auto_arrange()
    }

    /// Arrange all nodes using a connection-aware layout algorithm.
    ///
    /// Uses a topological sort to assign nodes to columns by depth, then applies
    /// a barycenter heuristic to order nodes within each column so that connected
    /// nodes are vertically adjacent, minimizing edge crossings. Each node is
    /// positioned at the average y of its upstream neighbors for clean horizontal
    /// connections. Returns a list of (node_id, new_position) pairs.
    pub fn auto_arrange(&mut self) -> Vec<(String, Pos2)> {
        if self.graph_nodes.len() < 2 {
            return Vec::new();
        }

        // Build adjacency: for each node, find its depth (column) via connections.
        // A node's depth = max(depth of all upstream nodes) + 1.
        // Nodes with no inputs or no connected inputs are depth 0.
        let node_ids: Vec<String> = self.graph_nodes.keys().cloned().collect();

        // Map node_id -> set of upstream node_ids (nodes that feed into this one).
        let mut upstream: HashMap<String, Vec<String>> = HashMap::new();
        for (node_id, node) in self.graph_nodes.iter() {
            let mut ups = Vec::new();
            for input in node.inputs.iter() {
                if let Some((output_node_id, _)) = &input.connection {
                    ups.push(output_node_id.clone());
                }
            }
            upstream.insert(node_id.clone(), ups);
        }

        // Compute depth for each node via iterative relaxation.
        let mut depth: HashMap<String, usize> = HashMap::new();
        for id in node_ids.iter() {
            depth.insert(id.clone(), 0);
        }

        // Relax until stable (handles DAGs of any depth).
        let mut changed = true;
        let max_iterations = node_ids.len() + 1;
        let mut iteration = 0;
        while changed && iteration < max_iterations {
            changed = false;
            iteration += 1;
            for id in node_ids.iter() {
                if let Some(ups) = upstream.get(id) {
                    let max_upstream_depth = ups.iter()
                        .filter_map(|uid| depth.get(uid))
                        .max()
                        .copied()
                        .unwrap_or(0);
                    if !ups.is_empty() {
                        let new_depth = max_upstream_depth + 1;
                        if new_depth > depth[id] {
                            depth.insert(id.clone(), new_depth);
                            changed = true;
                        }
                    }
                }
            }
        }

        // Build downstream map for tightening and barycenter backward sweep.
        let mut downstream: HashMap<String, Vec<String>> = HashMap::new();
        for (node_id, ups) in upstream.iter() {
            for up_id in ups {
                downstream.entry(up_id.clone()).or_default().push(node_id.clone());
            }
        }

        // Tighten: push each node as far right as possible so it sits just
        // before its earliest downstream consumer. This eliminates long-range
        // connections that skip many empty columns (e.g. a source in column 0
        // whose only consumer is in column 6 gets moved to column 5).
        for id in node_ids.iter() {
            if let Some(downs) = downstream.get(id) {
                let min_downstream_depth = downs.iter()
                    .filter_map(|did| depth.get(did))
                    .min()
                    .copied();
                if let Some(min_dd) = min_downstream_depth {
                    if min_dd > 0 {
                        let tight_depth = min_dd - 1;
                        if tight_depth > depth[id] {
                            depth.insert(id.clone(), tight_depth);
                        }
                    }
                }
            }
        }

        // Group nodes by depth (column).
        let max_depth = depth.values().max().copied().unwrap_or(0);
        let mut columns: Vec<Vec<String>> = vec![Vec::new(); max_depth + 1];
        for id in node_ids.iter() {
            let d = depth[id];
            columns[d].push(id.clone());
        }

        // Build node_id -> column index lookup.
        let mut node_column: HashMap<String, usize> = HashMap::new();
        for (col_idx, col) in columns.iter().enumerate() {
            for node_id in col {
                node_column.insert(node_id.clone(), col_idx);
            }
        }

        // Sort column 0 alphabetically as a stable baseline (no upstream info).
        if let Some(col) = columns.get_mut(0) {
            col.sort_by(|a, b| {
                let name_a = self.graph_nodes.get(a).map(|n| &n.settings.name).unwrap();
                let name_b = self.graph_nodes.get(b).map(|n| &n.settings.name).unwrap();
                name_a.cmp(name_b)
            });
        }

        // Barycenter heuristic: order nodes within each column by the average
        // position of their connected neighbors in the adjacent column.
        // Two full passes (forward + backward) minimizes edge crossings.
        for _pass in 0..2 {
            // Forward sweep: columns 1..max, sort by upstream neighbor positions.
            for col_idx in 1..columns.len() {
                let prev_col = columns[col_idx - 1].clone();
                let col = &mut columns[col_idx];
                col.sort_by(|a, b| {
                    let bc_a = barycenter(a, upstream.get(a).map(|v| v.as_slice()).unwrap_or(&[]), &prev_col, col_idx - 1, &node_column);
                    let bc_b = barycenter(b, upstream.get(b).map(|v| v.as_slice()).unwrap_or(&[]), &prev_col, col_idx - 1, &node_column);
                    bc_a.partial_cmp(&bc_b).unwrap_or(std::cmp::Ordering::Equal)
                });
            }

            // Backward sweep: columns (max-1)..0, sort by downstream neighbor positions.
            for col_idx in (0..columns.len().saturating_sub(1)).rev() {
                let next_col = columns[col_idx + 1].clone();
                let col = &mut columns[col_idx];
                col.sort_by(|a, b| {
                    let bc_a = barycenter(a, downstream.get(a).map(|v| v.as_slice()).unwrap_or(&[]), &next_col, col_idx + 1, &node_column);
                    let bc_b = barycenter(b, downstream.get(b).map(|v| v.as_slice()).unwrap_or(&[]), &next_col, col_idx + 1, &node_column);
                    bc_a.partial_cmp(&bc_b).unwrap_or(std::cmp::Ordering::Equal)
                });
            }
        }

        // Assign positions using connection-aware placement.
        // Node visual height is ~210px (40px header + 150px thumbnail + 20px info).
        let h_spacing = 280.0;
        let v_spacing = 280.0;
        let start_x = 100.0;
        let start_y = 100.0;

        // First pass: place column 0 on a regular grid.
        let mut node_positions: HashMap<String, Pos2> = HashMap::new();
        if let Some(col) = columns.first() {
            let col_height = (col.len() as f32 - 1.0) * v_spacing;
            let col_start_y = start_y - col_height * 0.5;
            for (row_index, node_id) in col.iter().enumerate() {
                node_positions.insert(
                    node_id.clone(),
                    Pos2::new(start_x, col_start_y + row_index as f32 * v_spacing),
                );
            }
        }

        // Subsequent columns: position each node at the average y of its upstream
        // neighbors so connections stay as horizontal as possible. Then enforce
        // minimum vertical spacing to prevent overlap.
        for col_index in 1..columns.len() {
            let col = &columns[col_index];
            let x = start_x + col_index as f32 * h_spacing;

            // Compute ideal y for each node based on upstream neighbor positions.
            let mut ideal_ys: Vec<(String, f32)> = Vec::new();
            for node_id in col {
                let ups = upstream.get(node_id).cloned().unwrap_or_default();
                let upstream_ys: Vec<f32> = ups.iter()
                    .filter_map(|uid| node_positions.get(uid).map(|p| p.y))
                    .collect();

                let ideal_y = if upstream_ys.is_empty() {
                    // Orphan in this column — will be placed after connected nodes.
                    f32::MAX
                } else {
                    // Average y of all upstream neighbors.
                    upstream_ys.iter().sum::<f32>() / upstream_ys.len() as f32
                };
                ideal_ys.push((node_id.clone(), ideal_y));
            }

            // Resolve overlaps: walk top to bottom, push nodes down if they're
            // too close to the previous node. Preserves the barycenter ordering.
            for i in 1..ideal_ys.len() {
                // Handle orphans (f32::MAX): place them relative to the last real node.
                if ideal_ys[i].1 == f32::MAX {
                    ideal_ys[i].1 = ideal_ys[i - 1].1 + v_spacing;
                }
                let min_y = ideal_ys[i - 1].1 + v_spacing;
                if ideal_ys[i].1 < min_y {
                    ideal_ys[i].1 = min_y;
                }
            }

            // Center the column around the average y of the previous column,
            // to keep the graph visually centered.
            let prev_col = &columns[col_index - 1];
            let prev_avg_y: f32 = prev_col.iter()
                .filter_map(|id| node_positions.get(id).map(|p| p.y))
                .sum::<f32>() / prev_col.len().max(1) as f32;
            let col_avg_y: f32 = ideal_ys.iter().map(|(_, y)| *y).sum::<f32>()
                / ideal_ys.len().max(1) as f32;
            let y_offset = prev_avg_y - col_avg_y;

            for (node_id, y) in &ideal_ys {
                node_positions.insert(node_id.clone(), Pos2::new(x, y + y_offset));
            }
        }

        // Apply positions to nodes.
        let mut moved: Vec<(String, Pos2)> = Vec::new();
        for col in columns.iter() {
            for node_id in col {
                if let Some(pos) = node_positions.get(node_id) {
                    if let Some(node) = self.graph_nodes.get_mut(node_id) {
                        node.position = *pos;
                        moved.push((node_id.clone(), *pos));
                    }
                }
            }
        }

        moved
    }
}

/// Compute the barycenter (average position index) of a node's neighbors
/// that reside in the specified adjacent column. Returns f32::MAX if no
/// neighbors are found in that column, pushing orphans to the bottom.
fn barycenter(
    _node_id: &str,
    neighbors: &[String],
    adj_col: &[String],
    adj_col_idx: usize,
    node_column: &HashMap<String, usize>,
) -> f32 {
    let mut sum = 0.0f32;
    let mut count = 0u32;
    for neighbor in neighbors {
        // Only consider neighbors actually in the adjacent column.
        if node_column.get(neighbor) == Some(&adj_col_idx) {
            if let Some(pos) = adj_col.iter().position(|id| id == neighbor) {
                sum += pos as f32;
                count += 1;
            }
        }
    }
    if count > 0 { sum / count as f32 } else { f32::MAX }
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
    pub editing_node_id: Option<String>,
    pub viewing_node_id_index: Option<(String, usize)>,   // node id, output index
    pub clear_editing_node: bool,
    pub clear_viewing_node: bool,
    pub nodes_to_delete: Vec<String>,
    pub connections_to_delete: Vec<(String, usize)>, // node id, input index
    /// Positions of all nodes that moved this frame (node_id, new_position).
    pub new_node_positions: Vec<(String, Pos2)>,
    pub dropped_connection: Option<TempConnection>,
    /// Set when the user right-clicks a material export node; carries the
    /// node id so Program can bind the 3D panels' channels from its input
    /// connections.
    pub view_material_node: Option<String>,
}

impl GraphEditorResponse {
    fn default() -> GraphEditorResponse {
        GraphEditorResponse {
            new_connection: None,
            editing_node_id: None,
            viewing_node_id_index: None,
            nodes_to_delete: Vec::new(),
            connections_to_delete: Vec::new(),
            clear_editing_node: false, // should editing node be cleared.  clicked on graph bg
            clear_viewing_node: false,
            new_node_positions: Vec::new(),
            dropped_connection: None,
            view_material_node: None,
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

#[cfg(test)]
#[path = "graph_editor_tests.rs"]
mod tests;
