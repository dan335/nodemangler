use eframe::egui;
use egui::Pos2;

use crate::graph::graph_node::GraphNode;
use crate::WINDOW_HEIGHT;

pub struct GraphEditor {
    position: Pos2,
    is_dragging: bool,
    last_drag_position: Option<Pos2>,
    nodes: Vec<GraphNode>
}

impl GraphEditor {
    pub fn new() -> GraphEditor {
        let mut nodes: Vec<GraphNode> = vec![];

        nodes.push(GraphNode::new(Pos2::new(0.0, WINDOW_HEIGHT * 0.6)));
        nodes.push(GraphNode::new(Pos2::new(0.0, WINDOW_HEIGHT * 0.4)));

        GraphEditor {
            position: Pos2::ZERO,
            is_dragging: false,
            last_drag_position: None,
            nodes,
        }
    }

    pub fn show(&mut self, ui: &mut egui::Ui) {
        let editor_rect = ui.max_rect();
        ui.allocate_rect(editor_rect, egui::Sense::hover());

        ui.set_clip_rect(editor_rect);

        let cursor_position = ui.ctx().input(|i| i.pointer.hover_pos()).unwrap_or(Pos2::ZERO);
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

        for node in self.nodes.iter_mut() {
            node.show(ui, self.position, cursor_position);
        }

        let e = ui.allocate_rect(editor_rect, egui::Sense::hover());

        if e.hovered() {
            
        }
    }

    fn start_dragging(&mut self) {
        self.is_dragging = true;
    }

    fn stop_dragging(&mut self) {
        self.is_dragging = false;
            self.last_drag_position = None;
    }
}