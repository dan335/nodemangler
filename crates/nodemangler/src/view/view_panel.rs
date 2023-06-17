use eframe::{
    egui::{self, Pos2, Rect},
    epaint::{Rounding, Stroke},
};
use epaint::{Vec2, TextureHandle};
use image::DynamicImage;

use crate::{graph::graph_node::GraphNode, themes::theme::Theme};

pub struct ViewPanel {
    image_texture_handle: Option<egui::TextureHandle>,
    image_id_index: Option<(String, usize, String)>,  // node id, output index, change_id
    position: Pos2,
}

impl ViewPanel {
    pub fn new() -> ViewPanel {
        ViewPanel {
            image_texture_handle: None,
            image_id_index: None,
            position: Pos2::ZERO,
        }
    }

    pub fn show(&mut self, ui: &mut egui::Ui, graph_node: &GraphNode, output_index: usize, theme: &Theme) {

        ui.with_layout(egui::Layout::centered_and_justified(egui::Direction::LeftToRight), |ui| {
            //ui.allocate_space(Vec2::new(300.0, 300.0));

            let rect = ui.max_rect();

            // bg
            ui.painter().add(egui::Shape::rect_filled(
                rect,
                Rounding::none(),
                theme.get().grid_bg,
            ));

            // bg grid
            self.draw_background_grid(ui, rect, self.position, theme);

            if let Some(output) = graph_node.outputs.get(output_index) {
                match &output.value {
                    mangler::value::Value::Bool(value) => {
                        ui.label(value.to_string());
                    },
                    mangler::value::Value::Integer(value) => {
                        ui.label(value.to_string());
                    },
                    mangler::value::Value::Decimal(value) => {
                        ui.label(value.to_string());
                    },
                    mangler::value::Value::String(value) => {
                        ui.label(value);
                    },
                    mangler::value::Value::DynamicImage {data, change_id} => {
                        match self.image_id_index.clone() {
                            Some((image_node_id, image_output_index, image_change_id)) => {
                                if image_node_id != graph_node.id || image_output_index != output_index || change_id != &image_change_id {
                                    let texture_handle = self.create_egui_image(ui, data.clone(), graph_node.id.clone());
                                    self.image_texture_handle = Some(texture_handle);
                                    self.image_id_index = Some((graph_node.id.clone(), output_index, change_id.clone()));
                                }
                            },
                            None => {
                                let texture_handle = self.create_egui_image(ui, data.clone(), graph_node.id.clone());
                                self.image_texture_handle = Some(texture_handle);
                                self.image_id_index = Some((graph_node.id.clone(), output_index, change_id.clone()));
                            },
                        }

                        if let Some(texture_handle) = &self.image_texture_handle {
                            ui.image(texture_handle, Vec2::new(texture_handle.size()[0] as f32, texture_handle.size()[1] as f32));
                        }
                    },
                    mangler::value::Value::Path(path) => {
                        ui.label(path.to_str().unwrap_or("None").to_string());
                    },
                    mangler::value::Value::FilterType(value) => {
                        ui.label(format!("{:?}", value));
                    },
                    mangler::value::Value::ColorFormat(value) => {
                        ui.label(format!("{:?}", value));
                    },
                    mangler::value::Value::Trigger => {
                        ui.label(format!("trigger"));
                    },
                    mangler::value::Value::ImageType(value) => {
                        ui.label(format!("{:?}", value));
                    },
                }
            }
        });
    }

    fn create_egui_image(&self, ui: &mut egui::Ui, dynamic_image: DynamicImage, name: String) -> TextureHandle {
        let rgba_image = dynamic_image.to_rgba8();

        let pixels = rgba_image.as_flat_samples();

        let size = [
            rgba_image.width() as usize,
            rgba_image.height() as usize,
        ];

        let color_image = epaint::ColorImage::from_rgba_unmultiplied(
            size,
            pixels.as_slice(),
        );

        ui.ctx().load_texture(
            name,
            color_image,
            Default::default(),
        )
    }
    

    pub fn draw_background_grid(&self, ui: &mut egui::Ui, rect: Rect, graph_position: Pos2, theme: &Theme) {
        let stroke = Stroke::new(1.0, theme.get().grid_lines);
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
