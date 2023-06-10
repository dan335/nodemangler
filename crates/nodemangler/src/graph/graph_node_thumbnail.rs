use crate::{graph_to_view_space_pos2, graph_to_view_space};
use eframe::epaint::{Color32, FontId};
use eframe::{egui, emath::Align2};
use egui::{Pos2, Rect, Vec2};
use mangler::output::Output;


#[derive(Clone)]
pub enum GraphNodeThumbnail {
    Image(egui::TextureHandle),
    Text(String),
}

impl GraphNodeThumbnail {
    pub fn show(
        &self,
        ui: &mut egui::Ui,
        top_center_pos: Pos2,
        graph_zoom: f32
    ) {
        match self {
            GraphNodeThumbnail::Image(thumbnail) => {
                let thumb_size =
                    graph_to_view_space_pos2(graph_zoom, thumbnail.size_vec2().to_pos2()).to_vec2();

                ui.painter().image(
                    thumbnail.id(),
                    Rect::from_center_size(
                        top_center_pos + Vec2::new(0.0, thumb_size.y / 2.0) + Vec2::new(0.0, 2.0),
                        thumb_size,
                    ),
                    Rect::from_min_max(Pos2::new(0.0, 0.0), Pos2::new(1.0, 1.0)),
                    Color32::WHITE,
                );
            },
            GraphNodeThumbnail::Text(txt) => {
                ui.painter().text(
                    Pos2::new(top_center_pos.x, top_center_pos.y + 10.0),
                    Align2::CENTER_TOP,
                    txt,
                    FontId::proportional(graph_to_view_space(graph_zoom, 20.0)),
                    Color32::from_gray(200),
                );
            },
        }
    }
}