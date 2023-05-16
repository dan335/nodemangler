use std::println;

use crate::graph::graph_input::draw_graph_input;
use crate::graph::graph_output::draw_graph_output;
use eframe::epaint::{Rounding, FontId, Color32, TextureId, ColorImage};
use eframe::{egui, emath::Align2};
use egui::{Pos2, Rect, Vec2};
use image::{ImageBuffer, Rgba, DynamicImage};
use mangler::nodes::node::Node;
use mangler::nodes::node_settings::NodeSettings;

use super::graph_editor::TempConnection;

pub const NODE_SIZE: Vec2 = Vec2::new(132.0, 132.0);
const NODE_ROUNDING: f32 = 2.0;

#[derive(Clone)]
pub struct GraphNode {
    pub id: String,
    position: egui::Pos2,
    settings: NodeSettings,
    is_dragging: bool,
    last_drag_position: Option<Pos2>,
    thumbnail: Option<egui::TextureHandle>,
    pub thumbnail_is_dirty: bool,   // node has updated, thumbnail needs updating
}

impl GraphNode {
    pub fn new(id: String, position: Pos2, settings: NodeSettings) -> GraphNode {
        GraphNode {
            id,
            position,
            settings,
            is_dragging: false,
            last_drag_position: None,
            thumbnail: None,
            thumbnail_is_dirty: true,
        }
    }

    pub fn get_rect(&self, graph_position: Pos2) -> Rect {
        Rect::from_center_size(self.position + graph_position.to_vec2(), NODE_SIZE)
    }

    pub fn show(
        &mut self,
        ui: &mut egui::Ui,
        graph_position: Pos2,
        cursor_position: Pos2,
        node: &Node,
        is_editing: bool,
        is_viewing: bool,
    ) -> GraphNodeResponse {
        let mut graph_node_response = GraphNodeResponse::default();
        //let pos = graph_position + self.position.to_vec2();
        let rounding = Rounding::same(NODE_ROUNDING);
        //let stroke = Stroke::new(1.0, egui::Color32::from_gray(110));

        //let cursor_inside = self.rect.contains(cursor_position);

        let bg_response = ui.allocate_rect(
            self.get_rect(graph_position),
            egui::Sense::click().union(egui::Sense::drag()),
        );

        if bg_response.clicked_by(egui::PointerButton::Primary) {
            self.stop_dragging();
            graph_node_response.is_left_click = true;
        } else if bg_response.clicked_by(egui::PointerButton::Secondary) {
            self.stop_dragging();
            graph_node_response.is_right_click = true;
        } else if bg_response.drag_started() {
            self.start_dragging();
        } else if bg_response.drag_released() {
            self.stop_dragging();
        }

        if self.is_dragging {
            if let Some(last_drag_position) = self.last_drag_position {
                self.position += cursor_position - last_drag_position;
            }

            self.last_drag_position = Some(cursor_position);
        }

        // bg
        ui.painter().add(egui::Shape::rect_filled(
            self.get_rect(graph_position),
            rounding,
            egui::Color32::from_gray(70),
        ));

        // inputs
        for (index, input) in node.inputs.iter().enumerate() {
            let input_output_response = draw_graph_input(
                input,
                self.get_input_position(index, graph_position),
                self.get_input_rect(index, graph_position),
                index,
                self.get_rect(graph_position),
                ui,
            );

            if input_output_response.has_started_creating_connection {
                graph_node_response.temp_connection = Some(TempConnection {
                    from_position: input_output_response.connection_from_position,
                    from_node_id: node.id.clone(),
                    from_connection_index: index,
                    from_connection_type: ConnectionType::Input,
                });
            }

            if input_output_response.has_stopped_creating_connection {
                graph_node_response.has_stopped_creating_connection = true;
                graph_node_response.connection_to_position =
                    input_output_response.connection_to_position;
            }
        }

        // outputs
        for (index, output) in node.outputs.iter().enumerate() {
            let input_output_response = draw_graph_output(
                output,
                self.get_output_position(index, graph_position),
                self.get_output_rect(index, graph_position),
                index,
                self.get_rect(graph_position),
                ui,
            );

            // started dragging from connection
            // create temp connection object
            if input_output_response.has_started_creating_connection {
                graph_node_response.temp_connection = Some(TempConnection {
                    from_position: input_output_response.connection_from_position,
                    from_node_id: node.id.clone(),
                    from_connection_index: index,
                    from_connection_type: ConnectionType::Output,
                });
            }

            if input_output_response.has_stopped_creating_connection {
                graph_node_response.has_stopped_creating_connection = true;
                graph_node_response.connection_to_position =
                    input_output_response.connection_to_position;
            }
        }

        // ms
        if let Some(time) = node.time {
            let pos = Pos2 { x: self.get_rect(graph_position).right_bottom().x, y: self.get_rect(graph_position).right_bottom().y + 5.0 };
            let text = format!("{:.4} ms", time.as_nanos() as f64 / 1_000_000.0);
            ui.painter().text(
                pos,
                Align2::RIGHT_TOP,
                text,
                egui::FontId::monospace(10.0),
                egui::Color32::from_gray(200),
            );
        }

        // thumbnail

        // convert to thumbnail
        // https://docs.rs/egui/latest/egui/struct.ColorImage.html#method.from_rgba_unmultiplied
        if self.thumbnail_is_dirty {
            self.thumbnail_is_dirty = false;

            let color_image = match &node.outputs[0].value {
                mangler::value::Value::ImageRgba32F(value) => {
                    let image_buffer = DynamicImage::ImageRgba32F(value.clone()).resize(NODE_SIZE.x as u32, NODE_SIZE.y as u32, image::imageops::FilterType::Triangle).to_rgba8();
                    let pixels = image_buffer.as_flat_samples();
                    let size = [image_buffer.width() as usize, image_buffer.height() as usize];
                    Some(ColorImage::from_rgba_unmultiplied(size, pixels.as_slice()))
                },
                mangler::value::Value::ImageRgba8(value) => {
                    let image_buffer = DynamicImage::ImageRgba8(value.clone()).resize(NODE_SIZE.x as u32, NODE_SIZE.y as u32, image::imageops::FilterType::Triangle).to_rgba8();
                    let pixels = image_buffer.as_flat_samples();
                    let size = [image_buffer.width() as usize, image_buffer.height() as usize];
                    Some(ColorImage::from_rgba_unmultiplied(size, pixels.as_slice()))
                },
                mangler::value::Value::ImageGray8(value) => {
                    let image_buffer = DynamicImage::ImageLuma8(value.clone()).resize(NODE_SIZE.x as u32, NODE_SIZE.y as u32, image::imageops::FilterType::Triangle).to_rgba8();
                    let pixels = image_buffer.as_flat_samples();
                    let size = [image_buffer.width() as usize, image_buffer.height() as usize];
                    Some(ColorImage::from_rgba_unmultiplied(size, pixels.as_slice()))
                },
                _ => None,
            };

            if let Some(img) = color_image {
                self.thumbnail = Some(ui.ctx().load_texture(self.id.clone(), img, Default::default()));
            }
        }

        // output
        match &node.outputs[0].value {
            mangler::value::Value::Integer(value) => {
                ui.painter().text(self.get_rect(graph_position).center(), Align2::CENTER_CENTER, value.to_string(), FontId::proportional(20.0), Color32::from_gray(200));
            },
            mangler::value::Value::Decimal(value) => {
                ui.painter().text(self.get_rect(graph_position).center(), Align2::CENTER_CENTER, value.to_string(), FontId::proportional(20.0), Color32::from_gray(200));
            },
            mangler::value::Value::String(value) => {
                ui.painter().text(self.get_rect(graph_position).center(), Align2::CENTER_CENTER, value.to_string(), FontId::proportional(20.0), Color32::from_gray(200));
            },

            mangler::value::Value::ImageRgba32F(_) => {
                if let Some(thumb) = &self.thumbnail {
                    ui.painter().image(thumb.id(), Rect::from_center_size(self.position + graph_position.to_vec2(), thumb.size_vec2()), Rect::from_min_max(Pos2::new(0.0, 0.0), Pos2::new(1.0, 1.0)), Color32::WHITE);
                }
            },

            mangler::value::Value::ImageRgba8(_) => {
                if let Some(thumb) = &self.thumbnail {
                    ui.painter().image(thumb.id(), Rect::from_center_size(self.position + graph_position.to_vec2(), thumb.size_vec2()), Rect::from_min_max(Pos2::new(0.0, 0.0), Pos2::new(1.0, 1.0)), Color32::WHITE);
                }
            },

            mangler::value::Value::ImageGray8(_) => {
                if let Some(thumb) = &self.thumbnail {
                    ui.painter().image(thumb.id(), Rect::from_center_size(self.position + graph_position.to_vec2(), thumb.size_vec2()), Rect::from_min_max(Pos2::new(0.0, 0.0), Pos2::new(1.0, 1.0)), Color32::WHITE);
                }
            },
        }

        // outline
        if is_editing {
            ui.painter().add(egui::Shape::rect_stroke(self.get_rect(graph_position), rounding, egui::Stroke::new(4.0, Color32::from_rgb(30, 150, 90))));
        }

        if is_viewing {
            //ui.painter().add(egui::Shape::rect_stroke(self.get_rect(graph_position).expand(10.0), rounding, egui::Stroke::new(2.0, Color32::GREEN)));
        }
        // ui.painter().add(egui::Shape::rect_stroke(
        //     rect,
        //     rounding,
        //     stroke
        // ));

        // text - name
        ui.painter().text(
            Pos2::new(
                self.get_rect(graph_position).center().x,
                self.get_rect(graph_position).top() - 20.0,
            ),
            Align2::CENTER_TOP,
            self.settings.name.clone(),
            egui::FontId::default(),
            egui::Color32::from_gray(220),
        );

        graph_node_response
    }

    fn start_dragging(&mut self) {
        self.is_dragging = true;
    }

    fn stop_dragging(&mut self) {
        self.is_dragging = false;
        self.last_drag_position = None;
    }

    pub fn get_input_position(&self, index: usize, graph_position: Pos2) -> Pos2 {
        let rect = self.get_rect(graph_position);
        Pos2::new(rect.left() - 14.0, rect.top() + 12.0 + 20.0 * index as f32)
    }

    pub fn get_output_position(&self, index: usize, graph_position: Pos2) -> Pos2 {
        let rect = self.get_rect(graph_position);
        Pos2::new(rect.right() + 14.0, rect.top() + 12.0 + 20.0 * index as f32)
    }

    pub fn get_input_rect(&self, index: usize, graph_position: Pos2) -> Rect {
        Rect::from_center_size(
            self.get_input_position(index, graph_position),
            Vec2::new(12.0, 12.0),
        )
    }

    pub fn get_output_rect(&self, index: usize, graph_position: Pos2) -> Rect {
        Rect::from_center_size(
            self.get_output_position(index, graph_position),
            Vec2::new(12.0, 12.0),
        )
    }
}

#[derive(Debug)]
pub struct GraphNodeResponse {
    pub temp_connection: Option<TempConnection>,
    pub has_stopped_creating_connection: bool,
    pub connection_to_position: Pos2,
    pub edit_node: bool,
    pub view_node: bool,
    pub is_right_click: bool,
    pub is_left_click: bool,
}

impl GraphNodeResponse {
    pub fn default() -> GraphNodeResponse {
        GraphNodeResponse {
            temp_connection: None,
            has_stopped_creating_connection: false,
            connection_to_position: Pos2::ZERO,
            edit_node: false,
            view_node: false,
            is_right_click: false,
            is_left_click: false,
        }
    }
}

pub struct InputOutputResponse {
    pub has_started_creating_connection: bool,
    pub connection_from_position: Pos2,
    pub has_stopped_creating_connection: bool,
    pub connection_to_position: Pos2,
}

impl InputOutputResponse {
    pub fn new() -> InputOutputResponse {
        InputOutputResponse {
            has_started_creating_connection: false,
            connection_from_position: Pos2::ZERO,
            has_stopped_creating_connection: false,
            connection_to_position: Pos2::ZERO,
        }
    }
}

#[derive(Debug, Clone)]
pub enum ConnectionType {
    Input,
    Output,
}
