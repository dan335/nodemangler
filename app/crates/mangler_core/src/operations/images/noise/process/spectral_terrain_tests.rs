use super::*;

use crate::input::Input;
use crate::value::Value;

/// Helper to create inputs with the given parameters.
fn make_inputs(
    seed: i32,
    width: i32,
    height: i32,
    detail: i32,
    roll_off: f32,
) -> Vec<Input> {
    vec![
        Input::new("seed".to_string(), Value::Integer(seed), None, None),
        Input::new("width".to_string(), Value::Integer(width), None, None),
        Input::new("height".to_string(), Value::Integer(height), None, None),
        Input::new("detail".to_string(), Value::Integer(detail), None, None),
        Input::new("roll off".to_string(), Value::Decimal(roll_off), None, None),
    ]
}

/// Default inputs matching the operation defaults.
fn default_inputs(seed: i32, width: i32, height: i32) -> Vec<Input> {
    make_inputs(seed, width, height, 16, 3.5)
}

#[tokio::test]
async fn test_settings() {
    let s = OpImageNoiseSpectralTerrain::settings();
    assert_eq!(s.name, "spectral terrain");
    assert_eq!(OpImageNoiseSpectralTerrain::create_inputs().len(), 5);
    assert_eq!(OpImageNoiseSpectralTerrain::create_outputs().len(), 1);
}

#[tokio::test]
async fn test_run_basic() {
    let mut inputs = default_inputs(1, 16, 16);
    let result = OpImageNoiseSpectralTerrain::run(&mut inputs).await;
    assert!(result.is_ok(), "run failed: {:?}", result.err());
    match &result.unwrap().responses[0].value {
        Value::Image { .. } => {}
        other => panic!("Expected Image, got {:?}", other),
    }
}

#[tokio::test]
async fn test_correct_dimensions() {
    let mut inputs = default_inputs(1, 32, 16);
    let result = OpImageNoiseSpectralTerrain::run(&mut inputs).await.unwrap();
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
    let r1 = OpImageNoiseSpectralTerrain::run(&mut default_inputs(7, 16, 16)).await.unwrap();
    let r2 = OpImageNoiseSpectralTerrain::run(&mut default_inputs(7, 16, 16)).await.unwrap();
    match (&r1.responses[0].value, &r2.responses[0].value) {
        (Value::Image { data: d1, .. }, Value::Image { data: d2, .. }) => {
            assert_eq!(
                d1.pixels().collect::<Vec<_>>(),
                d2.pixels().collect::<Vec<_>>(),
                "spectral terrain is not deterministic"
            );
        }
        _ => panic!("Expected Image"),
    }
}

#[tokio::test]
async fn test_different_seeds_differ() {
    let r1 = OpImageNoiseSpectralTerrain::run(&mut default_inputs(1, 32, 32)).await.unwrap();
    let r2 = OpImageNoiseSpectralTerrain::run(&mut default_inputs(42, 32, 32)).await.unwrap();
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
    // Output is min/max normalized, so both extremes must be present
    let r = OpImageNoiseSpectralTerrain::run(&mut default_inputs(3, 64, 64)).await.unwrap();
    match &r.responses[0].value {
        Value::Image { data, .. } => {
            let (min, max) = data.pixels().fold((1.0_f32, 0.0_f32), |(lo, hi), p| (lo.min(p[0]), hi.max(p[0])));
            assert!(min < 0.01, "expected normalized minimum near 0, got {min}");
            assert!(max > 0.99, "expected normalized maximum near 1, got {max}");
        }
        _ => panic!("Expected Image"),
    }
}

#[tokio::test]
async fn test_roll_off_smooths() {
    // Higher roll off should shift energy toward low frequencies, producing
    // a visibly smoother (lower total-variation) surface for the same seed.
    async fn total_variation(roll_off: f32) -> f64 {
        let r = OpImageNoiseSpectralTerrain::run(&mut make_inputs(5, 64, 64, 16, roll_off)).await.unwrap();
        match &r.responses[0].value {
            Value::Image { data, .. } => {
                let w = data.width() as usize;
                let h = data.height() as usize;
                let px: Vec<f32> = data.pixels().map(|p| p[0]).collect();
                let mut tv = 0.0_f64;
                for y in 0..h {
                    for x in 0..w {
                        let v = px[y * w + x] as f64;
                        let right = px[y * w + (x + 1) % w] as f64;
                        let below = px[((y + 1) % h) * w + x] as f64;
                        tv += (v - right).abs() + (v - below).abs();
                    }
                }
                tv
            }
            _ => panic!("Expected Image"),
        }
    }

    let tv_rough = total_variation(1.5).await;
    let tv_smooth = total_variation(5.0).await;
    assert!(
        tv_smooth < tv_rough,
        "expected roll off 5.0 (tv={tv_smooth}) to be smoother than roll off 1.5 (tv={tv_rough})"
    );
}

/// Renders a 256x256 image at the TRUE defaults from `create_inputs()` and
/// writes a 2x2 tiling mosaic BMP so tiling can be eyeballed.
/// Run with `cargo test -p mangler_core spectral_terrain::tests::render_preview -- --ignored --nocapture`.
#[tokio::test]
#[ignore]
async fn render_preview() {
    let mut inputs = OpImageNoiseSpectralTerrain::create_inputs();
    // Override the default 512x512 with 256x256 for a faster preview render.
    inputs[1].value = Value::Integer(256);
    inputs[2].value = Value::Integer(256);

    let start = std::time::Instant::now();
    let result = OpImageNoiseSpectralTerrain::run(&mut inputs).await.unwrap();
    println!("spectral terrain 256x256 default render: {:?}", start.elapsed());

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
    tile.to_dynamic().save(format!("{dir}/spectral_terrain_tile.bmp")).unwrap();
}
