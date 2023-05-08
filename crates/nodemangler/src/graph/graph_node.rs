use eframe::egui;
use eframe::epaint::Rounding;
use egui::{Pos2, Vec2, Rect, Stroke};

pub const MIN_NODE_SIZE: [f32; 2] = [200.0, 200.0];

pub struct GraphNode {
    position: egui::Pos2,
    is_dragging: bool,
    last_drag_position: Option<Pos2>,
}

impl GraphNode {
    pub fn new(position: Pos2) -> GraphNode {
        GraphNode {
            position,
            is_dragging: false,
            last_drag_position: None,
        }
    }

    pub fn show(&mut self, ui: &mut egui::Ui, graph_position: Pos2, cursor_position: Pos2) {
        let pos = graph_position + self.position.to_vec2();
        let rect = Rect::from_center_size(pos, Vec2::new(150.0, 50.0));
        let rounding = Rounding::same(3.0);
        let stroke = Stroke::new(2.0, egui::Color32::from_gray(110));

        let cursor_inside = rect.contains(cursor_position);

        let bg_response = ui.allocate_rect(
            rect,
            egui::Sense::click().union(egui::Sense::drag()),
        );

        if bg_response.clicked() {
            // clicked on bg
        } else if bg_response.drag_started() {
            self.start_dragging();
        } else if bg_response.drag_released() {
            self.stop_dragging();
        }

        if self.is_dragging {
            if let Some(last_drag_position) = self.last_drag_position {
                self.position += cursor_position - last_drag_position;
            }

            self.last_drag_position = Some(cursor_position);
        }

        ui.painter().add(egui::Shape::rect_filled(
            rect,
            rounding,
            egui::Color32::from_gray(70),
        ));

        ui.painter().add(egui::Shape::rect_stroke(
            rect,
            rounding,
            stroke
        ));
    }

    fn start_dragging(&mut self) {
        self.is_dragging = true;
    }

    fn stop_dragging(&mut self) {
        self.is_dragging = false;
            self.last_drag_position = None;
    }
}