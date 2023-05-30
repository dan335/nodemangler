// use eframe::{egui, emath::Align2, epaint::FontId};
// use egui::{Color32, Pos2, Rect, Rounding, Stroke};
// use mangler::{
//     node_settings::NodeSettings,
//     operation::{ConnectionSettings, Operation}, OperationListItem, OperationDescription,
// };

// const BUTTON_HEIGHT: f32 = 36.0;
// const BACKGROUND_COLOR: Color32 = egui::Color32::from_gray(50);
// const BACKGROUND_COLOR_HOVER: Color32 = egui::Color32::from_gray(80);

// pub struct MenuButton {
//     pub operation: Operation,
//     pub name: String,
// }

// impl MenuButton {
//     pub fn new(cateogy: &OperationDescription) -> Self {
//         Self {
//             operation: cateogy.operation.clone(),
//             name: cateogy.operation.settings().name,
//         }
//     }

//     pub fn show(&mut self, ui: &mut egui::Ui, button_index: usize) -> MenuButtonResult {
//         let container_rect = ui.max_rect();
//         let button_top_position = container_rect.top() + (BUTTON_HEIGHT * button_index as f32);
//         let button_min = Pos2::new(container_rect.left(), button_top_position);
//         let button_max = Pos2::new(container_rect.right(), button_top_position + BUTTON_HEIGHT);
//         let button_rect = Rect::from_two_pos(button_min, button_max);
//         let rounding = Rounding::same(2.0);
//         let stroke = Stroke::new(1.0, egui::Color32::from_gray(90));

//         let mut is_dragging = false;

//         ui.centered_and_justified(|ui| {
//             //ui.centered(|ui| {

//             let rect = Rect::from_min_max(
//                 button_rect.min,
//                 Pos2::new(button_rect.max.x, button_rect.max.y),
//             );

//             let response = ui.allocate_rect(rect, egui::Sense::drag().union(egui::Sense::hover()));

//             let mut background_color = BACKGROUND_COLOR;
//             if response.hovered() {
//                 background_color = BACKGROUND_COLOR_HOVER;
//             }

//             ui.painter().add(egui::Shape::rect_filled(
//                 rect.shrink(1.0),
//                 rounding,
//                 background_color,
//             ));

//             // let mut points: Vec<Pos2> = Vec::with_capacity(2);
//             // points.push(Pos2::new(rect.left(), rect.top()));
//             // points.push(Pos2::new(rect.right(), rect.top()));
//             // ui.painter().add(egui::Shape::line(points.clone(), stroke));

//             // points.clear();
//             // points.push(Pos2::new(rect.left(), rect.bottom() + 1.0));
//             // points.push(Pos2::new(rect.right(), rect.bottom() + 1.0));
//             // ui.painter().add(egui::Shape::line(points, stroke));

//             ui.painter().text(
//                 Pos2::new(rect.left() + 40.0, rect.center().y),
//                 Align2::LEFT_CENTER,
//                 self.name.clone(),
//                 FontId::default(),
//                 Color32::from_gray(220),
//             );

//             if response.clicked() {
//             } else if response.drag_started() {
//                 is_dragging = true;
//             } else if response.drag_released() {
//             }
//         });

//         MenuButtonResult { is_dragging }
//     }
// }

// pub struct MenuButtonResult {
//     pub is_dragging: bool,
// }
