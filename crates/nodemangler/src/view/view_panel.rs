use eframe::{
    egui,
    epaint::{ColorImage, Rounding, Stroke},
};
use egui::{Color32, Pos2, Rect};
use image::DynamicImage;
use mangler::nodes::node::Node;

use crate::graph::graph_node::GraphNode;

const BACKGROUND_COLOR: Color32 = egui::Color32::from_gray(35);
const GRID_COLOR: Color32 = egui::Color32::from_gray(45);

pub struct ViewPanel {
    image: Option<egui::TextureHandle>,
    image_node_id: Option<String>,
    position: Pos2,
}

impl ViewPanel {
    pub fn new() -> ViewPanel {
        ViewPanel {
            image: None,
            image_node_id: None,
            position: Pos2::ZERO,
        }
    }

    pub fn show(&mut self, ui: &mut egui::Ui, viewing_node: Option<&GraphNode>) {
        let rect = ui.max_rect();

        // bg
        ui.painter().add(egui::Shape::rect_filled(
            rect,
            Rounding::none(),
            BACKGROUND_COLOR,
        ));

        self.draw_background_grid(ui, rect, self.position);

        if let Some(node) = viewing_node {
            if self.image_node_id.is_none() || self.image_node_id.clone().unwrap() != node.id {
                self.create_thumbnail(ui, node);
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

        self.draw_left_right_borders(ui, rect);
    }

    pub fn draw_left_right_borders(&self, ui: &mut egui::Ui, rect: Rect) {
        let size = 2.0;
        let stroke = Stroke::new(size, egui::Color32::from_gray(10));

        let mut points: Vec<Pos2> = Vec::with_capacity(2);
        points.push(Pos2::new(rect.left() + (size * 0.5), rect.top()));
        points.push(Pos2::new(rect.left() + (size * 0.5), rect.bottom()));

        ui.painter().add(egui::Shape::line(points.clone(), stroke));

        points.clear();

        points.push(Pos2::new(rect.right() - (size * 0.5), rect.top()));
        points.push(Pos2::new(rect.right() - (size * 0.5), rect.bottom()));

        ui.painter().add(egui::Shape::line(points.clone(), stroke));
    }


    pub fn draw_background_grid(&self, ui: &mut egui::Ui, rect: Rect, graph_position: Pos2) {
        let stroke = Stroke::new(1.0, GRID_COLOR);
        let grid_size: f32 = 50.0;
        
        let mut x = rect.min.x + (graph_position.x % grid_size);
        let mut y = rect.min.y + (graph_position.y % grid_size);
        while x <= rect.max.x {
            let mut points: Vec<Pos2> = Vec::with_capacity(2);
            points.push(Pos2::new(x, rect.min.y));
            points.push(Pos2::new(x, rect.max.y));
            ui.painter().add(egui::Shape::line(points.clone(), stroke));

            x += grid_size;
        }

        while y <= rect.max.y {
            let mut points: Vec<Pos2> = Vec::with_capacity(2);
            points.push(Pos2::new(rect.min.x, y));
            points.push(Pos2::new(rect.max.x, y));
            ui.painter().add(egui::Shape::line(points.clone(), stroke));

            y += grid_size;
        }
    }


    fn create_thumbnail(&mut self, ui: &mut egui::Ui, node: &GraphNode) {
        let color_image = match &node.outputs[0].value {
            mangler::value::Value::Rgba32FImage(value) => {
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
            mangler::value::Value::RgbaImage(value) => {
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
            mangler::value::Value::GrayImage(value) => {
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
