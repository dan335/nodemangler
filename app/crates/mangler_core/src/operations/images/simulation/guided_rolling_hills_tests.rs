use super::*;

use crate::input::Input;
use crate::operations::images::noise::cellular::rolling_hills::OpImageNoiseRollingHills;
use crate::value::Value;

/// Builds the 16 default inputs: no guidance map connected, hill params
/// matching rolling hills' own defaults (so the unconnected fallback can be
/// compared pixel-for-pixel against that node).
fn default_inputs(seed: i32, width: i32, height: i32) -> Vec<Input> {
    vec![
        Input::new("seed".to_string(), Value::Integer(seed), None, None),
        Input::new("width".to_string(), Value::Integer(width), None, None),
        Input::new("height".to_string(), Value::Integer(height), None, None),
        Input::new("guidance map".to_string(), Value::Image { data: default_image(), change_id: get_id() }, None, None),
        Input::new("mask threshold".to_string(), Value::Decimal(0.5), None, None),
        Input::new("river width".to_string(), Value::Integer(8), None, None),
        Input::new("valley width".to_string(), Value::Integer(96), None, None),
        Input::new("valley shape".to_string(), Value::Decimal(0.5), None, None),
        Input::new("river depth".to_string(), Value::Decimal(0.35), None, None),
        Input::new("bank height".to_string(), Value::Decimal(0.1), None, None),
        Input::new("density".to_string(), Value::Decimal(6.0), None, None),
        Input::new("size".to_string(), Value::Decimal(1.4), None, None),
        Input::new("size_variation".to_string(), Value::Decimal(0.5), None, None),
        Input::new("height_variation".to_string(), Value::Decimal(0.5), None, None),
        Input::new("peakiness".to_string(), Value::Decimal(1.0), None, None),
        Input::new("merge".to_string(), Value::Decimal(1.0), None, None),
    ]
}

/// Overwrites input `idx` with a new value, keeping the same name.
fn set(inputs: &mut [Input], idx: usize, value: Value) {
    inputs[idx] = Input::new(inputs[idx].name.clone(), value, None, None);
}

/// Overwrites input `idx` with an image value.
fn set_image(inputs: &mut [Input], idx: usize, img: FloatImage) {
    set(inputs, idx, Value::Image { data: Arc::new(img), change_id: get_id() });
}

/// Uniform single-channel image filled with `value`.
fn solid_image(width: u32, height: u32, value: f32) -> FloatImage {
    let mut img = FloatImage::new(width, height, 1);
    for y in 0..height {
        for x in 0..width {
            img.put_pixel(x, y, &[value]);
        }
    }
    img
}

/// Black (river) single-pixel-wide vertical stripe at column `col` on an
/// otherwise white image, used as a synthetic river mask (dark = river).
fn vertical_stripe_mask(width: u32, height: u32, col: u32) -> FloatImage {
    let mut img = solid_image(width, height, 1.0);
    for y in 0..height {
        img.put_pixel(col, y, &[0.0]);
    }
    img
}

/// Extracts the pixel data of the response at `index`.
fn image_pixels(result: &OperationResponse, index: usize) -> Vec<Vec<f32>> {
    match &result.responses[index].value {
        Value::Image { data, .. } => data.pixels().map(|p| p.to_vec()).collect(),
        other => panic!("Expected Image, got {:?}", other),
    }
}

/// Reads channel-0 value at (x, y) of the response at `index`.
fn pixel_at(result: &OperationResponse, index: usize, x: usize, y: usize, w: usize) -> f32 {
    image_pixels(result, index)[y * w + x][0]
}

#[tokio::test]
async fn test_settings() {
    let s = OpImageSimulationGuidedRollingHills::settings();
    assert_eq!(s.name, "guided rolling hills");
    assert_eq!(OpImageSimulationGuidedRollingHills::create_inputs().len(), 16);
    assert_eq!(OpImageSimulationGuidedRollingHills::create_outputs().len(), 2);
}

#[tokio::test]
async fn test_run_basic() {
    let mut inputs = default_inputs(1, 16, 16);
    let result = OpImageSimulationGuidedRollingHills::run(&mut inputs).await;
    assert!(result.is_ok(), "run failed: {:?}", result.err());
}

#[tokio::test]
async fn test_correct_dimensions() {
    let mut inputs = default_inputs(1, 32, 16);
    let result = OpImageSimulationGuidedRollingHills::run(&mut inputs).await.unwrap();
    assert_eq!(result.responses.len(), 2);
    for i in 0..2 {
        match &result.responses[i].value {
            Value::Image { data, .. } => {
                assert_eq!(data.width(), 32, "output {i} width");
                assert_eq!(data.height(), 16, "output {i} height");
            }
            other => panic!("Expected Image, got {:?}", other),
        }
    }
}

#[tokio::test]
async fn test_deterministic() {
    let build = || {
        let mut inputs = default_inputs(7, 32, 32);
        set_image(&mut inputs, 3, vertical_stripe_mask(32, 32, 16));
        inputs
    };
    let r1 = OpImageSimulationGuidedRollingHills::run(&mut build()).await.unwrap();
    let r2 = OpImageSimulationGuidedRollingHills::run(&mut build()).await.unwrap();
    assert_eq!(image_pixels(&r1, 0), image_pixels(&r2, 0), "height output not deterministic");
    assert_eq!(image_pixels(&r1, 1), image_pixels(&r2, 1), "channel mask output not deterministic");
}

#[tokio::test]
async fn test_unconnected_matches_rolling_hills() {
    let mut inputs = default_inputs(3, 32, 32);
    let result = OpImageSimulationGuidedRollingHills::run(&mut inputs).await.unwrap();

    let mut rh_inputs = vec![
        Input::new("seed".to_string(), Value::Integer(3), None, None),
        Input::new("width".to_string(), Value::Integer(32), None, None),
        Input::new("height".to_string(), Value::Integer(32), None, None),
        Input::new("density".to_string(), Value::Decimal(6.0), None, None),
        Input::new("size".to_string(), Value::Decimal(1.4), None, None),
        Input::new("size_variation".to_string(), Value::Decimal(0.5), None, None),
        Input::new("height_variation".to_string(), Value::Decimal(0.5), None, None),
        Input::new("peakiness".to_string(), Value::Decimal(1.0), None, None),
        Input::new("merge".to_string(), Value::Decimal(1.0), None, None),
    ];
    let rh_result = OpImageNoiseRollingHills::run(&mut rh_inputs).await.unwrap();

    assert_eq!(image_pixels(&result, 0), image_pixels(&rh_result, 0), "unconnected height should match plain rolling hills exactly");
    for p in image_pixels(&result, 1) {
        assert_eq!(p[0], 0.0, "channel mask should be all black when unconnected");
    }
}

#[tokio::test]
async fn test_empty_mask_falls_back() {
    let mut unconnected = default_inputs(5, 24, 24);
    let unconnected_result = OpImageSimulationGuidedRollingHills::run(&mut unconnected).await.unwrap();

    // All-white connected mask = no river pixels (dark = river now).
    let mut connected_white = default_inputs(5, 24, 24);
    set_image(&mut connected_white, 3, solid_image(24, 24, 1.0));
    let connected_result = OpImageSimulationGuidedRollingHills::run(&mut connected_white).await.unwrap();

    assert_eq!(image_pixels(&unconnected_result, 0), image_pixels(&connected_result, 0), "all-white connected mask should fall back to plain hills");
    assert_eq!(image_pixels(&unconnected_result, 1), image_pixels(&connected_result, 1), "channel mask should still be all black");
}

#[tokio::test]
async fn test_all_river_mask_is_flat_bed() {
    // All-black connected mask = the entire image is channel: flat zero bed,
    // fully bright channel mask. (FloatImage::new is zero-filled = all-dark.)
    let mut inputs = default_inputs(5, 24, 24);
    set_image(&mut inputs, 3, FloatImage::new(24, 24, 1));
    let result = OpImageSimulationGuidedRollingHills::run(&mut inputs).await.unwrap();

    for p in image_pixels(&result, 0) {
        assert_eq!(p[0], 0.0, "all-river mask should produce a flat zero bed");
    }
    for p in image_pixels(&result, 1) {
        assert!(p[0] > 0.999, "channel mask should be fully bright everywhere, got {}", p[0]);
    }
}

#[tokio::test]
async fn test_channel_low_and_flat() {
    // 128px image; river width 16 / valley width 128 scale (1024px ref, /8
    // factor) to 2px / 16px respectively.
    let w = 128u32;
    let h = 128u32;
    let col = 64u32;
    let mut inputs = default_inputs(9, w as i32, h as i32);
    set_image(&mut inputs, 3, vertical_stripe_mask(w, h, col));
    set(&mut inputs, 5, Value::Integer(16));
    set(&mut inputs, 6, Value::Integer(128));
    let result = OpImageSimulationGuidedRollingHills::run(&mut inputs).await.unwrap();
    let height = image_pixels(&result, 0);

    // Channel pixels (within river width of the stripe) must be exactly 0:
    // at d=0 the wall, bank cut, and levee terms all vanish regardless of
    // river depth / bank height.
    for y in 0..h as usize {
        for dx in -2i32..=2 {
            let x = (col as i32 + dx) as usize;
            let v = height[y * w as usize + x][0];
            assert_eq!(v, 0.0, "channel pixel ({x},{y}) should be exactly 0, got {v}");
        }
    }

    // Far-field pixels (well beyond the valley) must show hill variance.
    let mut far = Vec::new();
    for y in 0..h as usize {
        for x in 0..w as usize {
            if (x as i32 - col as i32).abs() > 30 {
                far.push(height[y * w as usize + x][0]);
            }
        }
    }
    let (min, max) = far.iter().fold((1.0_f32, 0.0_f32), |(lo, hi), &v| (lo.min(v), hi.max(v)));
    assert!(max - min > 0.05, "expected hill variance in far field, got range {min}..{max}");
}

#[tokio::test]
async fn test_valley_rises_monotonically() {
    let w = 128u32;
    let h = 128u32;
    let col = 64u32;
    let mut inputs = default_inputs(11, w as i32, h as i32);
    set_image(&mut inputs, 3, vertical_stripe_mask(w, h, col));
    set(&mut inputs, 5, Value::Integer(8)); // river width -> 1px
    set(&mut inputs, 6, Value::Integer(400)); // valley width -> 50px
    set(&mut inputs, 8, Value::Decimal(1.0)); // river depth = 1: composed == wall
    set(&mut inputs, 9, Value::Decimal(0.0)); // bank height = 0: no levee bump
    let result = OpImageSimulationGuidedRollingHills::run(&mut inputs).await.unwrap();
    let height = image_pixels(&result, 0);

    let row = 64usize;
    let mut prev = height[row * w as usize + col as usize][0];
    for x in (col as usize + 1)..w as usize {
        let v = height[row * w as usize + x][0];
        assert!(v >= prev - 1e-6, "height should be non-decreasing away from the channel at x={x}: {v} < {prev}");
        prev = v;
    }
}

#[tokio::test]
async fn test_valley_wall_is_convex() {
    // The wall profile must be convex terrain: steepest right at the bank,
    // easing off toward the rim (never flattening out near the water). With
    // river depth 1 / bank 0 the height IS the wall profile; sRGB encoding is
    // an increasing concave transform, so the slope must still be
    // non-increasing walking away from the channel.
    let w = 128u32;
    let h = 128u32;
    let col = 64u32;
    let mut inputs = default_inputs(21, w as i32, h as i32);
    set_image(&mut inputs, 3, vertical_stripe_mask(w, h, col));
    set(&mut inputs, 5, Value::Integer(8)); // river width -> 1px
    set(&mut inputs, 6, Value::Integer(400)); // valley width -> 50px
    set(&mut inputs, 8, Value::Decimal(1.0)); // river depth = 1: composed == wall
    set(&mut inputs, 9, Value::Decimal(0.0)); // bank height = 0: no levee bump
    let result = OpImageSimulationGuidedRollingHills::run(&mut inputs).await.unwrap();
    let height = image_pixels(&result, 0);
    let row = 64usize;

    // Sample strictly inside the valley walls (d in (0, 1)): dist 2..=48.
    let profile: Vec<f32> = (2..=48).map(|dist| height[row * w as usize + col as usize + dist][0]).collect();
    let mut prev_slope = f32::INFINITY;
    for i in 1..profile.len() {
        let slope = profile[i] - profile[i - 1];
        assert!(slope <= prev_slope + 1e-4, "wall slope should be non-increasing away from the channel at dist={}: {slope} > {prev_slope}", i + 2);
        prev_slope = slope;
    }
    // And genuinely convex, not linear: the first step must clearly out-climb
    // the last.
    let first = profile[1] - profile[0];
    let last = profile[profile.len() - 1] - profile[profile.len() - 2];
    assert!(first > last + 1e-3, "wall should be steepest at the bank: first step {first} vs last step {last}");
}

#[tokio::test]
async fn test_valley_shape_changes_profile() {
    let w = 128u32;
    let h = 128u32;
    let col = 64u32;
    let build = |shape: f32| {
        let mut inputs = default_inputs(13, w as i32, h as i32);
        set_image(&mut inputs, 3, vertical_stripe_mask(w, h, col));
        set(&mut inputs, 5, Value::Integer(8));
        set(&mut inputs, 6, Value::Integer(400));
        set(&mut inputs, 7, Value::Decimal(shape));
        set(&mut inputs, 8, Value::Decimal(1.0));
        set(&mut inputs, 9, Value::Decimal(0.0));
        inputs
    };
    let v_shape = OpImageSimulationGuidedRollingHills::run(&mut build(0.0)).await.unwrap();
    let u_shape = OpImageSimulationGuidedRollingHills::run(&mut build(1.0)).await.unwrap();
    assert_ne!(image_pixels(&v_shape, 0), image_pixels(&u_shape, 0), "valley shape should change the falloff profile");
}

#[tokio::test]
async fn test_bank_height_raises_levee() {
    let w = 128u32;
    let h = 128u32;
    let col = 64u32;
    // river_width_px=1, valley_width_px=50; d ~= 0.25 (levee peak) around
    // dist = 1 + 0.25*50 = 13.5, so x = col+14 lands near the peak.
    let build = |bank: f32| {
        let mut inputs = default_inputs(15, w as i32, h as i32);
        set_image(&mut inputs, 3, vertical_stripe_mask(w, h, col));
        set(&mut inputs, 5, Value::Integer(8));
        set(&mut inputs, 6, Value::Integer(400));
        set(&mut inputs, 9, Value::Decimal(bank));
        inputs
    };
    let low = OpImageSimulationGuidedRollingHills::run(&mut build(0.0)).await.unwrap();
    let high = OpImageSimulationGuidedRollingHills::run(&mut build(0.3)).await.unwrap();
    let row = 64usize;

    let near_bank_x = col as usize + 14;
    let v_low = pixel_at(&low, 0, near_bank_x, row, w as usize);
    let v_high = pixel_at(&high, 0, near_bank_x, row, w as usize);
    assert!(v_high > v_low, "bank height should raise the levee band: high {v_high} <= low {v_low}");

    // Channel itself stays exactly 0 regardless of bank height.
    assert_eq!(pixel_at(&low, 0, col as usize, row, w as usize), 0.0);
    assert_eq!(pixel_at(&high, 0, col as usize, row, w as usize), 0.0);
}

#[tokio::test]
async fn test_hills_suppressed_near_channel() {
    // Per-hill modulation, not a per-pixel fade: with river depth 0 and no
    // levee the height is purely the (bank-cut) hill field, so near-bank
    // hills must come out lower than the same hills unconnected, while
    // pixels beyond rim + max hill radius must be BIT-IDENTICAL to the
    // unconnected fallback (factor 1 past the rim + unmodulated-stats
    // normalization).
    let w = 128u32;
    let h = 128u32;
    let col = 64u32;
    // Small hills so an untouched far zone exists: density 8 -> 16px cells,
    // size 0.6 / variation 0 -> max radius 9.6px. River width 8 -> 1px,
    // valley width 200 -> 25px: rim at dist 26, fully-untouched from
    // dist >= 26 + 9.6 -> use 40 with slack.
    let build = |connected: bool| {
        let mut inputs = default_inputs(23, w as i32, h as i32);
        if connected {
            set_image(&mut inputs, 3, vertical_stripe_mask(w, h, col));
        }
        set(&mut inputs, 5, Value::Integer(8)); // river width -> 1px
        set(&mut inputs, 6, Value::Integer(200)); // valley width -> 25px
        set(&mut inputs, 8, Value::Decimal(0.0)); // river depth 0: hills only
        set(&mut inputs, 9, Value::Decimal(0.0)); // no levee
        set(&mut inputs, 10, Value::Decimal(8.0)); // density
        set(&mut inputs, 11, Value::Decimal(0.6)); // size
        set(&mut inputs, 12, Value::Decimal(0.0)); // size_variation
        inputs
    };
    let guided = OpImageSimulationGuidedRollingHills::run(&mut build(true)).await.unwrap();
    let fallback = OpImageSimulationGuidedRollingHills::run(&mut build(false)).await.unwrap();
    let gh = image_pixels(&guided, 0);
    let fh = image_pixels(&fallback, 0);
    let row_stride = w as usize;

    // (a) Channel pixels are cut to exactly 0 even with river depth 0.
    for y in 0..h as usize {
        for dx in -1i32..=1 {
            let x = (col as i32 + dx) as usize;
            assert_eq!(gh[y * row_stride + x][0], 0.0, "channel pixel ({x},{y}) should be cut to 0");
        }
    }

    // (b) Near-bank hills are suppressed relative to the fallback.
    let band_mean = |pixels: &Vec<Vec<f32>>| {
        let mut sum = 0.0f64;
        let mut count = 0usize;
        for y in 0..h as usize {
            for x in 0..w as usize {
                let dist = (x as i32 - col as i32).abs();
                if (4..=12).contains(&dist) {
                    sum += pixels[y * row_stride + x][0] as f64;
                    count += 1;
                }
            }
        }
        sum / count as f64
    };
    let guided_mean = band_mean(&gh);
    let fallback_mean = band_mean(&fh);
    assert!(guided_mean < fallback_mean - 0.05, "near-bank hills should be suppressed: guided {guided_mean} vs fallback {fallback_mean}");

    // (c) Beyond rim + max hill radius the output is bit-identical to the
    // fallback.
    for y in 0..h as usize {
        for x in 0..w as usize {
            if (x as i32 - col as i32).abs() >= 40 {
                assert_eq!(gh[y * row_stride + x][0], fh[y * row_stride + x][0], "pixel ({x},{y}) past the valley should match the fallback exactly");
            }
        }
    }
}

#[tokio::test]
async fn test_channel_mask_output() {
    let w = 128u32;
    let h = 128u32;
    let col = 64u32;
    let mut inputs = default_inputs(19, w as i32, h as i32);
    set_image(&mut inputs, 3, vertical_stripe_mask(w, h, col));
    set(&mut inputs, 5, Value::Integer(8)); // river width -> 1px
    set(&mut inputs, 6, Value::Integer(400)); // valley width -> 50px
    let result = OpImageSimulationGuidedRollingHills::run(&mut inputs).await.unwrap();
    let mask = image_pixels(&result, 1);
    let row = 64usize;

    let at_channel = mask[row * w as usize + col as usize][0];
    assert!((at_channel - 1.0).abs() < 1e-5, "channel mask should be fully bright at the channel, got {at_channel}");

    let at_rim = mask[row * w as usize + (col as usize + 60)][0];
    assert_eq!(at_rim, 0.0, "channel mask should be black past the valley rim, got {at_rim}");

    let mut prev = at_channel;
    for x in (col as usize + 1)..w as usize {
        let v = mask[row * w as usize + x][0];
        assert!(v <= prev + 1e-6, "channel mask should be non-increasing away from the channel at x={x}");
        prev = v;
    }
}
