use eframe::egui;
use glam::{Mat4, Vec3};

const ROTATION_SPEED: f32 = 0.005;
const ZOOM_SPEED: f32 = 0.01;
const MIN_DISTANCE: f32 = 0.5;
const MAX_DISTANCE: f32 = 20.0;
const MAX_PITCH: f32 = 89.0_f32;

/// Maximum distance the orbit target may be panned away from the origin, so the
/// model can't be dragged entirely out of view.
const MAX_TARGET_OFFSET: f32 = 3.0;

/// `Copy` is trivially derivable — all fields are plain `f32`/`Vec3` — which lets
/// callers snapshot the camera by value into a paint callback without a manual
/// field copy.
#[derive(Clone, Copy)]
pub struct ArcballCamera {
    pub target: Vec3,
    pub distance: f32,
    pub yaw: f32,
    pub pitch: f32,
    pub fov_y: f32,
}

impl ArcballCamera {
    pub fn new() -> Self {
        Self {
            target: Vec3::ZERO,
            distance: 3.0,
            yaw: 0.5,
            pitch: 0.3,
            fov_y: 45.0_f32.to_radians(),
        }
    }

    pub fn eye_position(&self) -> Vec3 {
        let cos_pitch = self.pitch.cos();
        let offset = Vec3::new(
            cos_pitch * self.yaw.sin(),
            self.pitch.sin(),
            cos_pitch * self.yaw.cos(),
        );
        self.target + offset * self.distance
    }

    pub fn view_matrix(&self) -> Mat4 {
        Mat4::look_at_rh(self.eye_position(), self.target, Vec3::Y)
    }

    pub fn projection_matrix(&self, aspect: f32) -> Mat4 {
        Mat4::perspective_rh_gl(self.fov_y, aspect, 0.1, 100.0)
    }

    /// Pan the orbit `target` in the camera's screen plane so the dragged point
    /// tracks the cursor (standard DCC "content follows cursor" feel).
    ///
    /// `delta` is the pointer drag in **points** (egui screen units) and
    /// `viewport_height_px` the viewport height in the same units. The world-space
    /// distance covered by one unit of drag is `distance * tan(fov_y/2) * 2 /
    /// viewport_height_px` — i.e. the full vertical world extent visible at the
    /// target depth, divided across the viewport height. The resulting `target`
    /// offset is clamped to `MAX_TARGET_OFFSET` so the model stays reachable.
    pub fn pan(&mut self, delta: egui::Vec2, viewport_height_px: f32) {
        // Guard against a degenerate viewport (avoids a divide-by-zero).
        if viewport_height_px <= 0.0 {
            return;
        }

        // Camera basis vectors. `forward` points from the eye toward the target;
        // `right` and `up` span the screen plane.
        let forward = (self.target - self.eye_position()).normalize();
        let right = forward.cross(Vec3::Y).normalize();
        let up = right.cross(forward);

        // World units per point of drag at the target's depth.
        let scale = self.distance * (self.fov_y * 0.5).tan() * 2.0 / viewport_height_px;

        // Dragging right moves the *content* right, which means the target moves
        // left (subtract the right component); dragging down moves content down,
        // so the target moves up (add the up component for a downward +y delta).
        self.target += (-delta.x * right + delta.y * up) * scale;

        // Keep the target within a bounded region around the origin.
        if self.target.length() > MAX_TARGET_OFFSET {
            self.target = self.target.normalize() * MAX_TARGET_OFFSET;
        }
    }

    /// Restore the default orbit/zoom/target state, **preserving** the current
    /// `fov_y` (the FOV is driven separately by a UI slider and shouldn't jump
    /// back to the default on an F-key recenter).
    pub fn reset(&mut self) {
        let fov_y = self.fov_y;
        *self = Self::new();
        self.fov_y = fov_y;
    }

    /// Process drag (orbit / pan) and scroll (zoom) from an egui Response.
    ///
    /// - Primary drag orbits.
    /// - Middle drag pans (moves the target in the screen plane).
    ///
    /// `viewport_height_px` is needed to scale pan drags into world space.
    pub fn handle_input(
        &mut self,
        response: &egui::Response,
        scroll_delta: f32,
        viewport_height_px: f32,
    ) {
        // Orbit on primary drag
        if response.dragged_by(egui::PointerButton::Primary) {
            let delta = response.drag_delta();
            self.yaw -= delta.x * ROTATION_SPEED;
            self.pitch += delta.y * ROTATION_SPEED;

            let limit = MAX_PITCH.to_radians();
            self.pitch = self.pitch.clamp(-limit, limit);
        }

        // Pan on middle drag
        if response.dragged_by(egui::PointerButton::Middle) {
            self.pan(response.drag_delta(), viewport_height_px);
        }

        // Zoom on scroll
        if scroll_delta != 0.0 {
            self.distance *= 1.0 - scroll_delta * ZOOM_SPEED;
            self.distance = self.distance.clamp(MIN_DISTANCE, MAX_DISTANCE);
        }
    }
}

#[cfg(test)]
#[path = "arcball_camera_tests.rs"]
mod tests;
