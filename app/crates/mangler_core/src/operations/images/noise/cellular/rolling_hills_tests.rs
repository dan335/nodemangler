use super::*;

use crate::curve::{Curve, CurveInterpolation};
use crate::input::Input;
use crate::operations::images::tone_curve::identity_tone_curve;
use crate::value::Value;

/// Helper to create inputs with the given parameters (profile left at the
/// identity default).
fn make_inputs(
    seed: i32,
    width: i32,
    height: i32,
    density: f32,
    size: f32,
    size_variation: f32,
    height_variation: f32,
    peakiness: f32,
    merge: f32,
) -> Vec<Input> {
    vec![
        Input::new("seed".to_string(), Value::Integer(seed), None, None),
        Input::new("width".to_string(), Value::Integer(width), None, None),
        Input::new("height".to_string(), Value::Integer(height), None, None),
        Input::new("density".to_string(), Value::Decimal(density), None, None),
        Input::new("size".to_string(), Value::Decimal(size), None, None),
        Input::new("size_variation".to_string(), Value::Decimal(size_variation), None, None),
        Input::new("height_variation".to_string(), Value::Decimal(height_variation), None, None),
        Input::new("peakiness".to_string(), Value::Decimal(peakiness), None, None),
        Input::new("merge".to_string(), Value::Decimal(merge), None, None),
        Input::new("profile".to_string(), Value::Curve(identity_tone_curve()), None, None),
    ]
}

/// Default inputs matching the operation defaults.
fn default_inputs(seed: i32, width: i32, height: i32) -> Vec<Input> {
    make_inputs(seed, width, height, 6.0, 1.4, 0.5, 0.5, 1.0, 1.0)
}

#[tokio::test]
async fn test_settings() {
    let s = OpImageNoiseRollingHills::settings();
    assert_eq!(s.name, "rolling hills");
    assert_eq!(OpImageNoiseRollingHills::create_inputs().len(), 10);
    assert_eq!(OpImageNoiseRollingHills::create_outputs().len(), 1);
}

#[tokio::test]
async fn test_run_basic() {
    let mut inputs = default_inputs(1, 16, 16);
    let result = OpImageNoiseRollingHills::run(&mut inputs).await;
    assert!(result.is_ok(), "run failed: {:?}", result.err());
    match &result.unwrap().responses[0].value {
        Value::Image { .. } => {}
        other => panic!("Expected Image, got {:?}", other),
    }
}

#[tokio::test]
async fn test_correct_dimensions() {
    let mut inputs = default_inputs(1, 32, 16);
    let result = OpImageNoiseRollingHills::run(&mut inputs).await.unwrap();
    match &result.responses[0].value {
        Value::Image { data, .. } => {
            assert_eq!(data.width(), 32);
            assert_eq!(data.height(), 16);
        }
        other => panic!("Expected Image, got {:?}", other),
    }
}

#[tokio::test]
async fn test_deterministic() {
    let r1 = OpImageNoiseRollingHills::run(&mut default_inputs(7, 16, 16)).await.unwrap();
    let r2 = OpImageNoiseRollingHills::run(&mut default_inputs(7, 16, 16)).await.unwrap();
    match (&r1.responses[0].value, &r2.responses[0].value) {
        (Value::Image { data: d1, .. }, Value::Image { data: d2, .. }) => {
            assert_eq!(
                d1.pixels().collect::<Vec<_>>(),
                d2.pixels().collect::<Vec<_>>(),
                "rolling hills noise is not deterministic"
            );
        }
        _ => panic!("Expected Image"),
    }
}

#[tokio::test]
async fn test_different_seeds_differ() {
    let r1 = OpImageNoiseRollingHills::run(&mut default_inputs(1, 32, 32)).await.unwrap();
    let r2 = OpImageNoiseRollingHills::run(&mut default_inputs(42, 32, 32)).await.unwrap();
    match (&r1.responses[0].value, &r2.responses[0].value) {
        (Value::Image { data: d1, .. }, Value::Image { data: d2, .. }) => {
            assert_ne!(
                d1.pixels().collect::<Vec<_>>(),
                d2.pixels().collect::<Vec<_>>(),
                "different seeds should produce different output"
            );
        }
        _ => panic!("Expected Image"),
    }
}

#[tokio::test]
async fn test_normalized_full_range() {
    // Min/max normalization should stretch the output to (nearly) the full [0, 1] range
    let mut inputs = default_inputs(3, 64, 64);
    let result = OpImageNoiseRollingHills::run(&mut inputs).await.unwrap();
    match &result.responses[0].value {
        Value::Image { data, .. } => {
            let (min, max) = data.pixels().fold((1.0_f32, 0.0_f32), |(lo, hi), p| (lo.min(p[0]), hi.max(p[0])));
            assert!(min < 0.01, "expected near-zero minimum, got {min}");
            assert!(max > 0.99, "expected near-one maximum, got {max}");
        }
        _ => panic!("Expected Image"),
    }
}

#[tokio::test]
async fn test_zero_variation_still_hills() {
    // Uniform hills (no size or height variation) must still produce height
    // variation across the image from the hill shapes themselves.
    let mut inputs = make_inputs(3, 64, 64, 6.0, 1.4, 0.0, 0.0, 1.0, 1.0);
    let result = OpImageNoiseRollingHills::run(&mut inputs).await.unwrap();
    match &result.responses[0].value {
        Value::Image { data, .. } => {
            let (min, max) = data.pixels().fold((1.0_f32, 0.0_f32), |(lo, hi), p| (lo.min(p[0]), hi.max(p[0])));
            assert!(max - min > 0.1, "expected height variation, got range {min}..{max}");
        }
        _ => panic!("Expected Image"),
    }
}

#[tokio::test]
async fn test_peakiness_changes_profile() {
    // Flat-topped (0.25) and pointy (4.0) profiles must differ from the
    // default dome for the same hill arrangement.
    async fn pixels(peakiness: f32) -> Vec<f32> {
        let r = OpImageNoiseRollingHills::run(&mut make_inputs(3, 32, 32, 6.0, 1.4, 0.5, 0.5, peakiness, 1.0)).await.unwrap();
        match &r.responses[0].value {
            Value::Image { data, .. } => data.pixels().map(|p| p[0]).collect(),
            _ => panic!("Expected Image"),
        }
    }
    let dome = pixels(1.0).await;
    assert_ne!(dome, pixels(0.25).await, "flat-topped profile should differ from dome");
    assert_ne!(dome, pixels(4.0).await, "pointy profile should differ from dome");
}

#[tokio::test]
async fn test_merge_changes_composition() {
    // Tallest-wins (0) and summed (1) overlap handling must differ when
    // hills overlap, and tallest-wins must still produce a full-range image.
    let r0 = OpImageNoiseRollingHills::run(&mut make_inputs(3, 32, 32, 6.0, 1.4, 0.5, 0.5, 1.0, 0.0)).await.unwrap();
    let r1 = OpImageNoiseRollingHills::run(&mut make_inputs(3, 32, 32, 6.0, 1.4, 0.5, 0.5, 1.0, 1.0)).await.unwrap();
    match (&r0.responses[0].value, &r1.responses[0].value) {
        (Value::Image { data: d0, .. }, Value::Image { data: d1, .. }) => {
            assert_ne!(
                d0.pixels().collect::<Vec<_>>(),
                d1.pixels().collect::<Vec<_>>(),
                "merge 0 and merge 1 should compose overlaps differently"
            );
            let (min, max) = d0.pixels().fold((1.0_f32, 0.0_f32), |(lo, hi), p| (lo.min(p[0]), hi.max(p[0])));
            assert!(min < 0.01 && max > 0.99, "tallest-wins output should still normalize to full range, got {min}..{max}");
        }
        _ => panic!("Expected Image"),
    }
}

/// Runs with the given profile curve set on input 9 and returns the pixels.
async fn pixels_with_profile(profile: Curve) -> Vec<f32> {
    let mut inputs = default_inputs(3, 32, 32);
    inputs[9] = Input::new("profile".to_string(), Value::Curve(profile), None, None);
    let r = OpImageNoiseRollingHills::run(&mut inputs).await.unwrap();
    match &r.responses[0].value {
        Value::Image { data, .. } => data.pixels().map(|p| p[0]).collect(),
        _ => panic!("Expected Image"),
    }
}

#[tokio::test]
async fn test_default_profile_is_identity() {
    // The default (untouched identity) profile and an explicitly-set identity
    // curve must produce the same output: the fast path (optional_lut = None)
    // is exercised by both, so the default graph stays bit-identical to the
    // pre-profile behaviour.
    let via_defaults = {
        let mut inputs = OpImageNoiseRollingHills::create_inputs();
        inputs[0] = Input::new("seed".to_string(), Value::Integer(3), None, None);
        inputs[1] = Input::new("width".to_string(), Value::Integer(32), None, None);
        inputs[2] = Input::new("height".to_string(), Value::Integer(32), None, None);
        let r = OpImageNoiseRollingHills::run(&mut inputs).await.unwrap();
        match &r.responses[0].value {
            Value::Image { data, .. } => data.pixels().map(|p| p[0]).collect::<Vec<f32>>(),
            _ => panic!("Expected Image"),
        }
    };
    let via_identity = pixels_with_profile(identity_tone_curve()).await;
    assert_eq!(via_defaults, via_identity, "default profile should behave as identity");
}

#[tokio::test]
async fn test_profile_curve_changes_output() {
    // A steep-at-the-rim shaping curve (dome heights pushed up) must change
    // the output relative to the identity default.
    let shaped = Curve {
        points: vec![[0.0, 1.0], [0.5, 0.2], [1.0, 0.0]],
        closed: false,
        interpolation: CurveInterpolation::Smooth,
        handles: Vec::new(),
    };
    let identity = pixels_with_profile(identity_tone_curve()).await;
    let remapped = pixels_with_profile(shaped).await;
    assert_ne!(identity, remapped, "a non-identity profile should change the output");
}

#[tokio::test]
async fn test_crushing_profile_flattens_field() {
    // A constant-0 profile (y-down: both points on the bottom edge) crushes
    // every hill contribution to zero, so the pre-normalization field is
    // uniformly zero and the output collapses to a flat image.
    let crush = Curve {
        points: vec![[0.0, 1.0], [1.0, 1.0]],
        closed: false,
        interpolation: CurveInterpolation::Smooth,
        handles: Vec::new(),
    };
    let flat = pixels_with_profile(crush).await;
    let first = flat[0];
    assert!(flat.iter().all(|&v| v == first), "constant-0 profile should produce a flat image");
}

/// Renders the default rolling hills heightmap and reports the render time,
/// plus a 2x2 tiling mosaic BMP for eyeballing seamlessness. Run with
/// `cargo test -p mangler_core rolling_hills::tests::render_preview -- --ignored --nocapture`.
#[tokio::test]
#[ignore]
async fn render_preview() {
    let mut inputs = OpImageNoiseRollingHills::create_inputs();
    let start = std::time::Instant::now();
    let result = OpImageNoiseRollingHills::run(&mut inputs).await.unwrap();
    println!("rolling hills default render: {:?}", start.elapsed());

    let dir = "/private/tmp/claude-501/-Users-danielphillips-rust-nodemangler/51ed0e89-32e9-4cf5-83e1-33f0c21f455e/scratchpad/previews";
    std::fs::create_dir_all(dir).unwrap();
    let data = match &result.responses[0].value {
        Value::Image { data, .. } => data.clone(),
        other => panic!("Expected Image, got {other:?}"),
    };
    let (w, h, ch) = (data.width(), data.height(), data.channels());
    let src: Vec<Vec<f32>> = data.pixels().map(|p| p.to_vec()).collect();
    let mut tile = FloatImage::new(w * 2, h * 2, ch);
    for y in 0..h {
        for x in 0..w {
            let px = &src[(y * w + x) as usize];
            for (dx, dy) in [(0, 0), (w, 0), (0, h), (w, h)] { tile.put_pixel(x + dx, y + dy, px); }
        }
    }
    tile.to_dynamic().save(format!("{dir}/rolling_hills_tile.bmp")).unwrap();
}
