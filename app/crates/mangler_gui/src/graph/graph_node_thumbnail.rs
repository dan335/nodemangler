use crate::themes::theme::Theme;
use crate::{graph_to_view_space_pos2, graph_to_view_space};
use eframe::epaint::{Color32, FontId};
use eframe::{egui, emath::Align2};
use egui::{Pos2, Rect, Vec2};


#[derive(Clone)]
pub enum GraphNodeThumbnail {
    Image {
        texture_handle: egui::TextureHandle,
        width: u32,
        height: u32,
        /// Number of channels (1–4).
        channels: u32,
    },
    Color {
        texture_handle: egui::TextureHandle,
    },
    Text(String),
}

impl GraphNodeThumbnail {
    pub fn show(
        &self,
        ui: &mut egui::Ui,
        top_center_pos: Pos2,
        graph_zoom: f32,
        theme: &Theme,
    ) {
        match self {
            GraphNodeThumbnail::Image { texture_handle, width, height, channels } => {
                // image
                let thumb_size =
                    graph_to_view_space_pos2(graph_zoom, texture_handle.size_vec2().to_pos2()).to_vec2();

                ui.painter().image(
                    texture_handle.id(),
                    Rect::from_center_size(
                        top_center_pos + Vec2::new(0.0, thumb_size.y / 2.0) + Vec2::new(0.0, 2.0),
                        thumb_size,
                    ),
                    Rect::from_min_max(Pos2::new(0.0, 0.0), Pos2::new(1.0, 1.0)),
                    Color32::WHITE,
                );

                let info_pos = Pos2::new(
                    top_center_pos.x - thumb_size.x * 0.5,
                    top_center_pos.y + thumb_size.y + 10.0,
                );

                let channels_pos = Pos2 {
                    x: info_pos.x,
                    y: info_pos.y,
                };

                let res_pos = Pos2 {
                    x: info_pos.x + thumb_size.x,
                    y: info_pos.y,
                };

                // channel count label
                let ch_label = if *channels == 1 {
                    "1 channel".to_string()
                } else {
                    format!("{} channels", channels)
                };
                ui.painter().text(
                    channels_pos,
                    Align2::LEFT_TOP,
                    ch_label,
                    egui::FontId::monospace(10.0),
                    Color32::from(theme.get().text_faint),
                );

                // image res                
                ui.painter().text(
                    res_pos,
                    Align2::RIGHT_TOP,
                    format!("{}x{}", width, height),
                    egui::FontId::monospace(10.0),
                    Color32::from(theme.get().text_faint),
                );
            },
            GraphNodeThumbnail::Text(txt) => {
                ui.painter().text(
                    Pos2::new(top_center_pos.x, top_center_pos.y + 10.0),
                    Align2::CENTER_TOP,
                    txt,
                    FontId::proportional(graph_to_view_space(graph_zoom, 20.0)),
                    Color32::from(theme.get().override_text_color),
                );
            },
            GraphNodeThumbnail::Color { texture_handle } => {
                // image
                let thumb_size =
                    graph_to_view_space_pos2(graph_zoom, texture_handle.size_vec2().to_pos2()).to_vec2();

                ui.painter().image(
                    texture_handle.id(),
                    Rect::from_center_size(
                        top_center_pos + Vec2::new(0.0, thumb_size.y / 2.0) + Vec2::new(0.0, 2.0),
                        thumb_size,
                    ),
                    Rect::from_min_max(Pos2::new(0.0, 0.0), Pos2::new(1.0, 1.0)),
                    Color32::WHITE,
                );
            },
        }
    }
}