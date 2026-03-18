use eframe::{egui, emath::Align2};
use egui::{Pos2, Rect};
use epaint::Color32;
use std::time::Duration;

use crate::{graph_to_view_space, themes::theme::Theme};

pub fn show_graph_node_info(
    ui: &mut egui::Ui,
    time_option: Option<Duration>,
    node_rect: Rect,
    graph_zoom: f32,
    theme: &Theme,
) {
    // ms
    if let Some(time) = time_option {
        puffin::profile_scope!("graph node.inputs show time");
        let pos = Pos2 {
            x: node_rect.right_top().x,
            y: node_rect.right_top().y - 5.0,
        };
        let text = format!("{:.1} ms", time.as_nanos() as f64 / 1_000_000.0);
        ui.painter().text(
            pos,
            Align2::RIGHT_BOTTOM,
            text,
            egui::FontId::monospace(graph_to_view_space(graph_zoom, 10.0)),
            Color32::from(theme.get().text_faint),
        );
    }
}
