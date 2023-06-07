use eframe::{egui, emath::Align2};
use egui::{Pos2, Rect};
use mangler::output::Output;
use mangler::value::Value;
use std::time::Duration;

pub fn show_graph_node_info(
    ui: &mut egui::Ui,
    time_option: Option<Duration>,
    node_rect: Rect,
    outputs: &Vec<Output>,
) {
    // ms
    if let Some(time) = time_option {
        puffin::profile_scope!("graph node.inputs show time");
        let pos = Pos2 {
            x: node_rect.right_top().x,
            y: node_rect.right_top().y - 5.0,
        };
        let text = format!("{:.4} ms", time.as_nanos() as f64 / 1_000_000.0);
        ui.painter().text(
            pos,
            Align2::RIGHT_BOTTOM,
            text,
            egui::FontId::monospace(10.0),
            egui::Color32::from_gray(200),
        );
    }

    // image format
    if outputs.len() > 0 {
        if let Value::DynamicImage(image) = outputs[0].value.clone() {
            let bits = image.color().bits_per_pixel() / image.color().channel_count() as u16;
            let channels = match image.color().channel_count() {
                1 => "r".to_string(),
                2 => "rg".to_string(),
                3 => "rgb".to_string(),
                4 => "rgba".to_string(),
                _ => "".to_string(),
            };

            // if image.color().has_alpha() {
            //     channels = format!("{}a", channels);
            // }

            let pos = Pos2 {
                x: node_rect.right_top().x,
                y: node_rect.right_top().y - 20.0,
            };
            let text = format!("{}{}", channels, bits);
            ui.painter().text(
                pos,
                Align2::RIGHT_BOTTOM,
                text,
                egui::FontId::monospace(10.0),
                egui::Color32::from_gray(200),
            );
        }
    }
}
