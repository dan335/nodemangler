use crate::graph::graph_input::draw_graph_input;
use crate::graph::graph_node_header::show_graph_node_header;
use crate::graph::graph_node_info::show_graph_node_info;
use crate::graph::graph_output::draw_graph_output;
use crate::themes::theme::Theme;
use crate::{graph_to_view_space, graph_to_view_space_pos2, view_to_graph_space_pos2, NODE_SIZE};
use eframe::egui;
use egui::{Pos2, Rect, Vec2};
use mangler_core::input::Input;
use mangler_core::node_settings::NodeSettings;
use mangler_core::output::Output;
use mangler_core::AddNodeType;
use std::collections::HashMap;
use std::fmt::Debug;
use std::path::PathBuf;
use std::time::Duration;

use super::graph_editor::TempConnection;
use super::graph_node_thumbnail::GraphNodeThumbnail;
use super::graph_output::draw_graph_output_highlighted;

/// Cached 256-bin histogram for an image output.
/// Stores luminance and per-channel (R, G, B) distributions.
/// Keyed by output index on the node; recomputed when the image's change_id differs.
#[derive(Clone)]
pub struct HistogramCache {
    /// 256 bins representing the luminance distribution.
    pub bins: [u32; 256],
    /// 256 bins for the red channel.
    pub bins_r: [u32; 256],
    /// 256 bins for the green channel.
    pub bins_g: [u32; 256],
    /// 256 bins for the blue channel.
    pub bins_b: [u32; 256],
    /// The maximum bin count across all histograms (for shared vertical scale).
    pub max_count: u32,
    /// Number of channels in the source image (determines RGB vs grayscale display).
    pub channels: u32,
    /// The change_id of the image this histogram was computed from.
    pub image_change_id: String,
}

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
    /// For subgraph nodes, the path to the child `.mangler.json` file. `None`
    /// for operation nodes or subgraph nodes that haven't been loaded yet.
    pub subgraph_path: Option<PathBuf>,
    pub is_busy: bool,
    pub is_error: bool,
    pub error_message: Option<String>,
    pub is_enabled: bool,
    /// Optional user-defined display name for this node.
    pub custom_name: Option<String>,
    /// Whether this node has pending input changes requiring a manual run.
    pub is_dirty: bool,
    /// The node type used to create this node (for copy/paste).
    pub node_type: Option<AddNodeType>,
    /// Cached histograms for image outputs, keyed by output index.
    /// Recomputed automatically when the image's change_id differs from the cached one.
    pub histogram_cache: HashMap<usize, HistogramCache>,
}

impl GraphNode {
    pub fn new(
        id: String,
        position: Pos2,
        settings: NodeSettings,
        inputs: Vec<Input>,
        outputs: Vec<Output>,
        is_subgraph: bool,
        node_type: Option<AddNodeType>,
        is_enabled: bool,
        custom_name: Option<String>,
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
            subgraph_path: None,
            is_busy: false,
            is_error: false,
            is_dirty: false,
            error_message: None,
            is_enabled,
            custom_name,
            node_type,
            histogram_cache: HashMap::new(),
        }
    }

    pub fn get_rect(&self, graph_position: Pos2, graph_zoom: f32) -> Rect {
        let node_view_pos = graph_to_view_space_pos2(graph_zoom, self.position);
        let graph_view_pos = graph_to_view_space_pos2(graph_zoom, graph_position);

        let graph_pos = Pos2::new(
            graph_view_pos.x + node_view_pos.x,
            graph_view_pos.y + node_view_pos.y,
        );
        // Add extra height for the operation type row when a custom name is set.
        let node_size = if self.custom_name.is_some() {
            Vec2::new(NODE_SIZE.x, NODE_SIZE.y + 12.0)
        } else {
            NODE_SIZE
        };
        let view_size = graph_to_view_space_pos2(graph_zoom, node_size.to_pos2());
        Rect::from_center_size(graph_pos, view_size.to_vec2())
    }

    /// The node's full bounding rect in **graph space** (camera-independent),
    /// including its thumbnail block if present. Used to frame the camera on a
    /// set of nodes ("F" to focus). Mirrors the header-size rule in `get_rect`.
    pub fn graph_space_rect(&self) -> Rect {
        // Header box (matches the size rule used by `get_rect`).
        let node_size = if self.custom_name.is_some() {
            Vec2::new(NODE_SIZE.x, NODE_SIZE.y + 12.0)
        } else {
            NODE_SIZE
        };
        let header_rect = Rect::from_center_size(self.position, node_size);

        // Thumbnails hang below the header's bottom edge, centered horizontally
        // on the node (see `show` / `GraphNodeThumbnail::show`). Union them in so
        // framing doesn't clip previews.
        match &self.thumbnail {
            Some(thumbnail) => {
                let thumb_size = thumbnail.graph_space_size();
                let thumb_rect = Rect::from_center_size(
                    Pos2::new(
                        self.position.x,
                        header_rect.bottom() + thumb_size.y / 2.0,
                    ),
                    thumb_size,
                );
                header_rect.union(thumb_rect)
            },
            None => header_rect,
        }
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

        // Only the viewport that actually has the pointer may move the node:
        // the same node can render in several panels/windows per frame, and a
        // window without the pointer sees a far-offscreen fallback cursor that
        // would otherwise yank the node away.
        if self.is_dragging && ui.ctx().pointer_latest_pos().is_some() {
            if let Some(last_drag_position) = self.last_drag_position {
                let delta = view_to_graph_space_pos2(
                    graph_zoom,
                    panel_cursor_position - last_drag_position.to_vec2(),
                )
                .to_vec2();
                self.position += delta;
                graph_node_response.drag_delta = Some(delta);
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
            self.custom_name.as_deref(),
            node_rect,
            is_editing,
            self.is_subgraph,
            graph_zoom,
            theme,
            self.is_busy,
            self.is_enabled,
            self.is_dirty,
        );

        show_graph_node_info(ui, self.time, node_rect, graph_zoom, theme);

        if let Some(thumbnail) = &self.thumbnail {
            thumbnail.show(
                ui,
                self.get_rect(graph_position, graph_zoom).center_bottom(),
                graph_zoom,
                theme,
            );
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
                self.get_input_release_rect(index, node_rect, graph_zoom),
                index,
                node_rect,
                ui,
                bg_response.hovered(),
                temp_connection.as_ref(),
                theme,
                graph_zoom,
                panel_cursor_position,
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
                self.get_output_release_rect(index, node_rect, graph_zoom),
                index,
                node_rect,
                ui,
                bg_response.hovered(),
                temp_connection.as_ref(),
                theme,
                graph_zoom,
                panel_cursor_position,
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
                    draw_graph_output_highlighted(
                        self.get_output_position(index, node_rect, graph_zoom),
                        ui,
                        theme,
                        graph_zoom,
                    );
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
            node_rect.top()
                + graph_to_view_space(graph_zoom, 12.0)
                + graph_to_view_space(graph_zoom, 20.0) * index as f32,
        )
    }

    pub fn get_output_position(&self, index: usize, node_rect: Rect, graph_zoom: f32) -> Pos2 {
        Pos2::new(
            node_rect.right() + graph_to_view_space(graph_zoom, 14.0),
            node_rect.top()
                + graph_to_view_space(graph_zoom, 12.0)
                + graph_to_view_space(graph_zoom, 20.0) * index as f32,
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

    /// Forgiving drop zone for an input dot. Reaches out to the *left* of the
    /// dot (away from the node) so a connection can be released anywhere in the
    /// gutter to the input's left and still land here — the dots stay small but
    /// the target you can release on is much larger. Vertically it spans half
    /// the row spacing on each side so adjacent inputs tile with no gaps or
    /// overlap, and it is zoom-scaled so the zone tracks the on-screen layout.
    pub fn get_input_release_rect(&self, index: usize, node_rect: Rect, graph_zoom: f32) -> Rect {
        let pos = self.get_input_position(index, node_rect, graph_zoom);
        let half_row = graph_to_view_space(graph_zoom, 10.0);
        // How far left of the dot the zone reaches, and how far it extends back
        // toward the node (the dot sits 14px left of the node edge, so 14 here
        // lines the inner edge up with the node's left side).
        let reach = graph_to_view_space(graph_zoom, 45.0);
        let inward = graph_to_view_space(graph_zoom, 14.0);
        Rect::from_min_max(
            Pos2::new(pos.x - reach, pos.y - half_row),
            Pos2::new(pos.x + inward, pos.y + half_row),
        )
    }

    /// Forgiving drop zone for an output dot — mirror of
    /// [`get_input_release_rect`], reaching out to the *right* of the dot.
    pub fn get_output_release_rect(&self, index: usize, node_rect: Rect, graph_zoom: f32) -> Rect {
        let pos = self.get_output_position(index, node_rect, graph_zoom);
        let half_row = graph_to_view_space(graph_zoom, 10.0);
        let reach = graph_to_view_space(graph_zoom, 45.0);
        let inward = graph_to_view_space(graph_zoom, 14.0);
        Rect::from_min_max(
            Pos2::new(pos.x - inward, pos.y - half_row),
            Pos2::new(pos.x + reach, pos.y + half_row),
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
    pub view_node: Option<usize>, // usize = output index to view
    pub is_right_click: bool,
    pub is_left_click: bool,
    pub is_cursor_inside: bool,
    /// The movement delta in graph space applied this frame (for multi-node drag).
    pub drag_delta: Option<Vec2>,
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
            drag_delta: None,
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
