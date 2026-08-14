//! Tests for the blit (composite) operation.
use super::*;
use crate::float_image::FloatImage;
use crate::get_id;
use crate::input::Input;
use crate::value::Value;
use std::sync::Arc;

fn test_image(w: u32, h: u32) -> Arc<FloatImage> {
    let mut img = FloatImage::new(w, h, 4);
    for y in 0..h { for x in 0..w { img.put_pixel(x, y, &[x as f32 / w.max(1) as f32, y as f32 / h.max(1) as f32, 0.5, 1.0]); } }
    Arc::new(img)
}
fn image_input(w: u32, h: u32) -> Value { Value::Image { data: test_image(w, h), change_id: get_id() } }

/// Append the placement trio at its defaults (scale 1, rotation 0) so each test
/// lists only the inputs it actually cares about. The defaults are the identity
/// placement, so every assertion below is about the untransformed path.
fn with_placement(mut inputs: Vec<Input>) -> Vec<Input> {
    inputs.extend(placement_inputs());
    inputs
}

#[tokio::test]
async fn test_blit_settings() {
    let s = OpImageCombineBlit::settings();
    assert_eq!(s.name, "composite");
    assert_eq!(OpImageCombineBlit::create_inputs().len(), 7);
    assert_eq!(OpImageCombineBlit::create_outputs().len(), 1);
}

#[tokio::test]
async fn test_blit_1x1() {
    let bg = Value::Image { data: Arc::new(FloatImage::from_pixel(1, 1, 4, &[0.2, 0.2, 0.2, 1.0])), change_id: get_id() };
    let fg = Value::Image { data: Arc::new(FloatImage::from_pixel(1, 1, 4, &[0.8, 0.8, 0.8, 1.0])), change_id: get_id() };
    let mut inputs = with_placement(vec![
        Input::new("background".to_string(), bg, None, None),
        Input::new("foreground".to_string(), fg, None, None),
        Input::new("position x".to_string(), Value::Integer(0), None, None),
        Input::new("position y".to_string(), Value::Integer(0), None, None),
    ]);
    assert!(OpImageCombineBlit::run(&mut inputs).await.is_ok());
}

#[tokio::test]
async fn test_blit_out_of_bounds_position() {
    let mut inputs = with_placement(vec![
        Input::new("background".to_string(), image_input(4, 4), None, None),
        Input::new("foreground".to_string(), image_input(4, 4), None, None),
        Input::new("position x".to_string(), Value::Integer(100), None, None),
        Input::new("position y".to_string(), Value::Integer(100), None, None),
    ]);
    assert!(OpImageCombineBlit::run(&mut inputs).await.is_ok());
}

#[tokio::test]
async fn test_blit_preserves_background_dimensions() {
    let mut inputs = with_placement(vec![
        Input::new("background".to_string(), image_input(8, 8), None, None),
        Input::new("foreground".to_string(), image_input(4, 4), None, None),
        Input::new("position x".to_string(), Value::Integer(0), None, None),
        Input::new("position y".to_string(), Value::Integer(0), None, None),
    ]);
    let result = OpImageCombineBlit::run(&mut inputs).await.unwrap();
    match &result.responses[0].value {
        Value::Image { data, .. } => { assert_eq!(data.width(), 8); assert_eq!(data.height(), 8); }
        other => panic!("Expected Image, got {:?}", other),
    }
}

#[tokio::test]
async fn test_blit_grayscale_fg_broadcasts_to_rgb() {
    // A grayscale foreground onto an RGB background must fill all colour
    // channels (broadcast channel 0), not just red (which left a red decal).
    let bg = Value::Image { data: Arc::new(FloatImage::from_pixel(2, 2, 3, &[0.0, 0.0, 0.0])), change_id: get_id() };
    let fg = Value::Image { data: Arc::new(FloatImage::from_pixel(2, 2, 1, &[0.5])), change_id: get_id() };
    let mut inputs = with_placement(vec![
        Input::new("background".to_string(), bg, None, None),
        Input::new("foreground".to_string(), fg, None, None),
        Input::new("position x".to_string(), Value::Integer(0), None, None),
        Input::new("position y".to_string(), Value::Integer(0), None, None),
    ]);
    let result = OpImageCombineBlit::run(&mut inputs).await.unwrap();
    match &result.responses[0].value {
        Value::Image { data, .. } => {
            let p = data.get_pixel(0, 0);
            assert!((p[0] - 0.5).abs() < 1e-6, "red should be 0.5, got {}", p[0]);
            assert!((p[1] - 0.5).abs() < 1e-6, "green should be broadcast to 0.5, got {}", p[1]);
            assert!((p[2] - 0.5).abs() < 1e-6, "blue should be broadcast to 0.5, got {}", p[2]);
        }
        other => panic!("Expected Image, got {:?}", other),
    }
}

#[tokio::test]
async fn test_blit() {
    let mut inputs = with_placement(vec![
        Input::new("background".to_string(), image_input(8, 8), None, None),
        Input::new("foreground".to_string(), image_input(4, 4), None, None),
        Input::new("position x".to_string(), Value::Integer(2), None, None),
        Input::new("position y".to_string(), Value::Integer(2), None, None),
    ]);
    let result = OpImageCombineBlit::run(&mut inputs).await.unwrap();
    match &result.responses[0].value {
        Value::Image { data, .. } => { assert_eq!(data.width(), 8); assert_eq!(data.height(), 8); }
        other => panic!("Expected Image, got {:?}", other),
    }
}

// --------------------------------------------------- scaled / rotated pastes

/// The four placement inputs a transformed test overrides, in node order.
fn placed(x: i32, y: i32, sx: f32, sy: f32, rot: f32) -> Vec<Input> {
    vec![
        Input::new("position x".to_string(), Value::Integer(x), None, None),
        Input::new("position y".to_string(), Value::Integer(y), None, None),
        Input::new("scale x".to_string(), Value::Decimal(sx), None, None),
        Input::new("scale y".to_string(), Value::Decimal(sy), None, None),
        Input::new("rotation".to_string(), Value::Decimal(rot), None, None),
    ]
}

fn solid(w: u32, h: u32, v: &[f32]) -> Value {
    Value::Image { data: Arc::new(FloatImage::from_pixel(w, h, 4, v)), change_id: get_id() }
}

async fn composite(bg: Value, fg: Value, place: Vec<Input>) -> Arc<FloatImage> {
    let mut inputs = vec![
        Input::new("background".to_string(), bg, None, None),
        Input::new("foreground".to_string(), fg, None, None),
    ];
    inputs.extend(place);
    let result = OpImageCombineBlit::run(&mut inputs).await.unwrap();
    match &result.responses[0].value {
        Value::Image { data, .. } => data.clone(),
        other => panic!("Expected Image, got {other:?}"),
    }
}

#[tokio::test]
async fn scaling_covers_the_scaled_area_and_nothing_more() {
    // A 4x4 white foreground at scale 2 covers exactly 8x8 of a black canvas.
    let out = composite(
        solid(16, 16, &[0.0, 0.0, 0.0, 1.0]),
        solid(4, 4, &[1.0, 1.0, 1.0, 1.0]),
        placed(2, 3, 2.0, 2.0, 0.0),
    )
    .await;
    assert_eq!(out.dimensions(), (16, 16), "output follows the background");
    assert!(out.get_pixel(2, 3)[0] > 0.99, "top-left of the scaled paste");
    assert!(out.get_pixel(9, 10)[0] > 0.99, "bottom-right of the scaled paste");
    assert!(out.get_pixel(1, 3)[0] < 0.01, "one pixel left of it is untouched");
    assert!(out.get_pixel(10, 10)[0] < 0.01, "one pixel past it is untouched");
}

#[tokio::test]
async fn rotation_keeps_the_paste_centred_and_frees_the_corners() {
    // Rotation is about the foreground's own centre, so the middle stays put
    // while the bounding box's corners open up.
    let out = composite(
        solid(32, 32, &[0.0, 0.0, 0.0, 1.0]),
        solid(16, 16, &[1.0, 1.0, 1.0, 1.0]),
        placed(8, 8, 1.0, 1.0, 45.0),
    )
    .await;
    assert!(out.get_pixel(16, 16)[0] > 0.99, "centre still covered");
    assert!(out.get_pixel(8, 8)[0] < 0.01, "the unrotated top-left corner is now empty");
    // A 45-degree turn puts the source's top-left corner at the top middle.
    assert!(out.get_pixel(16, 5)[0] > 0.5, "the rotated tip reaches above the old edge");
}

#[tokio::test]
async fn the_untransformed_path_is_byte_identical() {
    // The whole point of routing through `placement`: defaults must not
    // resample. Compare an explicit scale-1/rotation-0 paste against one whose
    // placement inputs are the node's own defaults.
    let bg = || image_input(16, 16);
    let fg = || image_input(5, 7);
    let a = composite(bg(), fg(), placed(3, 4, 1.0, 1.0, 0.0)).await;
    let mut inputs = with_placement(vec![
        Input::new("background".to_string(), bg(), None, None),
        Input::new("foreground".to_string(), fg(), None, None),
        Input::new("position x".to_string(), Value::Integer(3), None, None),
        Input::new("position y".to_string(), Value::Integer(4), None, None),
    ]);
    let result = OpImageCombineBlit::run(&mut inputs).await.unwrap();
    let Value::Image { data: b, .. } = &result.responses[0].value else { panic!() };
    assert_eq!(a.as_raw(), b.as_raw());
}

#[tokio::test]
async fn a_placement_entirely_off_canvas_returns_the_background() {
    let out = composite(
        solid(8, 8, &[0.25, 0.25, 0.25, 1.0]),
        solid(4, 4, &[1.0, 1.0, 1.0, 1.0]),
        placed(500, 500, 1.0, 1.0, 30.0),
    )
    .await;
    assert_eq!(out.dimensions(), (8, 8));
    for y in 0..8 {
        for x in 0..8 {
            assert!((out.get_pixel(x, y)[0] - 0.25).abs() < 1e-6, "({x},{y}) was touched");
        }
    }
}

#[tokio::test]
async fn an_absurd_scale_reports_a_node_error() {
    let mut inputs = vec![
        Input::new("background".to_string(), solid(8, 8, &[0.0, 0.0, 0.0, 1.0]), None, None),
        Input::new("foreground".to_string(), image_input(4096, 4096), None, None),
    ];
    inputs.extend(placed(0, 0, 8.0, 8.0, 0.0));
    let err = OpImageCombineBlit::run(&mut inputs).await.unwrap_err();
    assert!(err.node_error.is_some(), "the size guard should surface as a node error");
    assert!(err.input_errors.is_empty());
}
