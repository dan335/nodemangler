use eframe::{
    egui,
    epaint::{ColorImage, Rounding},
};
use egui::{Color32, Pos2, Rect};
use image::DynamicImage;
use mangler::nodes::node::Node;

pub struct ViewPanel {
    image: Option<egui::TextureHandle>,
    image_node_id: Option<String>,
}

impl ViewPanel {
    pub fn new() -> ViewPanel {
        ViewPanel {
            image: None,
            image_node_id: None,
        }
    }

    pub fn show(&mut self, ui: &mut egui::Ui, viewing_node: Option<&Node>, image_is_dirty: bool) {
        // bg
        ui.painter().add(egui::Shape::rect_filled(
            ui.max_rect(),
            Rounding::none(),
            egui::Color32::from_gray(30),
        ));

        // create image
        if let Some(node) = viewing_node {
            if self.image.is_none()
                || &node.id != self.image_node_id.as_ref().unwrap()
                || image_is_dirty
            {
                let color_image = match &node.outputs[0].value {
                    mangler::value::Value::ImageRgba32F(value) => {
                        let image_buffer = DynamicImage::ImageRgba32F(value.clone())
                            .resize(
                                ui.max_rect().width() as u32,
                                ui.max_rect().height() as u32,
                                image::imageops::FilterType::Triangle,
                            )
                            .to_rgba8();
                        let pixels = image_buffer.as_flat_samples();
                        let size = [
                            image_buffer.width() as usize,
                            image_buffer.height() as usize,
                        ];
                        Some(ColorImage::from_rgba_unmultiplied(size, pixels.as_slice()))
                    }
                    mangler::value::Value::ImageRgba8(value) => {
                        let image_buffer = DynamicImage::ImageRgba8(value.clone())
                            .resize(
                                ui.max_rect().width() as u32,
                                ui.max_rect().height() as u32,
                                image::imageops::FilterType::Triangle,
                            )
                            .to_rgba8();
                        let pixels = image_buffer.as_flat_samples();
                        let size = [
                            image_buffer.width() as usize,
                            image_buffer.height() as usize,
                        ];
                        Some(ColorImage::from_rgba_unmultiplied(size, pixels.as_slice()))
                    }
                    mangler::value::Value::ImageGray8(value) => {
                        let image_buffer = DynamicImage::ImageLuma8(value.clone())
                            .resize(
                                ui.max_rect().width() as u32,
                                ui.max_rect().height() as u32,
                                image::imageops::FilterType::Triangle,
                            )
                            .to_rgba8();
                        let pixels = image_buffer.as_flat_samples();
                        let size = [
                            image_buffer.width() as usize,
                            image_buffer.height() as usize,
                        ];
                        Some(ColorImage::from_rgba_unmultiplied(size, pixels.as_slice()))
                    }
                    _ => None,
                };

                if let Some(img) = color_image {
                    self.image = Some(ui.ctx().load_texture(
                        node.id.clone(),
                        img,
                        Default::default(),
                    ));
                    self.image_node_id = Some(node.id.clone());
                }
            }
        }

        if let Some(image) = &self.image {
            ui.centered_and_justified(|ui| {
                ui.painter().image(
                    image.id(),
                    Rect::from_center_size(ui.max_rect().center(), image.size_vec2()),
                    Rect::from_min_max(Pos2::new(0.0, 0.0), Pos2::new(1.0, 1.0)),
                    Color32::WHITE,
                );
            });
        }
    }
}
