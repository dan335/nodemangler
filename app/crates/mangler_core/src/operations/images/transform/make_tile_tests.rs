use super::*;

use crate::float_image::FloatImage;
use crate::get_id;
use crate::input::Input;
use crate::value::Value;
use std::sync::Arc;

/// Creates a test FloatImage with an x/y gradient pattern (4 channels).
fn test_image(w: u32, h: u32) -> Arc<FloatImage> {
    let mut img = FloatImage::new(w, h, 4);
    for y in 0..h {
        for x in 0..w {
            let r = x as f32 / w.max(1) as f32;
            let g = y as f32 / h.max(1) as f32;
            img.put_pixel(x, y, &[r, g, 0.5, 1.0]);
        }
    }
    Arc::new(img)
}

/// Creates a Value::Image from a test gradient image.
fn image_input(w: u32, h: u32) -> Value {
    Value::Image { data: test_image(w, h), change_id: get_id() }
}

#[tokio::test]
async fn test_make_tile_settings() {
    let s = OpImageTransformMakeTile::settings();
    assert_eq!(s.name, "make tile");
    assert_eq!(OpImageTransformMakeTile::create_inputs().len(), 2);
    assert_eq!(OpImageTransformMakeTile::create_outputs().len(), 1);
}

#[tokio::test]
async fn test_make_tile_basic() {
    let mut inputs = vec![
        Input::new("image".to_string(), image_input(16, 16), None, None),
        Input::new("blend size".to_string(), Value::Decimal(0.25), None, None),
    ];
    let result = OpImageTransformMakeTile::run(&mut inputs).await.unwrap();
    assert_eq!(result.responses.len(), 1);
    match &result.responses[0].value {
        Value::Image { data, .. } => {
            assert_eq!(data.width(), 16);
            assert_eq!(data.height(), 16);
        }
        other => panic!("Expected Image, got {:?}", other),
    }
}

#[tokio::test]
async fn test_make_tile_1x1() {
    let mut inputs = vec![
        Input::new("image".to_string(), image_input(1, 1), None, None),
        Input::new("blend size".to_string(), Value::Decimal(0.25), None, None),
    ];
    let result = OpImageTransformMakeTile::run(&mut inputs).await;
    assert!(result.is_ok(), "make_tile 1x1 failed: {:?}", result.err());
}

#[tokio::test]
async fn test_make_tile_preserves_dimensions() {
    let mut inputs = vec![
        Input::new("image".to_string(), image_input(32, 16), None, None),
        Input::new("blend size".to_string(), Value::Decimal(0.1), None, None),
    ];
    let result = OpImageTransformMakeTile::run(&mut inputs).await.unwrap();
    match &result.responses[0].value {
        Value::Image { data, .. } => {
            assert_eq!(data.width(), 32);
            assert_eq!(data.height(), 16);
        }
        other => panic!("Expected Image, got {:?}", other),
    }
}

#[tokio::test]
async fn test_make_tile_premultiplied_no_hidden_color_bleed() {
    // Left half opaque black, right half fully transparent white. The edge/corner
    // cross-fades pull in the transparent side; without premultiplied blending the
    // hidden white RGB would tint the visible boundary strips grey.
    let mut img = FloatImage::new(16, 16, 4);
    for y in 0..16 {
        for x in 0..16 {
            if x < 8 {
                img.put_pixel(x, y, &[0.0, 0.0, 0.0, 1.0]);
            } else {
                img.put_pixel(x, y, &[1.0, 1.0, 1.0, 0.0]);
            }
        }
    }
    let mut inputs = vec![
        Input::new("image".to_string(), Value::Image { data: Arc::new(img), change_id: get_id() }, None, None),
        Input::new("blend size".to_string(), Value::Decimal(0.25), None, None),
    ];
    let result = OpImageTransformMakeTile::run(&mut inputs).await.unwrap();
    let Value::Image { data, .. } = &result.responses[0].value else { panic!() };
    for (x, y, px) in data.enumerate_pixels() {
        if px[3] > 0.01 {
            assert!(
                px[0] < 0.05 && px[1] < 0.05 && px[2] < 0.05,
                "hidden colour bled into visible pixel at ({x},{y}): {px:?}"
            );
        }
    }
}

#[tokio::test]
async fn test_make_tile_uniform_image_unchanged() {
    // A uniform image tiled should remain the same uniform color
    let img = Arc::new(FloatImage::from_pixel(8, 8, 4, &[0.39, 0.59, 0.78, 1.0]));
    let mut inputs = vec![
        Input::new("image".to_string(), Value::Image { data: img, change_id: get_id() }, None, None),
        Input::new("blend size".to_string(), Value::Decimal(0.25), None, None),
    ];
    let result = OpImageTransformMakeTile::run(&mut inputs).await.unwrap();
    match &result.responses[0].value {
        Value::Image { data, .. } => {
            // Blending uniform pixels together still gives the same color
            let p = data.get_pixel(4, 4);
            assert!((p[0] - 0.39).abs() < 0.01, "uniform image should stay uniform after tiling, got r={}", p[0]);
            assert!((p[1] - 0.59).abs() < 0.01, "uniform image should stay uniform after tiling, got g={}", p[1]);
            assert!((p[2] - 0.78).abs() < 0.01, "uniform image should stay uniform after tiling, got b={}", p[2]);
        }
        other => panic!("Expected Image, got {:?}", other),
    }
}
