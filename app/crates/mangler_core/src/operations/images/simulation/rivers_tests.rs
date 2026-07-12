use super::*;

use crate::input::Input;
use crate::value::Value;

/// Builds the 15 inputs for the rivers node. `height_map`/`guide_map` of `None`
/// leave those image inputs at their unconnected 1x1 placeholder; the remaining
/// fine-tuning params take sensible fixed defaults.
#[allow(clippy::too_many_arguments)]
fn make_inputs(
    seed: i32,
    width: i32,
    height: i32,
    height_map: Option<FloatImage>,
    guide_map: Option<FloatImage>,
    amount: f64,
    river_width: i32,
    guide_strength: f64,
) -> Vec<Input> {
    let img_value = |img: Option<FloatImage>| match img {
        Some(i) => Value::Image { data: Arc::new(i), change_id: get_id() },
        None => Value::Image { data: default_image(), change_id: get_id() },
    };
    vec![
        Input::new("seed".to_string(), Value::Integer(seed), None, None),
        Input::new("width".to_string(), Value::Integer(width), None, None),
        Input::new("height".to_string(), Value::Integer(height), None, None),
        Input::new("height map".to_string(), img_value(height_map), None, None),
        Input::new("river guide".to_string(), img_value(guide_map), None, None),
        Input::new("river amount".to_string(), Value::Decimal(amount as f32), None, None),
        Input::new("carve depth".to_string(), Value::Decimal(0.15), None, None),
        Input::new("depth exponent".to_string(), Value::Decimal(0.4), None, None),
        Input::new("river width".to_string(), Value::Integer(river_width), None, None),
        Input::new("valley width".to_string(), Value::Decimal(4.0), None, None),
        Input::new("valley shape".to_string(), Value::Decimal(0.5), None, None),
        Input::new("bed smoothing".to_string(), Value::Integer(2), None, None),
        Input::new("guide strength".to_string(), Value::Decimal(guide_strength as f32), None, None),
        Input::new("octaves".to_string(), Value::Integer(6), None, None),
        Input::new("frequency".to_string(), Value::Decimal(5.0), None, None),
    ]
}

/// Fallback-terrain inputs (no maps connected) with the given size/amount and a
/// river width bumped so scaling to the small test image leaves a few pixels.
fn fallback_inputs(seed: i32, width: i32, height: i32, amount: f64, river_width: i32) -> Vec<Input> {
    make_inputs(seed, width, height, None, None, amount, river_width, 0.5)
}

/// Extracts the pixel data of the response at `index`.
fn image_pixels(result: &OperationResponse, index: usize) -> Vec<Vec<f32>> {
    match &result.responses[index].value {
        Value::Image { data, .. } => data.pixels().map(|p| p.to_vec()).collect(),
        other => panic!("Expected Image, got {:?}", other),
    }
}

/// Extracts the single-channel pixel values of the response at `index`.
fn channel0(result: &OperationResponse, index: usize) -> Vec<f32> {
    match &result.responses[index].value {
        Value::Image { data, .. } => data.pixels().map(|p| p[0]).collect(),
        other => panic!("Expected Image, got {:?}", other),
    }
}

/// Builds a single-channel gentle top-to-bottom ramp: 0.45 + 0.1 * y / (h-1).
/// Oriented so flow runs vertically, aligned with the vertical guide stripe the
/// guide-attraction test measures (a left-to-right ramp would drain horizontally,
/// perpendicular to that stripe).
fn gentle_ramp(width: u32, height: u32) -> FloatImage {
    let mut img = FloatImage::new(width, height, 1);
    for y in 0..height {
        for x in 0..width {
            img.put_pixel(x, y, &[0.45 + 0.1 * y as f32 / (height - 1) as f32]);
        }
    }
    img
}

/// Builds a single-channel black image with a bright vertical stripe of width
/// `stripe_w` centered at column `col`.
fn vertical_stripe(width: u32, height: u32, col: u32, stripe_w: u32) -> FloatImage {
    let mut img = FloatImage::new(width, height, 1);
    let half = stripe_w / 2;
    for y in 0..height {
        for x in 0..width {
            let on = x + half >= col && x <= col + half;
            img.put_pixel(x, y, &[if on { 1.0 } else { 0.0 }]);
        }
    }
    img
}

#[tokio::test]
async fn test_settings() {
    let s = OpImageSimulationRivers::settings();
    assert_eq!(s.name, "rivers");
    assert_eq!(OpImageSimulationRivers::create_inputs().len(), 15);
    assert_eq!(OpImageSimulationRivers::create_outputs().len(), 4);
}

#[tokio::test]
async fn test_correct_dimensions() {
    let mut inputs = fallback_inputs(1, 32, 16, 0.5, 6);
    let result = OpImageSimulationRivers::run(&mut inputs).await.unwrap();
    assert_eq!(result.responses.len(), 4);
    for i in 0..4 {
        match &result.responses[i].value {
            Value::Image { data, .. } => {
                assert_eq!(data.width(), 32, "output {i} width");
                assert_eq!(data.height(), 16, "output {i} height");
            }
            other => panic!("output {i}: expected Image, got {other:?}"),
        }
    }
}

#[tokio::test]
async fn test_deterministic() {
    let r1 = OpImageSimulationRivers::run(&mut fallback_inputs(7, 48, 32, 0.5, 8)).await.unwrap();
    let r2 = OpImageSimulationRivers::run(&mut fallback_inputs(7, 48, 32, 0.5, 8)).await.unwrap();
    for i in 0..4 {
        assert_eq!(image_pixels(&r1, i), image_pixels(&r2, i), "output {i} is not deterministic");
    }
}

#[tokio::test]
async fn test_different_seeds_differ() {
    let r1 = OpImageSimulationRivers::run(&mut fallback_inputs(1, 32, 32, 0.5, 8)).await.unwrap();
    let r2 = OpImageSimulationRivers::run(&mut fallback_inputs(2, 32, 32, 0.5, 8)).await.unwrap();
    assert_ne!(
        image_pixels(&r1, 0),
        image_pixels(&r2, 0),
        "different seeds should produce different height output"
    );
}

#[tokio::test]
async fn test_rivers_reach_edge() {
    // All drainage exits at the image border, so at least one border pixel of
    // the river mask must be lit. River width bumped by 1024/64 so the trunk is
    // a few pixels wide after resolution scaling.
    let mut inputs = fallback_inputs(1, 64, 64, 0.5, 64);
    let result = OpImageSimulationRivers::run(&mut inputs).await.unwrap();
    let mask = channel0(&result, 1);
    let (w, h) = (64usize, 64usize);
    let mut edge_lit = false;
    for y in 0..h {
        for x in 0..w {
            if x == 0 || y == 0 || x == w - 1 || y == h - 1 {
                if mask[y * w + x] > 0.0 {
                    edge_lit = true;
                }
            }
        }
    }
    assert!(edge_lit, "expected at least one border pixel of the river mask to be lit");
}

#[tokio::test]
async fn test_guide_attracts() {
    // On a terrain that barely prefers any direction, a bright vertical guide
    // stripe should pull river mass toward its column when guide strength is
    // high, versus a strength of 0 (which is identical to no guide).
    let (w, h) = (32u32, 32u32);
    let ramp = gentle_ramp(w, h);
    let stripe = vertical_stripe(w, h, 8, 2);

    let run_with = |strength: f64| {
        make_inputs(1, w as i32, h as i32, Some(ramp.clone()), Some(stripe.clone()), 0.5, 64, strength)
    };

    let with_guide = OpImageSimulationRivers::run(&mut run_with(1.0)).await.unwrap();
    let without_guide = OpImageSimulationRivers::run(&mut run_with(0.0)).await.unwrap();

    let sum_cols = |result: &OperationResponse| -> f32 {
        let mask = channel0(result, 1);
        let mut s = 0.0;
        for y in 0..h as usize {
            for x in 6..=10usize {
                s += mask[y * w as usize + x];
            }
        }
        s
    };

    let s_on = sum_cols(&with_guide);
    let s_off = sum_cols(&without_guide);
    assert!(
        s_on > s_off,
        "guide should attract more river mass to its column (with={s_on}, without={s_off})"
    );
}

#[tokio::test]
async fn test_more_amount_more_rivers() {
    let count_lit = |result: &OperationResponse| -> usize {
        channel0(result, 1).iter().filter(|&&v| v > 0.0).count()
    };
    let high = OpImageSimulationRivers::run(&mut fallback_inputs(3, 64, 64, 0.8, 64)).await.unwrap();
    let low = OpImageSimulationRivers::run(&mut fallback_inputs(3, 64, 64, 0.2, 64)).await.unwrap();
    let hc = count_lit(&high);
    let lc = count_lit(&low);
    assert!(hc > lc, "more river amount should light more mask pixels (0.8={hc}, 0.2={lc})");
}

#[tokio::test]
async fn test_flow_map_everywhere() {
    // ln(1 + acc) / ln(1 + a_max) with acc >= 1 is strictly positive at every
    // pixel; assert the raw (encoded) float pixels are all > 0.
    let mut inputs = fallback_inputs(1, 32, 32, 0.5, 8);
    let result = OpImageSimulationRivers::run(&mut inputs).await.unwrap();
    let flow = channel0(&result, 2);
    assert!(flow.iter().all(|&v| v > 0.0), "flow map should be positive everywhere");
}

#[tokio::test]
async fn test_water_depth_in_rivers() {
    let mut inputs = fallback_inputs(3, 64, 64, 0.6, 64);
    let result = OpImageSimulationRivers::run(&mut inputs).await.unwrap();
    let mask = channel0(&result, 1);
    let depth = channel0(&result, 3);

    let river_pixels: Vec<usize> = (0..mask.len()).filter(|&i| mask[i] > 0.0).collect();
    assert!(!river_pixels.is_empty(), "expected some river-mask pixels");
    let wet = river_pixels.iter().filter(|&&i| depth[i] > 0.0).count();
    let frac = wet as f64 / river_pixels.len() as f64;
    assert!(frac >= 0.95, "river pixels should carry water depth ({frac:.3} wet)");
    assert!(depth.iter().any(|&v| v > 0.0), "expected some water-depth pixels");
}

/// Renders all four outputs at the 512x512 defaults to PNGs and reports timing.
/// Run with `cargo test -p mangler_core rivers::tests::render_preview -- --ignored --nocapture`.
#[tokio::test]
#[ignore]
async fn render_preview() {
    let mut inputs = OpImageSimulationRivers::create_inputs();
    let start = std::time::Instant::now();
    let result = OpImageSimulationRivers::run(&mut inputs).await.unwrap();
    println!("rivers 512x512 default render: {:?}", start.elapsed());

    let dir = "/private/tmp/claude-501/-Users-danielphillips-rust-nodemangler/98a53923-dfe7-4d74-844a-01a99e2322ea/scratchpad";
    let names = ["rivers_height", "rivers_mask", "rivers_flow", "rivers_depth"];
    for (i, name) in names.iter().enumerate() {
        match &result.responses[i].value {
            Value::Image { data, .. } => {
                data.to_dynamic().save(format!("{dir}/{name}.png")).unwrap();
            }
            other => panic!("output {i}: expected Image, got {other:?}"),
        }
    }
}
