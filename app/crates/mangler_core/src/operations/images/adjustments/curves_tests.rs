//! Tests for the curves adjustment operation (spline tone curve).

use super::*;

use crate::curve::CurveInterpolation;
use crate::float_image::FloatImage;
use crate::get_id;
use crate::input::Input;
use crate::value::Value;
use std::sync::Arc;

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

fn image_input(w: u32, h: u32) -> Value {
    Value::Image { data: test_image(w, h), change_id: get_id() }
}

/// Build the op's inputs with a specific master curve; the per-channel curves
/// stay at their identity defaults.
fn inputs_with_curve(image: Value, curve: Curve) -> Vec<Input> {
    inputs_with_curves(image, curve, identity(), identity(), identity())
}

/// The untouched default for every curve input.
fn identity() -> Curve {
    OpImageAdjustmentCurves::identity_curve()
}

/// Build the op's inputs with explicit master / red / green / blue curves.
fn inputs_with_curves(image: Value, master: Curve, red: Curve, green: Curve, blue: Curve) -> Vec<Input> {
    vec![
        Input::new("image".to_string(), image, None, None),
        Input::new("curve".to_string(), Value::Curve(master), None, None),
        Input::new("red".to_string(), Value::Curve(red), None, None),
        Input::new("green".to_string(), Value::Curve(green), None, None),
        Input::new("blue".to_string(), Value::Curve(blue), None, None),
    ]
}

/// Runs the op and unwraps the image output.
async fn run_image(mut inputs: Vec<Input>) -> Arc<FloatImage> {
    let result = OpImageAdjustmentCurves::run(&mut inputs).await.unwrap();
    let Value::Image { data, .. } = &result.responses[0].value else { panic!("expected image") };
    data.clone()
}

/// A linear tone curve through the given (input, output) anchor points.
/// Converts output values to y-down curve coordinates (`y = 1 - output`).
fn linear_tone_curve(anchors: &[(f32, f32)]) -> Curve {
    Curve {
        points: anchors.iter().map(|(x, out)| [*x, 1.0 - *out]).collect(),
        closed: false,
        interpolation: CurveInterpolation::Linear,
        handles: Vec::new(),
    }
}

#[tokio::test]
async fn test_curves_settings() {
    let s = OpImageAdjustmentCurves::settings();
    assert_eq!(s.name, "curves");
    assert_eq!(OpImageAdjustmentCurves::create_inputs().len(), 5);
    // image, master curve, then red/green/blue in that order.
    let names: Vec<String> = OpImageAdjustmentCurves::create_inputs().iter().map(|i| i.name.clone()).collect();
    assert_eq!(names, vec!["image", "curve", "red", "green", "blue"]);
    assert_eq!(OpImageAdjustmentCurves::create_outputs().len(), 1);
}

#[tokio::test]
async fn test_curves_default_input_is_identity() {
    // The default curve input must leave the image unchanged, so dropping the
    // node into an existing chain is a no-op until the user draws.
    let inputs = OpImageAdjustmentCurves::create_inputs();
    let Value::Curve(curve) = &inputs[1].value else { panic!("expected curve input") };
    let lut = tone_curve_lut(curve, 256);
    for (i, &v) in lut.iter().enumerate() {
        let x = i as f32 / 255.0;
        assert!((v - x).abs() < 0.01, "identity LUT off at {}: {}", x, v);
    }
}

#[tokio::test]
async fn test_curves_identity_leaves_image_unchanged() {
    let img = Arc::new(FloatImage::from_pixel(1, 1, 4, &[0.25, 0.5, 0.75, 0.6]));
    let mut inputs = inputs_with_curve(
        Value::Image { data: img, change_id: get_id() },
        OpImageAdjustmentCurves::identity_curve(),
    );
    let result = OpImageAdjustmentCurves::run(&mut inputs).await.unwrap();
    match &result.responses[0].value {
        Value::Image { data, .. } => {
            let p = data.get_pixel(0, 0);
            assert!((p[0] - 0.25).abs() < 0.01);
            assert!((p[1] - 0.5).abs() < 0.01);
            assert!((p[2] - 0.75).abs() < 0.01);
            // Alpha untouched exactly.
            assert_eq!(p[3], 0.6);
        }
        other => panic!("Expected Image, got {:?}", other),
    }
}

#[tokio::test]
async fn test_curves_brightening_curve_raises_midtones() {
    // Midpoint raised to 0.75: mid grey should brighten, endpoints pinned.
    let curve = linear_tone_curve(&[(0.0, 0.0), (0.5, 0.75), (1.0, 1.0)]);
    let img = Arc::new(FloatImage::from_pixel(1, 1, 4, &[0.5, 0.0, 1.0, 1.0]));
    let mut inputs = inputs_with_curve(Value::Image { data: img, change_id: get_id() }, curve);
    let result = OpImageAdjustmentCurves::run(&mut inputs).await.unwrap();
    match &result.responses[0].value {
        Value::Image { data, .. } => {
            let p = data.get_pixel(0, 0);
            assert!((p[0] - 0.75).abs() < 0.01, "midtone should map to 0.75, got {}", p[0]);
            assert!(p[1].abs() < 0.01, "black should stay black, got {}", p[1]);
            assert!((p[2] - 1.0).abs() < 0.01, "white should stay white, got {}", p[2]);
        }
        other => panic!("Expected Image, got {:?}", other),
    }
}

#[tokio::test]
async fn test_curves_inversion_curve() {
    // A descending diagonal inverts the image.
    let curve = linear_tone_curve(&[(0.0, 1.0), (1.0, 0.0)]);
    let img = Arc::new(FloatImage::from_pixel(1, 1, 4, &[0.2, 0.5, 0.9, 1.0]));
    let mut inputs = inputs_with_curve(Value::Image { data: img, change_id: get_id() }, curve);
    let result = OpImageAdjustmentCurves::run(&mut inputs).await.unwrap();
    match &result.responses[0].value {
        Value::Image { data, .. } => {
            let p = data.get_pixel(0, 0);
            assert!((p[0] - 0.8).abs() < 0.01);
            assert!((p[1] - 0.5).abs() < 0.01);
            assert!((p[2] - 0.1).abs() < 0.01);
        }
        other => panic!("Expected Image, got {:?}", other),
    }
}

#[test]
fn test_lut_extends_flat_past_endpoints() {
    // Curve only spans inputs [0.25, 0.75]: outside that range the mapping
    // clamps flat at the endpoint outputs (Photoshop behaviour).
    let curve = linear_tone_curve(&[(0.25, 0.2), (0.75, 0.8)]);
    let lut = tone_curve_lut(&curve, 256);
    assert!((lut[0] - 0.2).abs() < 0.01, "left of first point should be flat");
    assert!((lut[255] - 0.8).abs() < 0.01, "right of last point should be flat");
    // Middle of the span interpolates.
    assert!((sample_lut(&lut, 0.5) - 0.5).abs() < 0.02);
}

#[test]
fn test_lut_degenerate_curves() {
    // No points → identity ramp.
    let empty = Curve { points: vec![], closed: false, interpolation: CurveInterpolation::Linear, handles: vec![] };
    let lut = tone_curve_lut(&empty, 64);
    assert!((sample_lut(&lut, 0.3) - 0.3).abs() < 0.02);

    // One point → constant at its output.
    let single = Curve { points: vec![[0.5, 0.25]], closed: false, interpolation: CurveInterpolation::Linear, handles: vec![] };
    let lut = tone_curve_lut(&single, 64);
    assert!((sample_lut(&lut, 0.0) - 0.75).abs() < 0.01);
    assert!((sample_lut(&lut, 1.0) - 0.75).abs() < 0.01);
}

#[test]
fn test_lut_smooth_curve_within_range() {
    // A smooth S-curve: every LUT entry must be a valid, clamped value even
    // where the Catmull-Rom spline overshoots [0,1] in y.
    let curve = Curve {
        points: vec![[0.0, 1.0], [0.3, 0.9], [0.7, 0.1], [1.0, 0.0]],
        closed: false,
        interpolation: CurveInterpolation::Smooth,
        handles: Vec::new(),
    };
    let lut = tone_curve_lut(&curve, 1024);
    for &v in &lut {
        assert!(v.is_finite() && (0.0..=1.0).contains(&v), "LUT value out of range: {}", v);
    }
}

#[tokio::test]
async fn test_curves_1x1() {
    let img = Arc::new(FloatImage::from_pixel(1, 1, 4, &[0.784, 0.392, 0.196, 1.0]));
    let mut inputs = inputs_with_curve(
        Value::Image { data: img, change_id: get_id() },
        linear_tone_curve(&[(0.0, 0.0), (0.5, 0.3), (1.0, 1.0)]),
    );
    let result = OpImageAdjustmentCurves::run(&mut inputs).await;
    assert!(result.is_ok(), "curves 1x1 failed: {:?}", result.err());
}

#[tokio::test]
async fn test_curves_preserves_dimensions() {
    let mut inputs = inputs_with_curve(
        image_input(8, 8),
        linear_tone_curve(&[(0.0, 0.0), (1.0, 1.0)]),
    );
    let result = OpImageAdjustmentCurves::run(&mut inputs).await.unwrap();
    match &result.responses[0].value {
        Value::Image { data, .. } => {
            assert_eq!(data.width(), 8);
            assert_eq!(data.height(), 8);
        }
        other => panic!("Expected Image, got {:?}", other),
    }
}

#[tokio::test]
async fn test_curves_all_identity_passes_the_buffer_through() {
    // Every curve untouched: the node must hand back the original Arc so a
    // freshly dropped node costs nothing downstream.
    let src = test_image(4, 4);
    let inputs = inputs_with_curves(
        Value::Image { data: src.clone(), change_id: get_id() },
        identity(), identity(), identity(), identity(),
    );
    let out = run_image(inputs).await;
    assert!(Arc::ptr_eq(&src, &out), "all-identity curves must short-circuit");
}

#[tokio::test]
async fn test_curves_per_channel_inputs_are_tone_curves() {
    // The settings panel renders one editor box per unconnected ToneCurve
    // input, labelled by input name — so the marker has to be present.
    for input in OpImageAdjustmentCurves::create_inputs().iter().skip(1) {
        assert!(
            matches!(input.settings, Some(InputSettings::ToneCurve)),
            "input {} should be a tone curve",
            input.name
        );
        let Value::Curve(c) = &input.value else { panic!("expected curve default") };
        assert_eq!(*c, OpImageAdjustmentCurves::identity_curve(), "{} must default to identity", input.name);
    }
}

#[tokio::test]
async fn test_curves_red_curve_only_touches_red() {
    let src = Arc::new(FloatImage::from_pixel(2, 2, 4, &[0.5, 0.25, 0.75, 0.4]));
    let red = linear_tone_curve(&[(0.0, 0.0), (0.5, 0.8), (1.0, 1.0)]);
    let inputs = inputs_with_curves(
        Value::Image { data: src.clone(), change_id: get_id() },
        identity(), red, identity(), identity(),
    );
    let out = run_image(inputs).await;
    let p = out.get_pixel(0, 0);
    assert!((p[0] - 0.8).abs() < 0.01, "red should remap to 0.8, got {}", p[0]);
    // Green and blue go through untouched, bit for bit.
    assert_eq!(p[1], 0.25);
    assert_eq!(p[2], 0.75);
    assert_eq!(p[3], 0.4, "alpha must be preserved");
}

#[tokio::test]
async fn test_curves_channel_then_master_order() {
    // Composition is master(channel(v)) — verified against manual sampling.
    let red = linear_tone_curve(&[(0.0, 0.0), (0.5, 0.8), (1.0, 1.0)]);
    let master = linear_tone_curve(&[(0.0, 0.0), (0.5, 0.2), (1.0, 1.0)]);
    let src = Arc::new(FloatImage::from_pixel(1, 1, 4, &[0.5, 0.5, 0.5, 1.0]));
    let inputs = inputs_with_curves(
        Value::Image { data: src, change_id: get_id() },
        master.clone(), red.clone(), identity(), identity(),
    );
    let out = run_image(inputs).await;

    let red_lut = tone_curve_lut(&red, TONE_LUT_SIZE);
    let master_lut = tone_curve_lut(&master, TONE_LUT_SIZE);
    let expected = sample_lut(&master_lut, sample_lut(&red_lut, 0.5));
    let swapped = sample_lut(&red_lut, sample_lut(&master_lut, 0.5));

    let p = out.get_pixel(0, 0);
    assert!((p[0] - expected).abs() < 1e-5, "expected master(red(v)) = {}, got {}", expected, p[0]);
    assert!((expected - swapped).abs() > 0.05, "test curves must distinguish the two orders");
    // Green/blue get the master curve only.
    assert!((p[1] - sample_lut(&master_lut, 0.5)).abs() < 1e-5);
}

#[tokio::test]
async fn test_curves_per_channel_ignored_for_grayscale() {
    // 1-channel: no channel identity, so a red curve alone changes nothing.
    let src = Arc::new(FloatImage::from_pixel(2, 2, 1, &[0.5]));
    let inputs = inputs_with_curves(
        Value::Image { data: src.clone(), change_id: get_id() },
        identity(),
        linear_tone_curve(&[(0.0, 0.0), (0.5, 0.9), (1.0, 1.0)]),
        identity(), identity(),
    );
    let out = run_image(inputs).await;
    assert!(Arc::ptr_eq(&src, &out), "grayscale ignores per-channel curves");

    // 2-channel (grey + alpha) behaves the same way.
    let src2 = Arc::new(FloatImage::from_pixel(2, 2, 2, &[0.5, 0.3]));
    let inputs2 = inputs_with_curves(
        Value::Image { data: src2.clone(), change_id: get_id() },
        identity(),
        linear_tone_curve(&[(0.0, 0.0), (0.5, 0.9), (1.0, 1.0)]),
        identity(), identity(),
    );
    let out2 = run_image(inputs2).await;
    assert!(Arc::ptr_eq(&src2, &out2), "grey+alpha ignores per-channel curves");
}

#[tokio::test]
async fn test_curves_grayscale_still_honours_master() {
    let src = Arc::new(FloatImage::from_pixel(1, 1, 2, &[0.5, 0.3]));
    let inputs = inputs_with_curves(
        Value::Image { data: src, change_id: get_id() },
        linear_tone_curve(&[(0.0, 0.0), (0.5, 0.9), (1.0, 1.0)]),
        identity(), identity(), identity(),
    );
    let out = run_image(inputs).await;
    let p = out.get_pixel(0, 0);
    assert!((p[0] - 0.9).abs() < 0.01, "master should still apply, got {}", p[0]);
    assert_eq!(p[1], 0.3, "alpha untouched");
}

#[tokio::test]
async fn test_curves_blue_curve_cools_and_warms() {
    let src = Arc::new(FloatImage::from_pixel(1, 1, 3, &[0.5, 0.5, 0.5]));
    let up = inputs_with_curves(
        Value::Image { data: src.clone(), change_id: get_id() },
        identity(), identity(), identity(),
        linear_tone_curve(&[(0.0, 0.0), (0.5, 0.7), (1.0, 1.0)]),
    );
    let out = run_image(up).await;
    let p = out.get_pixel(0, 0);
    assert!(p[2] > p[0], "raising the blue curve should cool the image, got {:?}", p);
    assert_eq!(p[0], 0.5);
    assert_eq!(p[1], 0.5);
}

#[tokio::test]
async fn test_curves_output_range() {
    // An extreme smooth curve must still produce clamped output.
    let curve = Curve {
        points: vec![[0.0, 1.0], [0.2, 0.0], [0.8, 1.0], [1.0, 0.0]],
        closed: false,
        interpolation: CurveInterpolation::Smooth,
        handles: Vec::new(),
    };
    let mut inputs = inputs_with_curve(image_input(8, 8), curve);
    let result = OpImageAdjustmentCurves::run(&mut inputs).await.unwrap();
    match &result.responses[0].value {
        Value::Image { data, .. } => {
            for pixel in data.pixels() {
                for &val in pixel.iter().take(pixel.len().min(3)) {
                    assert!(val >= 0.0 && val <= 1.0, "pixel out of range: {}", val);
                }
            }
        }
        other => panic!("Expected Image, got {:?}", other),
    }
}
