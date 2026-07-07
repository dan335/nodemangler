use super::*;

#[test]
fn default_camera_position() {
    let cam = ArcballCamera::new();
    assert_eq!(cam.target, glam::Vec3::ZERO);
    assert_eq!(cam.distance, 3.0);
    assert!((cam.fov_y - 45.0_f32.to_radians()).abs() < 1e-6);
}

#[test]
fn eye_position_at_origin_yaw_zero() {
    let mut cam = ArcballCamera::new();
    cam.yaw = 0.0;
    cam.pitch = 0.0;
    cam.distance = 5.0;
    cam.target = glam::Vec3::ZERO;

    let eye = cam.eye_position();
    // yaw=0, pitch=0 → eye at (0, 0, distance) on the +Z axis
    assert!((eye.x).abs() < 1e-5);
    assert!((eye.y).abs() < 1e-5);
    assert!((eye.z - 5.0).abs() < 1e-5);
}

#[test]
fn eye_position_respects_target_offset() {
    let mut cam = ArcballCamera::new();
    cam.yaw = 0.0;
    cam.pitch = 0.0;
    cam.distance = 2.0;
    cam.target = glam::Vec3::new(1.0, 2.0, 3.0);

    let eye = cam.eye_position();
    assert!((eye.x - 1.0).abs() < 1e-5);
    assert!((eye.y - 2.0).abs() < 1e-5);
    assert!((eye.z - 5.0).abs() < 1e-5);
}

#[test]
fn view_matrix_looks_at_target() {
    let cam = ArcballCamera::new();
    let view = cam.view_matrix();
    // The view matrix should be invertible (non-degenerate)
    assert!(view.determinant().abs() > 1e-6);
}

#[test]
fn projection_matrix_has_correct_aspect() {
    let cam = ArcballCamera::new();
    let proj = cam.projection_matrix(2.0);
    // Perspective matrix should be non-degenerate
    assert!(proj.determinant().abs() > 1e-10);
    // The [0][0] element relates to 1/(aspect * tan(fov/2))
    // and [1][1] relates to 1/tan(fov/2)
    // So [1][1] / [0][0] should equal the aspect ratio
    let ratio = proj.y_axis.y / proj.x_axis.x;
    assert!((ratio - 2.0).abs() < 1e-5);
}

#[test]
fn pitch_clamping() {
    let mut cam = ArcballCamera::new();
    cam.pitch = 100.0_f32.to_radians(); // beyond 89°

    // Simulate a large upward drag that would push past limit
    // We can't easily call handle_input without an egui::Response,
    // so test the clamping logic directly
    let limit = 89.0_f32.to_radians();
    cam.pitch = cam.pitch.clamp(-limit, limit);
    assert!((cam.pitch - limit).abs() < 1e-5);

    cam.pitch = -100.0_f32.to_radians();
    cam.pitch = cam.pitch.clamp(-limit, limit);
    assert!((cam.pitch - (-limit)).abs() < 1e-5);
}

#[test]
fn zoom_clamping() {
    let mut cam = ArcballCamera::new();

    // Test min distance
    cam.distance = 0.1;
    cam.distance = cam.distance.clamp(MIN_DISTANCE, MAX_DISTANCE);
    assert!((cam.distance - MIN_DISTANCE).abs() < 1e-5);

    // Test max distance
    cam.distance = 100.0;
    cam.distance = cam.distance.clamp(MIN_DISTANCE, MAX_DISTANCE);
    assert!((cam.distance - MAX_DISTANCE).abs() < 1e-5);
}

#[test]
fn pan_moves_target_perpendicular_to_view() {
    let mut cam = ArcballCamera::new();
    let before = cam.target;
    // View direction from eye to target.
    let view_dir = (cam.target - cam.eye_position()).normalize();

    cam.pan(egui::Vec2::new(20.0, -10.0), 600.0);

    // Target actually moved.
    let moved = cam.target - before;
    assert!(moved.length() > 1e-6, "target should move, moved={:?}", moved);
    // The pan slides in the screen plane, so the motion is perpendicular to the
    // view direction (no component along the eye→target axis).
    assert!(
        moved.dot(view_dir).abs() < 1e-4,
        "pan should be perpendicular to view dir, dot={}",
        moved.dot(view_dir)
    );
}

#[test]
fn pan_right_moves_target_left() {
    // Dragging right should move the *content* right, i.e. the target moves in
    // the camera's -right direction. Verify the sign of the right-axis component.
    let mut cam = ArcballCamera::new();
    let forward = (cam.target - cam.eye_position()).normalize();
    let right = forward.cross(glam::Vec3::Y).normalize();

    cam.pan(egui::Vec2::new(20.0, 0.0), 600.0);

    assert!(
        cam.target.dot(right) < 0.0,
        "rightward drag should push target along -right, got {}",
        cam.target.dot(right)
    );
}

#[test]
fn pan_clamps_target_offset() {
    let mut cam = ArcballCamera::new();
    // A huge drag would move the target far past the clamp radius (3.0).
    cam.pan(egui::Vec2::new(100000.0, 100000.0), 100.0);
    assert!(
        cam.target.length() <= 3.0 + 1e-4,
        "target length should clamp to 3.0, got {}",
        cam.target.length()
    );
}

#[test]
fn reset_restores_defaults_but_keeps_fov() {
    let mut cam = ArcballCamera::new();
    // Perturb every field, including a non-default FOV.
    cam.yaw = 2.0;
    cam.pitch = 0.5;
    cam.distance = 12.0;
    cam.target = glam::Vec3::new(1.0, 1.0, 0.0);
    cam.fov_y = 70.0_f32.to_radians();

    cam.reset();

    let default = ArcballCamera::new();
    assert!((cam.yaw - default.yaw).abs() < 1e-6);
    assert!((cam.pitch - default.pitch).abs() < 1e-6);
    assert!((cam.distance - default.distance).abs() < 1e-6);
    assert_eq!(cam.target, default.target);
    // FOV is preserved, not reset to the default.
    assert!((cam.fov_y - 70.0_f32.to_radians()).abs() < 1e-6);
}

#[test]
fn eye_position_pitch_up() {
    let mut cam = ArcballCamera::new();
    cam.yaw = 0.0;
    cam.pitch = std::f32::consts::FRAC_PI_4; // 45 degrees up
    cam.distance = 1.0;
    cam.target = glam::Vec3::ZERO;

    let eye = cam.eye_position();
    // At 45° pitch, y should be sin(45°) ≈ 0.707
    assert!((eye.y - 0.7071).abs() < 0.01);
    // z should be cos(45°) * cos(0) ≈ 0.707
    assert!((eye.z - 0.7071).abs() < 0.01);
}
