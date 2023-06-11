use eframe::{
    egui,
    epaint::{Rounding, Stroke},
};
use egui::{Color32, Pos2, Rect};

use crate::{graph::graph_node::GraphNode, theme::Theme};

pub struct ViewPanel {
    image: Option<egui::TextureHandle>,
    image_node_id: Option<String>,
    position: Pos2,
}

impl ViewPanel {
    pub fn new() -> ViewPanel {
        ViewPanel {
            image: None,
            image_node_id: None,
            position: Pos2::ZERO,
        }
    }

    pub fn show(&mut self, ui: &mut egui::Ui, viewing_node: Option<&GraphNode>, theme: &Theme) {
        let rect = ui.max_rect();

        // bg
        ui.painter().add(egui::Shape::rect_filled(
            rect,
            Rounding::none(),
            theme.grid_bg,
        ));

        self.draw_background_grid(ui, rect, self.position, theme);

        if let Some(node) = viewing_node {
            if self.image_node_id.is_none() || self.image_node_id.clone().unwrap() != node.id {
                //self.create_thumbnail(ui, node);
            }

            if let Some(image) = &self.image {
                ui.centered_and_justified(|ui| {
                    ui.painter().image(
                        image.id(),
                        Rect::from_center_size(ui.max_rect().center(), image.size_vec2()),
                        Rect::from_min_max(Pos2::new(0.0, 0.0), Pos2::new(1.0, 1.0)),
                        Color32::WHITE,
                    );
                });
            }
        }

        self.draw_left_right_borders(ui, rect, theme);
    }

    pub fn draw_left_right_borders(&self, ui: &mut egui::Ui, rect: Rect, theme: &Theme) {
        let size = 2.0;
        let stroke = Stroke::new(size, egui::Color32::from(theme.panel_border_lines));

        let mut points: Vec<Pos2> = Vec::with_capacity(2);
        points.push(Pos2::new(rect.left() + (size * 0.5), rect.top()));
        points.push(Pos2::new(rect.left() + (size * 0.5), rect.bottom()));

        ui.painter().add(egui::Shape::line(points.clone(), stroke));

        points.clear();

        points.push(Pos2::new(rect.right() - (size * 0.5), rect.top()));
        points.push(Pos2::new(rect.right() - (size * 0.5), rect.bottom()));

        ui.painter().add(egui::Shape::line(points.clone(), stroke));
    }

    pub fn draw_background_grid(&self, ui: &mut egui::Ui, rect: Rect, graph_position: Pos2, theme: &Theme) {
        let stroke = Stroke::new(1.0, theme.grid_lines);
        let grid_size: f32 = 50.0;

        let mut x = rect.min.x + (graph_position.x % grid_size);
        let mut y = rect.min.y + (graph_position.y % grid_size);
        while x <= rect.max.x {
            let points: Vec<Pos2> = vec![
                Pos2::new(x, rect.min.y),
                Pos2::new(x, rect.max.y),
            ];
            ui.painter().add(egui::Shape::line(points.clone(), stroke));

            x += grid_size;
        }

        while y <= rect.max.y {
            let points: Vec<Pos2> = vec![
                Pos2::new(rect.min.x, y),
                Pos2::new(rect.max.x, y),
            ];
            ui.painter().add(egui::Shape::line(points.clone(), stroke));

            y += grid_size;
        }
    }
}
