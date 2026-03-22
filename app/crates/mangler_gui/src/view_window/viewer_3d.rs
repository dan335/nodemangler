use std::sync::{Arc, Mutex};

use eframe::egui;
use epaint::{CornerRadius, Rect};
use mangler_core::float_image::FloatImage;

use crate::themes::theme::Theme;

use super::{
    arcball_camera::ArcballCamera,
    gl_renderer::{GlRenderer, TextureChannel},
    material_channels::MaterialData,
};

pub struct Viewer3d {
    renderer: Arc<Mutex<Option<GlRenderer>>>,
    camera: ArcballCamera,
    pending_uploads: Arc<Mutex<Vec<PendingUpload>>>,
}

struct PendingUpload {
    channel: TextureChannel,
    image: FloatImage,
    change_id: String,
}

impl Viewer3d {
    pub fn new() -> Self {
        Self {
            renderer: Arc::new(Mutex::new(None)),
            camera: ArcballCamera::new(),
            pending_uploads: Arc::new(Mutex::new(Vec::new())),
        }
    }

    /// Show the 3D viewer with full material data (Phase 2).
    pub fn show_material(
        &mut self,
        ui: &mut egui::Ui,
        material: &MaterialData,
        theme: &Theme,
    ) {
        let view_rect = Rect::from_min_size(ui.cursor().left_top(), ui.available_size());

        // Background
        ui.painter().add(egui::Shape::rect_filled(
            view_rect,
            CornerRadius::ZERO,
            theme.get().grid_bg,
        ));

        let response = ui.allocate_rect(view_rect, egui::Sense::drag().union(egui::Sense::hover()));

        // Camera input
        let scroll_delta = if response.hovered() {
            ui.ctx().input(|i| i.smooth_scroll_delta.y)
        } else {
            0.0
        };
        self.camera.handle_input(&response, scroll_delta);

        if response.dragged() {
            ui.ctx().request_repaint();
        }

        // Stage texture uploads for all channels
        self.stage_material_uploads(material);

        let camera_snapshot = CameraSnapshot {
            projection_fov_y: self.camera.fov_y,
            distance: self.camera.distance,
            yaw: self.camera.yaw,
            pitch: self.camera.pitch,
        };

        let renderer = self.renderer.clone();
        let pending_uploads = self.pending_uploads.clone();
        let pixels_per_point = ui.ctx().pixels_per_point();

        let min_x = view_rect.min.x * pixels_per_point;
        let max_y = view_rect.max.y * pixels_per_point;
        let width = view_rect.width() * pixels_per_point;
        let height = view_rect.height() * pixels_per_point;

        let callback = egui::PaintCallback {
            rect: view_rect,
            callback: Arc::new(egui_glow::CallbackFn::new(move |info, painter| {
                let gl = painter.gl();

                let mut renderer_guard = renderer.lock().unwrap();
                if renderer_guard.is_none() {
                    *renderer_guard = Some(GlRenderer::new(gl));
                }
                let r = renderer_guard.as_mut().unwrap();

                // Process pending texture uploads
                {
                    let mut uploads = pending_uploads.lock().unwrap();
                    for upload in uploads.drain(..) {
                        r.upload_texture(gl, upload.channel, &upload.image, &upload.change_id);
                    }
                }

                let screen_height = info.screen_size_px[1] as f32;
                let vp_x = min_x as i32;
                let vp_y = (screen_height - max_y) as i32;
                let vp_w = width as i32;
                let vp_h = height as i32;

                let camera = ArcballCamera {
                    target: glam::Vec3::ZERO,
                    distance: camera_snapshot.distance,
                    yaw: camera_snapshot.yaw,
                    pitch: camera_snapshot.pitch,
                    fov_y: camera_snapshot.projection_fov_y,
                };

                r.render(gl, [vp_x, vp_y, vp_w, vp_h], &camera);
            })),
        };

        ui.painter().add(callback);
    }

    /// Show the 3D viewer with a single image as albedo (backward compat for simple viewing).
    #[allow(dead_code)]
    pub fn show(
        &mut self,
        ui: &mut egui::Ui,
        float_image: &FloatImage,
        change_id: &str,
        theme: &Theme,
    ) {
        let material = MaterialData {
            albedo: Some((float_image.clone(), change_id.to_string())),
            normal: None,
            roughness: None,
            metallic: None,
            height: None,
            ao: None,
        };
        self.show_material(ui, &material, theme);
    }

    fn stage_material_uploads(&self, material: &MaterialData) {
        let renderer = self.renderer.lock().unwrap();
        let mut uploads = self.pending_uploads.lock().unwrap();

        let channels = [
            (TextureChannel::Albedo, &material.albedo),
            (TextureChannel::Normal, &material.normal),
            (TextureChannel::Roughness, &material.roughness),
            (TextureChannel::Metallic, &material.metallic),
            (TextureChannel::Height, &material.height),
            (TextureChannel::AmbientOcclusion, &material.ao),
        ];

        for (channel, data) in &channels {
            if let Some((image, change_id)) = data {
                let needs = if let Some(r) = renderer.as_ref() {
                    r.needs_update(*channel, change_id)
                } else {
                    true
                };
                if needs {
                    uploads.push(PendingUpload {
                        channel: *channel,
                        image: image.clone(),
                        change_id: change_id.clone(),
                    });
                }
            }
            // Note: clearing unused channels would need to happen in the callback too.
            // For now, previously uploaded textures remain until replaced.
        }
    }
}

struct CameraSnapshot {
    projection_fov_y: f32,
    distance: f32,
    yaw: f32,
    pitch: f32,
}
