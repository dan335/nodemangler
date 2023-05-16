use eframe::{egui, emath::Align2, epaint::FontId};
use egui::{Color32, Pos2, Rect, Rounding, Stroke};
use mangler::nodes::{
    node_settings::NodeSettings,
    operation::{ConnectionSettings, Operation},
};

const BUTTON_HEIGHT: f32 = 36.0;

pub struct MenuButton {
    pub node_settings: NodeSettings,
    pub input_settings: Vec<ConnectionSettings>,
    pub output_settings: Vec<ConnectionSettings>,
    pub operation: Operation,
}

// impl Clone for MenuButton {
//     fn clone(&self) -> Self {
//         MenuButton {
//             operation: self.operation.clone(),
//         }
//     }
// }

impl MenuButton {
    // pub fn new(node_settings: NodeSettings, input_settings: Vec<ConnectionSettings>, output_settings: Vec<ConnectionSettings>) -> MenuButton {
    //     MenuButton {
    //         node_settings,
    //         input_settings,
    //         output_settings,
    //         T,
    //     }
    // }

    pub fn show(&mut self, ui: &mut egui::Ui, button_index: usize) -> MenuButtonResult {
        let container_rect = ui.max_rect();
        let button_top_position = container_rect.top() + (BUTTON_HEIGHT * button_index as f32);
        let button_min = Pos2::new(container_rect.left(), button_top_position);
        let button_max = Pos2::new(container_rect.right(), button_top_position + BUTTON_HEIGHT);
        let button_rect = Rect::from_two_pos(button_min, button_max);
        let rounding = Rounding::same(2.0);
        let stroke = Stroke::new(1.0, egui::Color32::from_gray(90));

        let mut is_dragging = false;

        ui.centered_and_justified(|ui| {
            //ui.centered(|ui| {

            let rect = Rect::from_min_max(
                button_rect.min,
                Pos2::new(button_rect.max.x, button_rect.max.y),
            );
            ui.painter().add(egui::Shape::rect_filled(
                rect,
                rounding,
                egui::Color32::from_gray(50),
            ));

            let mut points: Vec<Pos2> = Vec::with_capacity(2);
            points.push(Pos2::new(rect.left(), rect.top()));
            points.push(Pos2::new(rect.right(), rect.top()));
            ui.painter().add(egui::Shape::line(points.clone(), stroke));

            points.clear();
            points.push(Pos2::new(rect.left(), rect.bottom() + 1.0));
            points.push(Pos2::new(rect.right(), rect.bottom() + 1.0));
            ui.painter().add(egui::Shape::line(points, stroke));

            ui.painter().text(
                rect.center(),
                Align2::CENTER_CENTER,
                self.node_settings.name.clone(),
                FontId::default(),
                Color32::from_gray(220),
            );

            let response = ui.allocate_rect(rect, egui::Sense::click().union(egui::Sense::drag()));

            if response.clicked() {
            } else if response.drag_started() {
                is_dragging = true;
            } else if response.drag_released() {
            }
        });

        MenuButtonResult { is_dragging }
    }
}

pub struct MenuButtonResult {
    pub is_dragging: bool,
}
