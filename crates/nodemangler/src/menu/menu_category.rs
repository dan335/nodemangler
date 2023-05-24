use eframe::{egui, emath::Align2, epaint::{FontId, PathShape, Vec2}};
use egui::{Color32, Pos2, Rect, Rounding, Stroke};
use mangler::{
    node_settings::NodeSettings,
    operation::{ConnectionSettings, Operation}, OperationDescription, OperationCategory,
};

use super::menu_button::MenuButton;

const BUTTON_HEIGHT: f32 = 36.0;
const BACKGROUND_COLOR: Color32 = egui::Color32::from_gray(70);
const BACKGROUND_COLOR_HOVER: Color32 = egui::Color32::from_gray(80);

pub struct MenuCategory {
    name: String,
    pub buttons: Vec<MenuButton>,
    pub is_collapsed: bool,
}

impl MenuCategory {
    pub fn new(category: &OperationCategory) -> Self {
        let mut buttons: Vec<MenuButton> = Vec::new();

        for operation in category.operations.iter() {
            buttons.push(MenuButton::new(operation));
        }

        Self {
            name: category.name.to_owned(),
            buttons,
            is_collapsed: true,
        }
    }

    pub fn show(&mut self, ui: &mut egui::Ui, index: usize) {
        let container_rect = ui.max_rect();
        let button_top_position = container_rect.top() + (BUTTON_HEIGHT * index as f32);
        let button_min = Pos2::new(container_rect.left(), button_top_position);
        let button_max = Pos2::new(container_rect.right(), button_top_position + BUTTON_HEIGHT);
        let button_rect = Rect::from_two_pos(button_min, button_max);
        let rounding = Rounding::same(2.0);
        let stroke = Stroke::new(1.0, egui::Color32::from_gray(90));

        ui.centered_and_justified(|ui| {
            let rect = Rect::from_min_max(
                button_rect.min,
                Pos2::new(button_rect.max.x, button_rect.max.y),
            );

            let response = ui.allocate_rect(rect, egui::Sense::hover());

            let mut background_color = BACKGROUND_COLOR;
            if response.hovered() {
                background_color = BACKGROUND_COLOR_HOVER;
            }

            ui.painter().add(egui::Shape::rect_filled(
                rect.shrink(1.0),
                rounding,
                background_color,
            ));

            // let mut points: Vec<Pos2> = Vec::with_capacity(2);
            // points.push(Pos2::new(rect.left(), rect.top()));
            // points.push(Pos2::new(rect.right(), rect.top()));
            // ui.painter().add(egui::Shape::line(points.clone(), stroke));

            // points.clear();
            // points.push(Pos2::new(rect.left(), rect.bottom() + 1.0));
            // points.push(Pos2::new(rect.right(), rect.bottom() + 1.0));
            // ui.painter().add(egui::Shape::line(points, stroke));

            let mut points: Vec<Pos2> = Vec::new();

            if self.is_collapsed {
                points.push(rect.left_center() + Vec2::new(10.0, -5.0));
                points.push(rect.left_center() + Vec2::new(15.0, 0.0));
                points.push(rect.left_center() + Vec2::new(10.0, 5.0));
            } else {
                points.push(rect.left_center() + Vec2::new(5.0, 0.0));
                points.push(rect.left_center() + Vec2::new(15.0, 0.0));
                points.push(rect.left_center() + Vec2::new(10.0, 5.0));
            }
            

            let triangle = PathShape::convex_polygon(points, Color32::from_gray(150), stroke);

            ui.painter().add(triangle);

            ui.painter().text(
                Pos2::new(rect.left() + 25.0, rect.center().y),
                Align2::LEFT_CENTER,
                self.name.clone(),
                FontId::default(),
                Color32::from_gray(220),
            );

            let response = ui.allocate_rect(rect, egui::Sense::click());

            if response.clicked() {
                self.is_collapsed = !self.is_collapsed;
            }
        });
    }
}