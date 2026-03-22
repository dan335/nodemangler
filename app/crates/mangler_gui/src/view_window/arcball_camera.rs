use eframe::egui;
use glam::{Mat4, Vec3};

const ROTATION_SPEED: f32 = 0.005;
const ZOOM_SPEED: f32 = 0.01;
const MIN_DISTANCE: f32 = 0.5;
const MAX_DISTANCE: f32 = 20.0;
const MAX_PITCH: f32 = 89.0_f32;

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

    /// Process drag (orbit) and scroll (zoom) from an egui Response.
    pub fn handle_input(&mut self, response: &egui::Response, scroll_delta: f32) {
        // Orbit on primary drag
        if response.dragged_by(egui::PointerButton::Primary) {
            let delta = response.drag_delta();
            self.yaw -= delta.x * ROTATION_SPEED;
            self.pitch += delta.y * ROTATION_SPEED;

            let limit = MAX_PITCH.to_radians();
            self.pitch = self.pitch.clamp(-limit, limit);
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
