use super::*;

use crate::input::Input;
use crate::value::Value;

/// Helper to create inputs with the given parameters.
fn make_inputs(
    seed: i32,
    width: i32,
    height: i32,
    density: f32,
    size: f32,
    size_variation: f32,
    height_variation: f32,
) -> Vec<Input> {
    vec![
        Input::new("seed".to_string(), Value::Integer(seed), None, None),
        Input::new("width".to_string(), Value::Integer(width), None, None),
        Input::new("height".to_string(), Value::Integer(height), None, None),
        Input::new("density".to_string(), Value::Decimal(density), None, None),
        Input::new("size".to_string(), Value::Decimal(size), None, None),
        Input::new("size_variation".to_string(), Value::Decimal(size_variation), None, None),
        Input::new("height_variation".to_string(), Value::Decimal(height_variation), None, None),
    ]
}

/// Default inputs matching the operation defaults.
fn default_inputs(seed: i32, width: i32, height: i32) -> Vec<Input> {
    make_inputs(seed, width, height, 6.0, 1.4, 0.5, 0.5)
}

#[tokio::test]
async fn test_settings() {
    let s = OpImageNoiseRollingHills::settings();
    assert_eq!(s.name, "rolling hills");
    assert_eq!(OpImageNoiseRollingHills::create_inputs().len(), 7);
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
    let mut inputs = make_inputs(3, 64, 64, 6.0, 1.4, 0.0, 0.0);
    let result = OpImageNoiseRollingHills::run(&mut inputs).await.unwrap();
    match &result.responses[0].value {
        Value::Image { data, .. } => {
            let (min, max) = data.pixels().fold((1.0_f32, 0.0_f32), |(lo, hi), p| (lo.min(p[0]), hi.max(p[0])));
            assert!(max - min > 0.1, "expected height variation, got range {min}..{max}");
        }
        _ => panic!("Expected Image"),
    }
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
