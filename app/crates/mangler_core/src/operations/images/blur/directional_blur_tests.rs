use super::*;

use crate::curve::{Curve, CurveInterpolation};
use crate::float_image::FloatImage;
use crate::get_id;
use crate::input::Input;
use crate::operations::scale_to_resolution;
use crate::value::Value;
use std::sync::Arc;

/// Creates a gradient test image as a 4-channel FloatImage.
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

/// Wraps a test image as a `Value::Image`.
fn image_input(w: u32, h: u32) -> Value {
    Value::Image { data: test_image(w, h), change_id: get_id() }
}

/// Wraps a curve as the node's `path` input.
fn path_input(curve: Curve) -> Input {
    Input::new("path".to_string(), Value::Curve(curve), None, None)
}

/// The untouched default path input (angle mode).
fn default_path_input() -> Input {
    path_input(Curve::default())
}

#[tokio::test]
async fn test_directional_blur_settings() {
    let s = OpImageAdjustmentDirectionalBlur::settings();
    assert_eq!(s.name, "directional blur");
    assert_eq!(OpImageAdjustmentDirectionalBlur::create_inputs().len(), 5);
    assert_eq!(OpImageAdjustmentDirectionalBlur::create_outputs().len(), 1);
}

#[tokio::test]
async fn test_directional_blur_basic() {
    let mut inputs = vec![
        Input::new("image".to_string(), image_input(8, 8), None, None),
        Input::new("angle".to_string(), Value::Decimal(45.0), None, None),
        Input::new("samples".to_string(), Value::Integer(8), None, None),
        Input::new("intensity".to_string(), Value::Decimal(5.0), None, None),
        default_path_input(),
    ];
    let result = OpImageAdjustmentDirectionalBlur::run(&mut inputs).await.unwrap();
    match &result.responses[0].value {
        Value::Image { data, .. } => {
            assert_eq!(data.width(), 8);
            assert_eq!(data.height(), 8);
        }
        other => panic!("Expected Image, got {:?}", other),
    }
}

#[tokio::test]
async fn test_directional_blur_1x1() {
    let img = Arc::new(FloatImage::from_pixel(1, 1, 4, &[0.78, 0.39, 0.20, 1.0]));
    let mut inputs = vec![
        Input::new("image".to_string(), Value::Image { data: img, change_id: get_id() }, None, None),
        Input::new("angle".to_string(), Value::Decimal(45.0), None, None),
        Input::new("samples".to_string(), Value::Integer(4), None, None),
        Input::new("intensity".to_string(), Value::Decimal(5.0), None, None),
        default_path_input(),
    ];
    let result = OpImageAdjustmentDirectionalBlur::run(&mut inputs).await;
    assert!(result.is_ok(), "directional_blur 1x1 failed: {:?}", result.err());
}

#[tokio::test]
async fn test_directional_blur_uniform_image_unchanged() {
    // Blurring a uniform image should not change pixel values
    let uniform = Arc::new(FloatImage::from_pixel(8, 8, 4, &[0.39, 0.39, 0.39, 1.0]));
    let mut inputs = vec![
        Input::new("image".to_string(), Value::Image { data: uniform, change_id: get_id() }, None, None),
        Input::new("angle".to_string(), Value::Decimal(90.0), None, None),
        Input::new("samples".to_string(), Value::Integer(8), None, None),
        Input::new("intensity".to_string(), Value::Decimal(10.0), None, None),
        default_path_input(),
    ];
    let result = OpImageAdjustmentDirectionalBlur::run(&mut inputs).await.unwrap();
    match &result.responses[0].value {
        Value::Image { data, .. } => {
            let px = data.get_pixel(4, 4);
            assert!((px[0] - 0.39).abs() < 0.02, "uniform image should be unchanged, got {}", px[0]);
        }
        other => panic!("Expected Image, got {:?}", other),
    }
}

#[tokio::test]
async fn test_directional_blur_zero_intensity() {
    let mut inputs = vec![
        Input::new("image".to_string(), image_input(4, 4), None, None),
        Input::new("angle".to_string(), Value::Decimal(0.0), None, None),
        Input::new("samples".to_string(), Value::Integer(4), None, None),
        Input::new("intensity".to_string(), Value::Decimal(0.0), None, None),
        default_path_input(),
    ];
    let result = OpImageAdjustmentDirectionalBlur::run(&mut inputs).await.unwrap();
    match &result.responses[0].value {
        Value::Image { .. } => {}
        other => panic!("Expected Image, got {:?}", other),
    }
}

/// Runs the node and returns the output image.
async fn run_blur(image: Value, angle: f32, samples: i32, intensity: f32, path: Curve) -> Arc<FloatImage> {
    let mut inputs = vec![
        Input::new("image".to_string(), image, None, None),
        Input::new("angle".to_string(), Value::Decimal(angle), None, None),
        Input::new("samples".to_string(), Value::Integer(samples), None, None),
        Input::new("intensity".to_string(), Value::Decimal(intensity), None, None),
        path_input(path),
    ];
    let result = OpImageAdjustmentDirectionalBlur::run(&mut inputs).await.unwrap();
    match &result.responses[0].value {
        Value::Image { data, .. } => data.clone(),
        other => panic!("Expected Image, got {:?}", other),
    }
}

#[tokio::test]
async fn test_directional_blur_default_path_uses_angle_mode() {
    // With the untouched default path, the node must reproduce the classic
    // angle-mode math exactly: taps at t*intensity along (cos a, sin a).
    let (w, h) = (8u32, 8u32);
    let (angle, samples, intensity) = (30.0f32, 5i32, 6.0f32);
    let img = test_image(w, h);
    let out = run_blur(
        Value::Image { data: img.clone(), change_id: get_id() },
        angle, samples, intensity, Curve::default(),
    ).await;

    // Reference: the pre-path angle-mode loop, replicated verbatim.
    let intensity_px = scale_to_resolution(intensity, w, h);
    let rad = angle.to_radians();
    let (dx, dy) = (rad.cos(), rad.sin());
    let mut sample = vec![0.0f32; 4];
    for y in 0..h {
        for x in 0..w {
            let mut sums = [0.0f64; 4];
            for i in 0..samples {
                let t = (i as f32 / (samples - 1) as f32) * 2.0 - 1.0;
                let offset = t * intensity_px;
                img.bilinear_sample(x as f32 + dx * offset, y as f32 + dy * offset, &mut sample);
                for c in 0..4 {
                    sums[c] += sample[c] as f64;
                }
            }
            let got = out.get_pixel(x, y);
            for c in 0..4 {
                let expected = (sums[c] / samples as f64) as f32;
                assert!(
                    (got[c] - expected).abs() < 1e-6,
                    "default path should match angle mode at ({x},{y}) ch{c}: got {}, expected {expected}",
                    got[c]
                );
            }
        }
    }
}

#[tokio::test]
async fn test_directional_blur_vertical_path_matches_angle_90() {
    // A straight vertical 2-point path is arc-length-sampled into exactly the
    // same tap positions as a 90-degree angle line, so the results must agree
    // (up to float noise in cos/sin at 90 degrees).
    let (w, h) = (8u32, 8u32);
    let vertical = Curve {
        points: vec![[0.5, 0.2], [0.5, 0.8]],
        closed: false,
        interpolation: CurveInterpolation::Linear,
        handles: Vec::new(),
    };
    let img = image_input(w, h);
    // Intensity is authored at a 1024px reference; 256 scales to a 2px
    // half-length on this 8px image, a real smear rather than sub-pixel noise.
    // Angle 0 on the path run proves the angle input is ignored in path mode.
    let path_out = run_blur(img.clone(), 0.0, 7, 256.0, vertical).await;
    let angle_out = run_blur(img, 90.0, 7, 256.0, Curve::default()).await;
    for y in 0..h {
        // Skip the x = 0 column: angle mode's cos(90°) is ~-7e-8 rather than
        // exactly 0, and bilinear clamping is discontinuous at the left edge
        // (floor(-epsilon) = -1 pushes the tap onto pixel column 1), so that
        // column diverges by design; the path mode's exact 0.0 x-offsets are
        // the cleaner behaviour.
        for x in 1..w {
            let a = path_out.get_pixel(x, y);
            let b = angle_out.get_pixel(x, y);
            for c in 0..4 {
                assert!(
                    (a[c] - b[c]).abs() < 1e-4,
                    "vertical path should match angle 90 at ({x},{y}) ch{c}: {} vs {}",
                    a[c], b[c]
                );
            }
        }
    }
}

#[tokio::test]
async fn test_directional_blur_degenerate_path_falls_back_to_angle_mode() {
    // A drawn-but-degenerate path (two coincident points, zero arc length)
    // must fall back to angle mode bit-exactly.
    let degenerate = Curve {
        points: vec![[0.3, 0.7], [0.3, 0.7]],
        closed: false,
        interpolation: CurveInterpolation::Linear,
        handles: Vec::new(),
    };
    let (w, h) = (6u32, 6u32);
    let img = image_input(w, h);
    let degen_out = run_blur(img.clone(), 45.0, 6, 8.0, degenerate).await;
    let angle_out = run_blur(img, 45.0, 6, 8.0, Curve::default()).await;
    for y in 0..h {
        for x in 0..w {
            let a = degen_out.get_pixel(x, y);
            let b = angle_out.get_pixel(x, y);
            assert_eq!(&a[..], &b[..], "degenerate path should equal angle mode at ({x},{y})");
        }
    }
}
