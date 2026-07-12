use super::*;

use crate::input::Input;
use crate::value::Value;

/// Builds the 14 default inputs. `height_map` / `mask` of `None` leave the
/// image input at its unconnected 1x1 placeholder. Individual tests mutate
/// entries by index afterwards.
fn make_inputs(
    width: i32,
    height: i32,
    height_map: Option<FloatImage>,
    mask: Option<FloatImage>,
) -> Vec<Input> {
    let map_value = match height_map {
        Some(img) => Value::Image { data: Arc::new(img), change_id: get_id() },
        None => Value::Image { data: default_image(), change_id: get_id() },
    };
    let mask_value = match mask {
        Some(img) => Value::Image { data: Arc::new(img), change_id: get_id() },
        None => Value::Image { data: default_image(), change_id: get_id() },
    };
    vec![
        Input::new("seed".to_string(), Value::Integer(1), None, None),
        Input::new("width".to_string(), Value::Integer(width), None, None),
        Input::new("height".to_string(), Value::Integer(height), None, None),
        Input::new("height map".to_string(), map_value, None, None),
        Input::new("river mask".to_string(), mask_value, None, None),
        Input::new("mask threshold".to_string(), Value::Decimal(0.5), None, None),
        Input::new("carve depth".to_string(), Value::Decimal(0.15), None, None),
        Input::new("river width".to_string(), Value::Integer(6), None, None),
        Input::new("valley width".to_string(), Value::Integer(48), None, None),
        Input::new("valley shape".to_string(), Value::Decimal(0.5), None, None),
        Input::new("bank smoothing".to_string(), Value::Integer(2), None, None),
        Input::new("monotonic bed".to_string(), Value::Bool(true), None, None),
        Input::new("octaves".to_string(), Value::Integer(6), None, None),
        Input::new("frequency".to_string(), Value::Decimal(5.0), None, None),
    ]
}

/// Overwrites input `idx` with a new value, keeping the same name.
fn set(inputs: &mut [Input], idx: usize, value: Value) {
    inputs[idx] = Input::new(inputs[idx].name.clone(), value, None, None);
}

/// Builds a single-channel horizontal ramp image with values 0..=1 across x.
fn ramp_image(width: u32, height: u32) -> FloatImage {
    let mut img = FloatImage::new(width, height, 1);
    for y in 0..height {
        for x in 0..width {
            img.put_pixel(x, y, &[x as f32 / (width - 1) as f32]);
        }
    }
    img
}

/// Builds a black single-channel image with a single white horizontal line at
/// row `row`.
fn horizontal_line_mask(width: u32, height: u32, row: u32) -> FloatImage {
    let mut img = FloatImage::new(width, height, 1);
    for x in 0..width {
        img.put_pixel(x, row, &[1.0]);
    }
    img
}

/// Builds a black single-channel image with a white diagonal segment from
/// (start, start) to (end, end) inclusive.
fn diagonal_mask(width: u32, height: u32, start: u32, end: u32) -> FloatImage {
    let mut img = FloatImage::new(width, height, 1);
    for i in start..=end {
        img.put_pixel(i, i, &[1.0]);
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
    let s = OpImageSimulationCarveRiver::settings();
    assert_eq!(s.name, "carve river");
    assert_eq!(OpImageSimulationCarveRiver::create_inputs().len(), 14);
    assert_eq!(OpImageSimulationCarveRiver::create_outputs().len(), 2);
}

#[tokio::test]
async fn test_correct_dimensions() {
    let mask = horizontal_line_mask(32, 16, 8);
    let mut inputs = make_inputs(32, 16, None, Some(mask));
    let result = OpImageSimulationCarveRiver::run(&mut inputs).await.unwrap();
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
        let mask = diagonal_mask(32, 32, 4, 27);
        make_inputs(32, 32, None, Some(mask))
    };
    let r1 = OpImageSimulationCarveRiver::run(&mut build()).await.unwrap();
    let r2 = OpImageSimulationCarveRiver::run(&mut build()).await.unwrap();
    assert_eq!(image_pixels(&r1, 0), image_pixels(&r2, 0), "height output not deterministic");
    assert_eq!(image_pixels(&r1, 1), image_pixels(&r2, 1), "water depth output not deterministic");
}

#[tokio::test]
async fn test_unconnected_mask_is_passthrough() {
    // Connected ramp terrain spanning the full 0..1 range (normalization is
    // identity), mask unconnected: the height output must equal the sRGB-
    // encoded ramp exactly and the water depth must be all black.
    let ramp = ramp_image(32, 32);
    let expected: Vec<Vec<f32>> = ramp
        .pixels()
        .map(|p| vec![crate::color::color_spaces::rgb_linear::linear_to_nonlinear_srgb(p[0])])
        .collect();

    let mut inputs = make_inputs(32, 32, Some(ramp), None);
    let result = OpImageSimulationCarveRiver::run(&mut inputs).await.unwrap();

    let height = image_pixels(&result, 0);
    assert_eq!(height.len(), expected.len());
    for (i, (got, want)) in height.iter().zip(expected.iter()).enumerate() {
        assert!((got[0] - want[0]).abs() < 1e-5, "pixel {i}: got {}, want {}", got[0], want[0]);
    }
    for (i, p) in image_pixels(&result, 1).iter().enumerate() {
        assert_eq!(p[0], 0.0, "depth pixel {i} should be black");
    }
}

#[tokio::test]
async fn test_monotonic_bed() {
    // Terrain rising left→right (0..1), a 1px river line across the middle row.
    // River width 0 keeps the channel exactly one pixel wide so the along-
    // channel receiver chain is unambiguous and the border outlet is reached.
    let w = 32usize;
    let row = 16usize;
    let make = |monotonic: bool| {
        let ramp = ramp_image(32, 32);
        let mask = horizontal_line_mask(32, 32, row as u32);
        let mut inputs = make_inputs(32, 32, Some(ramp), Some(mask));
        set(&mut inputs, 6, Value::Decimal(0.15)); // carve depth
        set(&mut inputs, 7, Value::Integer(0)); // river width 0 -> 1px channel
        set(&mut inputs, 8, Value::Integer(0)); // valley width off
        set(&mut inputs, 10, Value::Integer(0)); // bank smoothing off
        set(&mut inputs, 11, Value::Bool(monotonic));
        inputs
    };

    let on = OpImageSimulationCarveRiver::run(&mut make(true)).await.unwrap();
    let off = OpImageSimulationCarveRiver::run(&mut make(false)).await.unwrap();

    // Enforcement ON: the water line cannot rise downstream toward the right
    // outlet, so the carved bed at the right end sits at or below the middle.
    let on_mid = pixel_at(&on, 0, 16, row, w);
    let on_right = pixel_at(&on, 0, 31, row, w);
    assert!(on_right <= on_mid + 1e-4, "monotonic ON: right {on_right} should be <= middle {on_mid}");

    // Enforcement OFF: the naive carve just follows the rising terrain, so the
    // right end is higher than the middle.
    let off_mid = pixel_at(&off, 0, 16, row, w);
    let off_right = pixel_at(&off, 0, 31, row, w);
    assert!(off_right > off_mid, "monotonic OFF: right {off_right} should be > middle {off_mid}");
}

#[tokio::test]
async fn test_never_raises() {
    // Ramp terrain spanning 0..1 with a diagonal line kept in the interior
    // (columns 0 and 31 stay untouched, so both runs normalize identically to
    // identity). Every carved pixel must be <= the passthrough pixel.
    let w = 32usize;
    let ramp = ramp_image(32, 32);
    let mut pass_inputs = make_inputs(32, 32, Some(ramp), None);
    set(&mut pass_inputs, 10, Value::Integer(0));
    let passthrough = OpImageSimulationCarveRiver::run(&mut pass_inputs).await.unwrap();
    let pass = image_pixels(&passthrough, 0);

    let ramp2 = ramp_image(32, 32);
    let mask = diagonal_mask(32, 32, 8, 23);
    let mut inputs = make_inputs(32, 32, Some(ramp2), Some(mask));
    set(&mut inputs, 7, Value::Integer(0)); // river width 0 (exact pixels)
    set(&mut inputs, 8, Value::Integer(64)); // valley width ~2px, stays interior
    set(&mut inputs, 10, Value::Integer(0)); // bank smoothing off
    let carved = OpImageSimulationCarveRiver::run(&mut inputs).await.unwrap();
    let got = image_pixels(&carved, 0);

    for (i, (c, p)) in got.iter().zip(pass.iter()).enumerate() {
        assert!(c[0] <= p[0] + 1e-4, "pixel {i}: carved {} raised above passthrough {}", c[0], p[0]);
    }
    let _ = w;
}

#[tokio::test]
async fn test_far_terrain_untouched() {
    // Small river + valley widths with a central diagonal: pixels far from the
    // mask must match the passthrough exactly. Corners stay untouched so both
    // runs normalize identically.
    let w = 32usize;
    let h = 32usize;
    let start = 12u32;
    let end = 19u32;

    let ramp = ramp_image(32, 32);
    let mut pass_inputs = make_inputs(32, 32, Some(ramp), None);
    set(&mut pass_inputs, 10, Value::Integer(0));
    let passthrough = OpImageSimulationCarveRiver::run(&mut pass_inputs).await.unwrap();
    let pass = image_pixels(&passthrough, 0);

    let ramp2 = ramp_image(32, 32);
    let mask = diagonal_mask(32, 32, start, end);
    let mut inputs = make_inputs(32, 32, Some(ramp2), Some(mask));
    set(&mut inputs, 7, Value::Integer(0)); // river width 0
    set(&mut inputs, 8, Value::Integer(64)); // valley width ~2px
    set(&mut inputs, 10, Value::Integer(0)); // bank smoothing off (no spread)
    let carved = OpImageSimulationCarveRiver::run(&mut inputs).await.unwrap();
    let got = image_pixels(&carved, 0);

    // A pixel whose distance to the whole diagonal segment exceeds
    // river + valley width (2px) must be identical to the passthrough. Use a
    // comfortable margin of 6px.
    let far = |x: usize, y: usize| -> bool {
        (start..=end).all(|i| {
            let dx = x as f64 - i as f64;
            let dy = y as f64 - i as f64;
            (dx * dx + dy * dy).sqrt() > 6.0
        })
    };
    for y in 0..h {
        for x in 0..w {
            if far(x, y) {
                let idx = y * w + x;
                assert!((got[idx][0] - pass[idx][0]).abs() < 1e-5,
                    "far pixel ({x},{y}) changed: {} vs {}", got[idx][0], pass[idx][0]);
            }
        }
    }
}

#[tokio::test]
async fn test_water_depth_only_in_channel() {
    // Horizontal river line at row 8, ~2px channel. Water depth must be zero
    // outside the channel band (rows 6..=10) and positive somewhere inside.
    let w = 32usize;
    let h = 32usize;
    let row = 8usize;
    let mask = horizontal_line_mask(32, 32, row as u32);
    let mut inputs = make_inputs(32, 32, None, Some(mask));
    set(&mut inputs, 6, Value::Decimal(0.2)); // carve depth
    set(&mut inputs, 7, Value::Integer(64)); // river width -> ~2px
    set(&mut inputs, 8, Value::Integer(0)); // valley width off
    set(&mut inputs, 10, Value::Integer(0)); // bank smoothing off
    let result = OpImageSimulationCarveRiver::run(&mut inputs).await.unwrap();
    let depth = image_pixels(&result, 1);

    let mut any_positive = false;
    for y in 0..h {
        for x in 0..w {
            let v = depth[y * w + x][0];
            let in_band = (row as i64 - y as i64).abs() <= 2;
            if !in_band {
                assert_eq!(v, 0.0, "depth outside channel at ({x},{y}) should be 0");
            } else if v > 0.0 {
                any_positive = true;
            }
        }
    }
    assert!(any_positive, "expected some positive water depth inside the channel");
}

/// Renders a 512x512 carve over fallback terrain with a bent (sine) river path
/// and saves the height + depth PNGs. Run with
/// `cargo test -p mangler_core carve_river::tests::render_preview -- --ignored --nocapture`.
#[tokio::test]
#[ignore]
async fn render_preview() {
    let w = 512u32;
    let h = 512u32;
    // Bent sine river path drawn as a thick polyline.
    let mut mask = FloatImage::new(w, h, 1);
    for x in 0..w {
        let t = x as f64 / w as f64;
        let cy = 256.0 + 140.0 * (t * std::f64::consts::TAU).sin();
        for dy in -2i64..=2 {
            let y = cy as i64 + dy;
            if y >= 0 && y < h as i64 {
                mask.put_pixel(x, y as u32, &[1.0]);
            }
        }
    }

    let mut inputs = make_inputs(w as i32, h as i32, None, Some(mask));
    set(&mut inputs, 0, Value::Integer(7));
    let start = std::time::Instant::now();
    let result = OpImageSimulationCarveRiver::run(&mut inputs).await.unwrap();
    println!("carve river 512x512 render: {:?}", start.elapsed());

    let dir = "/private/tmp/claude-501/-Users-danielphillips-rust-nodemangler/98a53923-dfe7-4d74-844a-01a99e2322ea/scratchpad";
    match &result.responses[0].value {
        Value::Image { data, .. } => { data.to_dynamic().save(format!("{dir}/carve_river_height.png")).unwrap(); }
        other => panic!("Expected Image, got {other:?}"),
    }
    match &result.responses[1].value {
        Value::Image { data, .. } => { data.to_dynamic().save(format!("{dir}/carve_river_depth.png")).unwrap(); }
        other => panic!("Expected Image, got {other:?}"),
    }
}
