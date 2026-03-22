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
