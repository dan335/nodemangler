use super::*;

use crate::input::Input;
use crate::value::Value;


#[tokio::test]
async fn test_opimageshapescircle_settings() {
    let s = OpImageShapesCircle::settings();
    assert_eq!(s.name, "circle");
    assert_eq!(OpImageShapesCircle::create_inputs().len(), 5);
    assert_eq!(OpImageShapesCircle::create_outputs().len(), 1);
}

/// Builds the input vector in create_inputs() order: width, height, radius,
/// center_x, center_y.
fn inputs(width: i32, height: i32, radius: f32, cx: f32, cy: f32) -> Vec<Input> {
    vec![
        Input::new("width".to_string(), Value::Integer(width), None, None),
        Input::new("height".to_string(), Value::Integer(height), None, None),
        Input::new("radius".to_string(), Value::Decimal(radius), None, None),
        Input::new("center_x".to_string(), Value::Decimal(cx), None, None),
        Input::new("center_y".to_string(), Value::Decimal(cy), None, None),
    ]
}

#[tokio::test]
async fn test_opimageshapescircle_run() {
    let mut inputs = inputs(64, 64, 0.5, 0.0, 0.0);
    let result = OpImageShapesCircle::run(&mut inputs).await;
    assert!(result.is_ok(), "run failed: {:?}", result.err());

    let Value::Image { data, .. } = &result.unwrap().responses[0].value else {
        panic!("expected an image output");
    };
    assert_eq!(data.dimensions(), (64, 64));
    assert_eq!(data.channels(), 1, "shape masks are single-channel");

    // Center is inside the disc (white); a corner is well outside (black).
    assert!(data.get_pixel(32, 32)[0] > 0.99, "center should be filled");
    assert!(data.get_pixel(0, 0)[0] < 0.01, "corner should be empty");
}

#[tokio::test]
async fn test_opimageshapescircle_stays_round_on_wide_canvas() {
    // radius 1.0 spans half the *shorter* dimension (32 px here). On a 128x64
    // canvas the disc must remain round: a point 40 px right of center is
    // outside, while a point 40 px right would be *inside* a naive ellipse that
    // stretched the radius to half the width (64 px).
    let mut inputs = inputs(128, 64, 1.0, 0.0, 0.0);
    let data = match OpImageShapesCircle::run(&mut inputs).await.unwrap().responses.remove(0).value {
        Value::Image { data, .. } => data,
        other => panic!("expected image, got {other:?}"),
    };
    // Center (64, 32) filled; (104, 32) is 40 px out horizontally -> outside.
    assert!(data.get_pixel(64, 32)[0] > 0.99, "center filled");
    assert!(data.get_pixel(104, 32)[0] < 0.01, "40px horizontal is outside a round disc of radius 32");
}
