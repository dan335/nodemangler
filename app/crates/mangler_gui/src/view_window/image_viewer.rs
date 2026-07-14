use eframe::egui::{self};
use epaint::{TextureHandle, Pos2, CornerRadius, Rect, Stroke, Color32};
use mangler_core::float_image::FloatImage;

use crate::{
    graph_to_view_space, graph_to_view_space_pos2,
    pan_zoom::{self, PanZoomController},
    themes::theme::Theme,
};

pub struct ImageViewer {
    image_texture_handle: Option<egui::TextureHandle>,
    image_id_index: Option<(String, usize, String)>,  // node id, output index, change_id
    pub position: Pos2,
    pub zoom: f32,
    /// Shared drag-to-pan state machine (same controller as the graph editor).
    pan_zoom: PanZoomController,
    /// Last consumed value of `show`'s `fit_seq` — a viewer fits once per new
    /// sequence value, so one bump refits every 2D panel exactly once.
    last_fit_seq: u64,
}

impl ImageViewer {
    pub fn new() -> ImageViewer {
        ImageViewer {
            image_texture_handle: None,
            image_id_index: None,
            position: Pos2::ZERO,
            zoom: 1.0,
            pan_zoom: PanZoomController::new(),
            last_fit_seq: 0,
        }
    }

    /// Renders the image viewer panel with pan/zoom controls.
    ///
    /// When `fit_on_change` is true, the view auto-fits (as if F were pressed)
    /// the first frame a new image appears — used for library image previews,
    /// where each freshly-opened image should frame itself. Node-output views
    /// pass false, since their `change_id` changes every graph run and fitting
    /// would fight the user's pan/zoom.
    ///
    /// `fit_seq` is a fit *request* counter (owned by `Program`, bumped when
    /// the user picks something to view, e.g. right-clicks a node output): any
    /// value different from the last one this viewer consumed triggers a fit,
    /// so an explicit "view this" always centers and frames the image even if
    /// the same output was already showing.
    pub fn show(&mut self, ui: &mut egui::Ui, node_id: String, output_index: usize, change_id: String, float_image: &FloatImage, fit_on_change: bool, fit_seq: u64, theme: &Theme) {

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

        // Auto-fit a newly-loaded image before drawing, so its first frame is
        // already framed. Detected against the texture cache key, which
        // `draw_image` updates below.
        if fit_on_change {
            let changed = match &self.image_id_index {
                Some((n, o, c)) => n != &node_id || *o != output_index || c != &change_id,
                None => true,
            };
            if changed {
                self.fit_to_view(view_rect, float_image.width() as f32, float_image.height() as f32);
            }
        }

        // Explicit fit request (e.g. right-clicked a node output to view it):
        // consume each new sequence value with one fit.
        if fit_seq != self.last_fit_seq {
            self.last_fit_seq = fit_seq;
            self.fit_to_view(view_rect, float_image.width() as f32, float_image.height() as f32);
        }

        self.draw_image(node_id, output_index, change_id, float_image, ui, view_rect, theme);

        let view_rect_response = ui.allocate_rect(view_rect, egui::Sense::drag().union(egui::Sense::hover()));

        if view_rect_response.drag_started_by(egui::PointerButton::Primary) {
            self.pan_zoom.start_dragging();
        } else if view_rect_response.drag_stopped_by(egui::PointerButton::Primary) {
            self.pan_zoom.stop_dragging();
        }

        // Pointer state from this ui's own (per-viewport) input, so previews
        // hosted in secondary OS windows track their window's pointer rather
        // than the main window's.
        let cursor_position = pan_zoom::viewport_cursor(ui);
        let cursor_primary_down: bool = ui.ctx().input(|i| i.pointer.primary_down());

        // Fit image to view on F key
        if ui.ctx().input(|i| i.key_pressed(egui::Key::F)) {
            self.fit_to_view(view_rect, float_image.width() as f32, float_image.height() as f32);
        }

        // Scroll-to-zoom about the cursor (shared with the graph editor, but
        // with the wider image bounds so large images can zoom out to fit).
        if view_rect.contains(cursor_position) {
            pan_zoom::zoom_about_cursor(
                ui,
                &mut self.position,
                &mut self.zoom,
                cursor_position,
                pan_zoom::IMAGE_ZOOM_BOUNDS,
            );
        }

        // Drag-to-pan (shared state machine with the graph editor).
        self.pan_zoom.update(
            &mut self.position,
            self.zoom,
            cursor_position,
            view_rect.contains(cursor_position),
            cursor_primary_down,
        );
    }

    /// Draws the image on the canvas, uploading a new GPU texture when the image changes.
    fn draw_image(&mut self, node_id: String, output_index: usize, change_id: String, float_image: &FloatImage, ui: &mut egui::Ui, view_rect: Rect, theme: &Theme) {
        let needs_update = match &self.image_id_index {
            Some((image_node_id, image_output_index, image_change_id)) => {
                image_node_id != &node_id || *image_output_index != output_index || image_change_id != &change_id
            },
            None => true,
        };

        if needs_update {
            let texture_handle = self.create_egui_image(ui, float_image, node_id.clone());
            self.image_texture_handle = Some(texture_handle);
            self.image_id_index = Some((node_id.clone(), output_index, change_id.clone()));
        }

        if let Some(texture_handle) = &self.image_texture_handle {
            let rect = self.get_rect(
                Pos2::new(
                    view_rect.left() + float_image.width() as f32 * 0.5,
                    view_rect.top() + float_image.height() as f32 * 0.5
                ),
                self.zoom,
                float_image.width() as f32,
                float_image.height() as f32
            );
            let uv = Rect::from_min_max(Pos2::new(0.0, 0.0), Pos2::new(1.0, 1.0));
            ui.painter().image(texture_handle.id(), rect, uv, Color32::WHITE);
            // Outline the image so its bounds stay visible against the grid
            // (dark images otherwise blend into the background — and the curve
            // overlay needs the [0,1]² extent to be legible).
            ui.painter().rect_stroke(
                rect,
                CornerRadius::ZERO,
                Stroke::new(1.0, theme.get().text_faint),
                epaint::StrokeKind::Outside,
            );
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

    /// Resets position and zoom so the entire image fills the view.
    fn fit_to_view(&mut self, view_rect: Rect, img_width: f32, img_height: f32) {
        let view_width = view_rect.width();
        let view_height = view_rect.height();

        // Larger zoom = smaller on screen (graph_to_view_space divides by zoom),
        // so pick the axis where the image is most oversized relative to the view.
        let zoom = (img_width / view_width)
            .max(img_height / view_height)
            .clamp(pan_zoom::IMAGE_ZOOM_BOUNDS[0], pan_zoom::IMAGE_ZOOM_BOUNDS[1]);

        // Center the image: screen center = (graph_position + position) / zoom
        // graph_position used in draw_image = (view_rect.left() + w/2, view_rect.top() + h/2)
        let center_x = view_rect.center().x * zoom - view_rect.left() - img_width / 2.0;
        let center_y = view_rect.center().y * zoom - view_rect.top() - img_height / 2.0;

        self.zoom = zoom;
        self.position = Pos2::new(center_x, center_y);
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


    /// Converts a FloatImage to an egui texture for GPU display.
    fn create_egui_image(&self, ui: &mut egui::Ui, float_image: &FloatImage, name: String) -> TextureHandle {
        // Convert the internal f32 buffer to an 8-bit RGBA image for GPU upload
        let rgba_image = float_image.to_rgba8();

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