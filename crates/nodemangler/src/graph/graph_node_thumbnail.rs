use crate::graph_to_view_space_pos2;
use eframe::epaint::{Color32, FontId};
use eframe::{egui, emath::Align2};
use egui::{Pos2, Rect, Vec2};
use mangler::output::Output;
use mangler::value::Value;

pub fn show_graph_node_thumbnail(
    ui: &mut egui::Ui,
    outputs: &Vec<Output>,
    thumbnail: Option<egui::TextureHandle>,
    top_center_pos: Pos2,
    graph_zoom: f32,
) {
    // show output result on node
    if let Some(thumbnail) = thumbnail {
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
    } else {
        if outputs.len() > 0 {
            match outputs[0].value.clone() {
                Value::Bool(value) => {
                    show_output_text(ui, top_center_pos, value.to_string(), graph_zoom)
                }
                Value::Integer(value) => {
                    show_output_text(ui, top_center_pos, value.to_string(), graph_zoom)
                }
                Value::Decimal(value) => {
                    show_output_text(ui, top_center_pos, value.to_string(), graph_zoom)
                }
                Value::String(value) => {
                    show_output_text(ui, top_center_pos, value.to_string(), graph_zoom)
                }

                Value::FilterType(value) => {
                    show_output_text(ui, top_center_pos, format!("{:?}", value), graph_zoom)
                }
                Value::ImageFormat(value) => {
                    show_output_text(ui, top_center_pos, format!("{:?}", value), graph_zoom)
                }
                Value::UiButton(_) => todo!(),
                Value::DynamicImage(_) => {}
                Value::Path(_) => todo!(),
            }
        }
    }

    fn show_output_text(ui: &mut egui::Ui, position: Pos2, txt: String, graph_zoom: f32) {
        puffin::profile_scope!("graph node.show_output_text()");
        ui.painter().text(
            Pos2::new(position.x, position.y + 10.0),
            Align2::CENTER_TOP,
            txt,
            FontId::proportional(20.0 * graph_zoom),
            Color32::from_gray(200),
        );
    }
}
