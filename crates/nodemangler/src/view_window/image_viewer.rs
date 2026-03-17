use eframe::egui::{self};
use epaint::{TextureHandle, Pos2, CornerRadius, Rect, Stroke, Color32};
use image::DynamicImage;

use crate::{view_to_graph_space, graph_to_view_space, themes::theme::Theme, view_to_graph_space_pos2, graph_to_view_space_pos2};

const ZOOM_MULTIPLIER: f32 = 0.001;
const ZOOM_BOUNDS: [f32; 2] = [0.15, 5.0];

pub struct ImageViewer {
    image_texture_handle: Option<egui::TextureHandle>,
    image_id_index: Option<(String, usize, String)>,  // node id, output index, change_id
    pub position: Pos2,
    pub zoom: f32,
    is_dragging: bool,
    last_drag_position: Option<Pos2>,
    previous_cursor_primary_down: Option<bool>,
}

impl ImageViewer {
    pub fn new() -> ImageViewer {
        ImageViewer {
            image_texture_handle: None,
            image_id_index: None,
            position: Pos2::ZERO,
            zoom: 1.0,
            is_dragging: false,
            last_drag_position: None,
            previous_cursor_primary_down: None,
        }
    }

    pub fn show(&mut self, ui: &mut egui::Ui, node_id: String, output_index: usize, change_id: String, dynamic_image: &DynamicImage, cursor_position: Pos2, theme: &Theme) {
        
        let view_rect = Rect::from_min_size(
            ui.cursor().left_top(),
            ui.available_size()
        );

        ui.set_clip_rect(view_rect);

        // bg
        ui.painter().add(egui::Shape::rect_filled(
            view_rect,
            CornerRadius::ZERO,
            theme.get().grid_bg,
        ));

        self.draw_background_grid(ui, view_rect, self.position + view_rect.left_top().to_vec2(), theme);

        self.draw_image(node_id, output_index, change_id, dynamic_image, ui, view_rect);

        let view_rect_response = ui.allocate_rect(view_rect, egui::Sense::drag().union(egui::Sense::hover()));

        if view_rect_response.drag_started_by(egui::PointerButton::Primary) {
            self.start_dragging();
        } else if view_rect_response.drag_stopped_by(egui::PointerButton::Primary) {
            self.stop_dragging();
        }
        

        let cursor_primary_down: bool = ui.ctx().input(|i| i.pointer.primary_down());


        if ui.rect_contains_pointer(view_rect) {
            ui.ctx().input(|input_state| {
                // let mouse_x = cursor_position.x - editor_rect.min.x;
                // let mouse_y = cursor_position.y - editor_rect.min.y;
                //println!("{} {}, {:?}", mouse_x, mouse_y, self.position);
                let new_zoom = (self.zoom * (1.0 + input_state.smooth_scroll_delta.y * ZOOM_MULTIPLIER))
                    .min(ZOOM_BOUNDS[1])
                    .max(ZOOM_BOUNDS[0]);
    
                let old_x = view_to_graph_space(self.zoom, view_rect.max.x - view_rect.min.x);
                let new_x = view_to_graph_space(new_zoom, view_rect.max.x - view_rect.min.x);
                let old_y = view_to_graph_space(self.zoom, view_rect.max.y - view_rect.min.y);
                let new_y = view_to_graph_space(new_zoom, view_rect.max.y - view_rect.min.y);
    
                let mouse_percent_x = cursor_position.x / (view_rect.max.x - view_rect.min.x);
                let mouse_percent_y = cursor_position.y / (view_rect.max.y - view_rect.min.y);
    
                self.position.x += view_to_graph_space(
                    new_zoom,
                    mouse_percent_x * graph_to_view_space(new_zoom, new_x - old_x),
                );
                self.position.y += view_to_graph_space(
                    new_zoom,
                    mouse_percent_y * graph_to_view_space(new_zoom, new_y - old_y),
                );
    
                self.zoom = new_zoom;
            });
        }
        

        

        let cursor_inside = view_rect.contains(cursor_position);

        //let mut cursor_primary_went_down = false; // did mouse button go down this frame
        let mut cursor_primary_went_up = false; // did mous button go up this rame

        if let Some(previous_cursor_primary_down) = self.previous_cursor_primary_down {
            if previous_cursor_primary_down && !cursor_primary_down {
                cursor_primary_went_up = true;
            }
            // if !previous_cursor_primary_down && cursor_primary_down {
            //     cursor_primary_went_down = true;
            // }
        }

        // mouse
        if cursor_primary_went_up {
            self.stop_dragging();
        }

        if self.is_dragging && !cursor_inside {
            self.stop_dragging();
        }

        if self.is_dragging {
            if let Some(last_drag_position) = self.last_drag_position {
                //self.position += (cursor_position - last_drag_position) *(1.0 / self.zoom);

                self.position += view_to_graph_space_pos2(
                    self.zoom,
                    cursor_position - last_drag_position.to_vec2(),
                )
                .to_vec2();
            }

            self.last_drag_position = Some(cursor_position);
        }
    }

    fn draw_image(&mut self, node_id: String, output_index: usize, change_id: String, dynamic_image: &DynamicImage, ui: &mut egui::Ui, view_rect: Rect) {
        let needs_update = match &self.image_id_index {
            Some((image_node_id, image_output_index, image_change_id)) => {
                image_node_id != &node_id || *image_output_index != output_index || image_change_id != &change_id
            },
            None => true,
        };

        if needs_update {
            let texture_handle = self.create_egui_image(ui, dynamic_image, node_id.clone());
            self.image_texture_handle = Some(texture_handle);
            self.image_id_index = Some((node_id.clone(), output_index, change_id.clone()));
        }

        if let Some(texture_handle) = &self.image_texture_handle {
            let rect = self.get_rect(
                Pos2::new(
                    view_rect.left() + dynamic_image.width() as f32 * 0.5,
                    view_rect.top() + dynamic_image.height() as f32 * 0.5
                ),
                self.zoom,
                dynamic_image.width() as f32,
                dynamic_image.height() as f32
            );
            let uv = Rect::from_min_max(Pos2::new(0.0, 0.0), Pos2::new(1.0, 1.0));
            ui.painter().image(texture_handle.id(), rect, uv, Color32::WHITE);
        }
    }


    pub fn draw_background_grid(&self, ui: &mut egui::Ui, editor_rect: Rect, graph_position: Pos2, theme: &Theme) {
        let stroke = Stroke::new(1.0, theme.get().grid_lines);
        let grid_size: f32 = 50.0;

        let mut x = graph_to_view_space(self.zoom, graph_position.x % grid_size);
        let mut y = graph_to_view_space(self.zoom, graph_position.y % grid_size);

        while x <= editor_rect.max.x {
            ui.painter().line_segment(
                [Pos2::new(x, editor_rect.min.y), Pos2::new(x, editor_rect.max.y)],
                stroke,
            );
            x += graph_to_view_space(self.zoom, grid_size);
        }

        while y <= editor_rect.max.y {
            ui.painter().line_segment(
                [Pos2::new(editor_rect.min.x, y), Pos2::new(editor_rect.max.x, y)],
                stroke,
            );
            y += graph_to_view_space(self.zoom, grid_size);
        }
    }

    fn start_dragging(&mut self) {
        self.is_dragging = true;
    }

    fn stop_dragging(&mut self) {
        self.is_dragging = false;
        self.last_drag_position = None;
    }

    pub fn get_rect(&self, graph_position: Pos2, graph_zoom: f32, width: f32, height: f32) -> Rect {
        let node_view_pos = graph_to_view_space_pos2(graph_zoom, self.position);
        let graph_view_pos = graph_to_view_space_pos2(graph_zoom, graph_position);

        let graph_pos = Pos2::new(
            graph_view_pos.x + node_view_pos.x,
            graph_view_pos.y + node_view_pos.y,
        );
        //println!("graph pos node {:?}", graph_pos);
        //let view_pos = graph_to_view_space_pos2(graph_zoom, graph_pos);
        let view_size = graph_to_view_space_pos2(graph_zoom, Pos2::new(width, height));
        Rect::from_center_size(graph_pos, view_size.to_vec2())
    }


    fn create_egui_image(&self, ui: &mut egui::Ui, dynamic_image: &DynamicImage, name: String) -> TextureHandle {
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
}