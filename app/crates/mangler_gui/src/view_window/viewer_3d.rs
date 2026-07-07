use std::sync::{Arc, Mutex};

use eframe::egui;
use epaint::{CornerRadius, Rect};
use mangler_core::float_image::FloatImage;

use crate::themes::theme::Theme;

use super::{
    arcball_camera::ArcballCamera,
    gl_renderer::{GlRenderer, MeshKind, MeshResolution, RenderParams, TextureChannel},
    material_channels::MaterialData,
    viewer_settings::{light_direction, Viewer3dSettings},
};

pub struct Viewer3d {
    renderer: Arc<Mutex<Option<GlRenderer>>>,
    camera: ArcballCamera,
    pub mesh_kind: MeshKind,
    /// Tessellation level for the current mesh (paired with `mesh_kind` to key
    /// the renderer's mesh cache).
    pub mesh_resolution: MeshResolution,
    pending_uploads: Arc<Mutex<Vec<PendingUpload>>>,
}

/// A queued GL-thread action for one PBR texture slot, staged from the UI
/// thread in `stage_material_uploads` and drained inside the egui_glow paint
/// callback (where the GL context is actually available).
enum PendingUpload {
    /// Upload new image data (channel newly bound, or its change_id changed).
    Upload {
        channel: TextureChannel,
        image: FloatImage,
        change_id: String,
    },
    /// The channel's assignment was cleared in the UI (combo box set back to
    /// "None") but the renderer still holds a texture for it from a previous
    /// frame — delete it so the shader stops sampling stale data.
    Clear(TextureChannel),
}

/// Pure decision table for one material channel's staging action, factored
/// out of `stage_material_uploads` so it's unit-testable without a GL context.
#[derive(Debug, PartialEq, Eq)]
enum StagingDecision {
    /// Queue a `PendingUpload::Upload` for this channel.
    Upload,
    /// Queue a `PendingUpload::Clear` for this channel.
    Clear,
    /// The GL state already matches the material — nothing to stage.
    None,
}

/// Decide what to stage for a single channel this frame.
///
/// - `has_data`: the resolved `MaterialData` entry for the channel is `Some`.
/// - `needs_upload`: the renderer reports the change_id differs (or the
///   renderer doesn't exist yet). Only meaningful when `has_data` is true.
/// - `renderer_has_texture`: the renderer still holds a GL texture for the slot.
fn decide_staging(has_data: bool, needs_upload: bool, renderer_has_texture: bool) -> StagingDecision {
    if has_data {
        // Channel is bound: upload only when the image content changed.
        if needs_upload {
            StagingDecision::Upload
        } else {
            StagingDecision::None
        }
    } else if renderer_has_texture {
        // Channel was unbound but a stale texture is still on the GPU: clear it
        // so the shader falls back to its "no texture" default.
        StagingDecision::Clear
    } else {
        StagingDecision::None
    }
}

impl Viewer3d {
    pub fn new() -> Self {
        Self {
            renderer: Arc::new(Mutex::new(None)),
            camera: ArcballCamera::new(),
            mesh_kind: MeshKind::Sphere,
            mesh_resolution: MeshResolution::default(),
            pending_uploads: Arc::new(Mutex::new(Vec::new())),
        }
    }

    /// Show the 3D viewer with full material data (Phase 2).
    ///
    /// `settings` holds the per-leaf light/camera parameters (owned by the
    /// panel); they feed the FOV, the light direction/color and are snapshotted
    /// into the frame's `RenderParams`.
    pub fn show_material(
        &mut self,
        ui: &mut egui::Ui,
        material: &MaterialData,
        settings: &Viewer3dSettings,
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

        // Drive the camera FOV from the settings slider each frame (before the
        // snapshot below), so the perspective updates live as the slider moves.
        self.camera.fov_y = settings.fov_y_degrees.to_radians();

        // Camera input. Pan (middle drag) needs the viewport height in points to
        // scale the drag into world space.
        let scroll_delta = if response.hovered() {
            ui.ctx().input(|i| i.smooth_scroll_delta.y)
        } else {
            0.0
        };
        self.camera
            .handle_input(&response, scroll_delta, view_rect.height());

        // F-key recenters the camera, but only while this panel is hovered —
        // deliberately gated on hover so multiple 3D panels don't all reset at
        // once (unlike the 2D viewer's ungated F).
        if response.hovered() && ui.input(|i| i.key_pressed(egui::Key::F)) {
            self.camera.reset();
        }

        if response.dragged() {
            ui.ctx().request_repaint();
        }

        // Stage texture uploads for all channels
        self.stage_material_uploads(material);

        // Snapshot everything the GL callback needs into an owned RenderParams.
        // The camera is `Copy`, so this captures pan target, orbit, zoom and FOV.
        let params = RenderParams {
            camera: self.camera,
            mesh_kind: self.mesh_kind,
            mesh_resolution: self.mesh_resolution,
            light_dir: light_direction(settings.light_azimuth, settings.light_elevation),
            // Premultiply color by intensity to form the light radiance.
            light_color: glam::Vec3::from(settings.light_color) * settings.light_intensity,
            height_scale: settings.height_scale,
            env_intensity: settings.env_intensity,
            show_skybox: settings.show_skybox,
            uv_tiling: settings.uv_tiling,
            tone_map: settings.tone_map,
            wireframe: settings.wireframe,
            shadows: settings.shadows,
            ssao: settings.ssao,
            ssao_radius: settings.ssao_radius,
            ssao_intensity: settings.ssao_intensity,
            // Wireframe line color comes from the active theme (grid_lines is a
            // contrasty line color present in every theme) — never hardcoded.
            // Gamma-space RGBA to match the shader's post-gamma framebuffer write.
            wire_color: theme.get().grid_lines.to_normalized_gamma_f32(),
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

                // Process pending texture uploads/clears staged by the UI thread.
                {
                    let mut uploads = pending_uploads.lock().unwrap();
                    for upload in uploads.drain(..) {
                        match upload {
                            PendingUpload::Upload { channel, image, change_id } => {
                                r.upload_texture(gl, channel, &image, &change_id);
                            }
                            PendingUpload::Clear(channel) => {
                                r.clear_texture(gl, channel);
                            }
                        }
                    }
                }

                let screen_height = info.screen_size_px[1] as f32;
                let vp_x = min_x as i32;
                let vp_y = (screen_height - max_y) as i32;
                let vp_w = width as i32;
                let vp_h = height as i32;

                r.render(gl, [vp_x, vp_y, vp_w, vp_h], &params);
            })),
        };

        ui.painter().add(callback);
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
            (TextureChannel::Emissive, &material.emissive),
        ];

        for (channel, data) in &channels {
            // Renderer may not exist yet (created lazily in the paint callback):
            // then nothing is on the GPU and a bound channel always needs upload.
            let needs_upload = if let Some((_, change_id)) = data {
                renderer
                    .as_ref()
                    .map_or(true, |r| r.needs_update(*channel, change_id))
            } else {
                false
            };
            let renderer_has_texture = renderer
                .as_ref()
                .map_or(false, |r| r.has_texture(*channel));

            // Stale-texture fix: unbinding a channel now queues a Clear (see
            // `decide_staging` / `PendingUpload::Clear`), so previously uploaded
            // textures no longer linger on the GPU after being set to "None".
            match decide_staging(data.is_some(), needs_upload, renderer_has_texture) {
                StagingDecision::Upload => {
                    // `decide_staging` only returns Upload when `data.is_some()`.
                    let (image, change_id) = data.as_ref().unwrap();
                    uploads.push(PendingUpload::Upload {
                        channel: *channel,
                        image: image.clone(),
                        change_id: change_id.clone(),
                    });
                }
                StagingDecision::Clear => {
                    uploads.push(PendingUpload::Clear(*channel));
                }
                StagingDecision::None => {}
            }
        }
    }
}

#[cfg(test)]
#[path = "viewer_3d_tests.rs"]
mod tests;
