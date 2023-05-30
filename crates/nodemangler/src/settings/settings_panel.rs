// use std::path::PathBuf;

// use eframe::egui;
// use mangler::SetNodeInputMessage;
// use tokio::sync::mpsc::Sender;

// use crate::graph::graph_node::GraphNode;

// use super::{graph_settings_panel, node_settings_panel};

// pub fn show(
//     ui: &mut egui::Ui,
//     node_option: Option<&mut GraphNode>,
//     tx_input: Sender<SetNodeInputMessage>,
//     program_name: &mut String,
//     program_path: &mut Option<PathBuf>,
// ) {
//     let left_top = ui.max_rect().left_top();
//     let right_bottom = ui.max_rect().right_bottom();
//     let padding = 10.0;

//     // create rect for content
//     let ui_rect = egui::Rect::from_two_pos(
//         egui::Pos2::new(left_top.x + padding, left_top.y + padding),
//         egui::Pos2::new(right_bottom.x - padding, right_bottom.y - padding),
//     );

//     ui.allocate_ui_at_rect(ui_rect, |ui| {
//         if let Some(node) = node_option {
//             node_settings_panel::show(ui, node, tx_input);
//         } else {
//             graph_settings_panel::show(ui, program_name, program_path);
//         }
//     });
// }
