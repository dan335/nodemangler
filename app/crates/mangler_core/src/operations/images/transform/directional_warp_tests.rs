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

/// Creates a horizontal gradient intensity map (4 channels, grayscale).
fn gradient_h_image(w: u32, h: u32) -> Value {
    let mut img = FloatImage::new(w, h, 4);
    for y in 0..h {
        for x in 0..w {
            let v = x as f32 / w.max(1) as f32;
            img.put_pixel(x, y, &[v, v, v, 1.0]);
        }
    }
    Value::Image { data: Arc::new(img), change_id: get_id() }
}

#[tokio::test]
async fn test_directional_warp_settings() {
    let s = OpImageTransformDirectionalWarp::settings();
    assert_eq!(s.name, "directional warp");
    assert_eq!(OpImageTransformDirectionalWarp::create_inputs().len(), 4);
    assert_eq!(OpImageTransformDirectionalWarp::create_outputs().len(), 1);
}

#[tokio::test]
async fn test_directional_warp_basic() {
    let mut inputs = vec![
        Input::new("image".to_string(), image_input(16, 16), None, None),
        Input::new("intensity map".to_string(), gradient_h_image(16, 16), None, None),
        Input::new("angle".to_string(), Value::Decimal(0.0), None, None),
        Input::new("intensity".to_string(), Value::Decimal(5.0), None, None),
    ];
    let result = OpImageTransformDirectionalWarp::run(&mut inputs).await.unwrap();
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
async fn test_directional_warp_1x1() {
    let mut inputs = vec![
        Input::new("image".to_string(), image_input(1, 1), None, None),
        Input::new("intensity map".to_string(), image_input(1, 1), None, None),
        Input::new("angle".to_string(), Value::Decimal(0.0), None, None),
        Input::new("intensity".to_string(), Value::Decimal(5.0), None, None),
    ];
    let result = OpImageTransformDirectionalWarp::run(&mut inputs).await;
    assert!(result.is_ok(), "directional_warp 1x1 failed: {:?}", result.err());
}

#[tokio::test]
async fn test_directional_warp_zero_intensity_passthrough() {
    // With intensity=0, all displacements are 0 -> output should equal input
    let img = Arc::new(FloatImage::from_pixel(8, 8, 4, &[0.59, 0.78, 0.20, 1.0]));
    let map = Arc::new(FloatImage::from_pixel(8, 8, 4, &[0.5, 0.5, 0.5, 1.0]));
    let mut inputs = vec![
        Input::new("image".to_string(), Value::Image { data: img, change_id: get_id() }, None, None),
        Input::new("intensity map".to_string(), Value::Image { data: map, change_id: get_id() }, None, None),
        Input::new("angle".to_string(), Value::Decimal(45.0), None, None),
        Input::new("intensity".to_string(), Value::Decimal(0.0), None, None),
    ];
    let result = OpImageTransformDirectionalWarp::run(&mut inputs).await.unwrap();
    match &result.responses[0].value {
        Value::Image { data, .. } => {
            let p = data.get_pixel(4, 4);
            // With zero intensity, output should match the uniform input pixel
            assert!((p[0] - 0.59).abs() < 0.01, "zero intensity should give passthrough, got r={}", p[0]);
            assert!((p[1] - 0.78).abs() < 0.01, "zero intensity should give passthrough, got g={}", p[1]);
        }
        other => panic!("Expected Image, got {:?}", other),
    }
}

#[tokio::test]
async fn test_directional_warp_different_angles() {
    // Run at multiple angles to ensure no panics
    for angle in [0.0, 45.0, 90.0, 180.0, 270.0, 360.0] {
        let mut inputs = vec![
            Input::new("image".to_string(), image_input(8, 8), None, None),
            Input::new("intensity map".to_string(), gradient_h_image(8, 8), None, None),
            Input::new("angle".to_string(), Value::Decimal(angle), None, None),
            Input::new("intensity".to_string(), Value::Decimal(5.0), None, None),
        ];
        let result = OpImageTransformDirectionalWarp::run(&mut inputs).await;
        assert!(result.is_ok(), "directional_warp at angle {} failed: {:?}", angle, result.err());
    }
}
