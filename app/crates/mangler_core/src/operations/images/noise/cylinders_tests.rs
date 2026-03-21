use super::*;

use crate::input::Input;
use crate::value::Value;

fn make_inputs(width: i32, height: i32, frequency: f32) -> Vec<Input> {
    vec![
        Input::new("width".to_string(), Value::Integer(width), None, None),
        Input::new("height".to_string(), Value::Integer(height), None, None),
        Input::new("frequency".to_string(), Value::Decimal(frequency), None, None),
    ]
}

#[tokio::test]
async fn test_settings() {
    let s = OpImageNoiseCylinders::settings();
    assert_eq!(s.name, "concentric rings");
    assert_eq!(OpImageNoiseCylinders::create_inputs().len(), 3);
    assert_eq!(OpImageNoiseCylinders::create_outputs().len(), 1);
}

#[tokio::test]
async fn test_run_basic() {
    let mut inputs = make_inputs(16, 16, 2.0);
    let result = OpImageNoiseCylinders::run(&mut inputs).await;
    assert!(result.is_ok(), "run failed: {:?}", result.err());
    match &result.unwrap().responses[0].value {
        Value::DynamicImage { .. } => {}
        other => panic!("Expected DynamicImage, got {:?}", other),
    }
}

#[tokio::test]
async fn test_correct_dimensions() {
    let mut inputs = make_inputs(32, 16, 2.0);
    let result = OpImageNoiseCylinders::run(&mut inputs).await.unwrap();
    match &result.responses[0].value {
        Value::DynamicImage { data, .. } => {
            assert_eq!(data.width(), 32);
            assert_eq!(data.height(), 16);
        }
        other => panic!("Expected DynamicImage, got {:?}", other),
    }
}

#[tokio::test]
async fn test_1x1() {
    let mut inputs = make_inputs(1, 1, 1.0);
    let result = OpImageNoiseCylinders::run(&mut inputs).await;
    assert!(result.is_ok(), "1x1 cylinders failed: {:?}", result.err());
}

#[tokio::test]
async fn test_deterministic() {
    let r1 = OpImageNoiseCylinders::run(&mut make_inputs(32, 32, 3.0)).await.unwrap();
    let r2 = OpImageNoiseCylinders::run(&mut make_inputs(32, 32, 3.0)).await.unwrap();
    match (&r1.responses[0].value, &r2.responses[0].value) {
        (Value::DynamicImage { data: d1, .. }, Value::DynamicImage { data: d2, .. }) => {
            assert_eq!(d1.to_luma8().pixels().collect::<Vec<_>>(),
                       d2.to_luma8().pixels().collect::<Vec<_>>(),
                       "cylinders noise is not deterministic");
        }
        _ => panic!("Expected DynamicImage"),
    }
}

#[tokio::test]
async fn test_tiles_seamlessly() {
    // Verify that the seam between adjacent tiles is smooth.
    // Pixels at x=0 and x=w-1 are adjacent across the tile boundary, so they
    // won't be identical but should be close. Use a large image with low frequency
    // to keep per-pixel gradients small.
    let size = 256;
    let r = OpImageNoiseCylinders::run(&mut make_inputs(size, size, 2.0)).await.unwrap();
    match &r.responses[0].value {
        Value::DynamicImage { data, .. } => {
            let buf = data.to_luma8();
            let w = buf.width();
            let h = buf.height();
            // Horizontal seam: compare left edge to right edge (adjacent across wrap)
            for y in 0..h {
                let left = buf.get_pixel(0, y)[0];
                let right = buf.get_pixel(w - 1, y)[0];
                assert!((left as i16 - right as i16).unsigned_abs() <= 8,
                        "horizontal seam mismatch at y={}: left={}, right={}", y, left, right);
            }
            // Vertical seam: compare top edge to bottom edge (adjacent across wrap)
            for x in 0..w {
                let top = buf.get_pixel(x, 0)[0];
                let bottom = buf.get_pixel(x, h - 1)[0];
                assert!((top as i16 - bottom as i16).unsigned_abs() <= 8,
                        "vertical seam mismatch at x={}: top={}, bottom={}", x, top, bottom);
            }
        }
        _ => panic!("Expected DynamicImage"),
    }
}

#[tokio::test]
async fn test_frequency_affects_output() {
    let r1 = OpImageNoiseCylinders::run(&mut make_inputs(32, 32, 1.0)).await.unwrap();
    let r2 = OpImageNoiseCylinders::run(&mut make_inputs(32, 32, 5.0)).await.unwrap();
    match (&r1.responses[0].value, &r2.responses[0].value) {
        (Value::DynamicImage { data: d1, .. }, Value::DynamicImage { data: d2, .. }) => {
            assert_ne!(d1.to_luma8().pixels().collect::<Vec<_>>(),
                       d2.to_luma8().pixels().collect::<Vec<_>>(),
                       "different frequencies should produce different output");
        }
        _ => panic!("Expected DynamicImage"),
    }
}
