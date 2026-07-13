use super::*;

use crate::input::Input;
use crate::operations::images::noise::cellular::rolling_hills::OpImageNoiseRollingHills;
use crate::value::Value;

/// Builds the 17 default inputs: no guidance map connected, mask mode, hill
/// params matching rolling hills' own defaults (so the unconnected fallback
/// can be compared pixel-for-pixel against that node).
fn default_inputs(seed: i32, width: i32, height: i32) -> Vec<Input> {
    vec![
        Input::new("seed".to_string(), Value::Integer(seed), None, None),
        Input::new("width".to_string(), Value::Integer(width), None, None),
        Input::new("height".to_string(), Value::Integer(height), None, None),
        Input::new("guidance map".to_string(), Value::Image { data: default_image(), change_id: get_id() }, None, None),
        Input::new("map is distance field".to_string(), Value::Bool(false), None, None),
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

/// White single-pixel-wide vertical stripe at column `col` on an otherwise
/// black image, used as a synthetic river mask.
fn vertical_stripe_mask(width: u32, height: u32, col: u32) -> FloatImage {
    let mut img = FloatImage::new(width, height, 1);
    for y in 0..height {
        img.put_pixel(col, y, &[1.0]);
    }
    img
}

/// Horizontal gradient image: value = x / (width - 1) across every row, used
/// as a synthetic precomputed distance field.
fn horizontal_gradient(width: u32, height: u32) -> FloatImage {
    let mut img = FloatImage::new(width, height, 1);
    for y in 0..height {
        for x in 0..width {
            img.put_pixel(x, y, &[x as f32 / (width - 1) as f32]);
        }
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
    assert_eq!(OpImageSimulationGuidedRollingHills::create_inputs().len(), 17);
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

    let mut connected_black = default_inputs(5, 24, 24);
    set_image(&mut connected_black, 3, FloatImage::new(24, 24, 1));
    let connected_result = OpImageSimulationGuidedRollingHills::run(&mut connected_black).await.unwrap();

    assert_eq!(image_pixels(&unconnected_result, 0), image_pixels(&connected_result, 0), "all-black connected mask should fall back to plain hills");
    assert_eq!(image_pixels(&unconnected_result, 1), image_pixels(&connected_result, 1), "channel mask should still be all black");
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
    set(&mut inputs, 6, Value::Integer(16));
    set(&mut inputs, 7, Value::Integer(128));
    let result = OpImageSimulationGuidedRollingHills::run(&mut inputs).await.unwrap();
    let height = image_pixels(&result, 0);

    // Channel pixels (within river width of the stripe) must be exactly 0:
    // at d=0 the hill-modulation and levee terms both vanish regardless of
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
    set(&mut inputs, 6, Value::Integer(8)); // river width -> 1px
    set(&mut inputs, 7, Value::Integer(400)); // valley width -> 50px
    set(&mut inputs, 9, Value::Decimal(1.0)); // river depth = 1: composed == ramp
    set(&mut inputs, 10, Value::Decimal(0.0)); // bank height = 0: no levee bump
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
async fn test_valley_shape_changes_profile() {
    let w = 128u32;
    let h = 128u32;
    let col = 64u32;
    let build = |shape: f32| {
        let mut inputs = default_inputs(13, w as i32, h as i32);
        set_image(&mut inputs, 3, vertical_stripe_mask(w, h, col));
        set(&mut inputs, 6, Value::Integer(8));
        set(&mut inputs, 7, Value::Integer(400));
        set(&mut inputs, 8, Value::Decimal(shape));
        set(&mut inputs, 9, Value::Decimal(1.0));
        set(&mut inputs, 10, Value::Decimal(0.0));
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
        set(&mut inputs, 6, Value::Integer(8));
        set(&mut inputs, 7, Value::Integer(400));
        set(&mut inputs, 10, Value::Decimal(bank));
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
async fn test_distance_field_mode() {
    let w = 32u32;
    let h = 32u32;
    let grad = horizontal_gradient(w, h);
    let mut inputs = default_inputs(17, w as i32, h as i32);
    set_image(&mut inputs, 3, grad.clone());
    set(&mut inputs, 4, Value::Bool(true));
    let result = OpImageSimulationGuidedRollingHills::run(&mut inputs).await.unwrap();
    let row = 16usize;

    let bright = pixel_at(&result, 0, w as usize - 1, row, w as usize);
    let dark = pixel_at(&result, 0, 0, row, w as usize);
    assert!(bright < 0.05, "bright side (g~=1) should be near 0, got {bright}");
    assert!(dark > bright + 0.1, "dark side (g~=0) should be substantially higher, got dark={dark} bright={bright}");

    // Mask-mode-only params must have no effect in distance-field mode.
    let mut inputs2 = default_inputs(17, w as i32, h as i32);
    set_image(&mut inputs2, 3, grad);
    set(&mut inputs2, 4, Value::Bool(true));
    set(&mut inputs2, 5, Value::Decimal(0.9)); // mask threshold, ignored
    set(&mut inputs2, 6, Value::Integer(64)); // river width, ignored
    set(&mut inputs2, 7, Value::Integer(300)); // valley width, ignored
    let result2 = OpImageSimulationGuidedRollingHills::run(&mut inputs2).await.unwrap();
    assert_eq!(image_pixels(&result, 0), image_pixels(&result2, 0), "mask-only params should be ignored in distance-field mode");
    assert_eq!(image_pixels(&result, 1), image_pixels(&result2, 1));
}

#[tokio::test]
async fn test_channel_mask_output() {
    let w = 128u32;
    let h = 128u32;
    let col = 64u32;
    let mut inputs = default_inputs(19, w as i32, h as i32);
    set_image(&mut inputs, 3, vertical_stripe_mask(w, h, col));
    set(&mut inputs, 6, Value::Integer(8)); // river width -> 1px
    set(&mut inputs, 7, Value::Integer(400)); // valley width -> 50px
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
