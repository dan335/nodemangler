use super::*;

use crate::input::Input;
use crate::value::Value;

/// Helper to create inputs with the given parameters. `height_map` of `None`
/// leaves the image input at its unconnected 1x1 placeholder.
fn make_inputs(
    seed: i32,
    width: i32,
    height: i32,
    height_map: Option<FloatImage>,
    droplets: i32,
) -> Vec<Input> {
    let map_value = match height_map {
        Some(img) => Value::Image { data: Arc::new(img), change_id: get_id() },
        None => Value::Image { data: default_image(), change_id: get_id() },
    };
    vec![
        Input::new("seed".to_string(), Value::Integer(seed), None, None),
        Input::new("width".to_string(), Value::Integer(width), None, None),
        Input::new("height".to_string(), Value::Integer(height), None, None),
        Input::new("height map".to_string(), map_value, None, None),
        Input::new("droplets".to_string(), Value::Integer(droplets), None, None),
        Input::new("capacity".to_string(), Value::Decimal(4.0), None, None),
        Input::new("erosion rate".to_string(), Value::Decimal(0.3), None, None),
        Input::new("deposition rate".to_string(), Value::Decimal(0.3), None, None),
        Input::new("lifetime".to_string(), Value::Integer(48), None, None),
        Input::new("erosion radius".to_string(), Value::Integer(2), None, None),
        Input::new("inertia".to_string(), Value::Decimal(0.05), None, None),
        Input::new("evaporation".to_string(), Value::Decimal(0.02), None, None),
        Input::new("octaves".to_string(), Value::Integer(4), None, None),
        Input::new("frequency".to_string(), Value::Decimal(3.0), None, None),
    ]
}

/// Default inputs with an unconnected height map and a small droplet count.
fn default_inputs(seed: i32, width: i32, height: i32) -> Vec<Input> {
    make_inputs(seed, width, height, None, 500)
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

/// Extracts the pixel data of the response at `index`.
fn image_pixels(result: &OperationResponse, index: usize) -> Vec<Vec<f32>> {
    match &result.responses[index].value {
        Value::Image { data, .. } => data.pixels().map(|p| p.to_vec()).collect(),
        other => panic!("Expected Image, got {:?}", other),
    }
}

#[tokio::test]
async fn test_settings() {
    let s = OpImageSimulationHydraulicErosion::settings();
    assert_eq!(s.name, "hydraulic erosion");
    assert_eq!(OpImageSimulationHydraulicErosion::create_inputs().len(), 14);
    assert_eq!(OpImageSimulationHydraulicErosion::create_outputs().len(), 1);
}

#[tokio::test]
async fn test_run_basic() {
    let mut inputs = default_inputs(1, 16, 16);
    let result = OpImageSimulationHydraulicErosion::run(&mut inputs).await;
    assert!(result.is_ok(), "run failed: {:?}", result.err());
    let result = result.unwrap();
    assert_eq!(result.responses.len(), 1);
    match &result.responses[0].value {
        Value::Image { .. } => {}
        other => panic!("Expected Image, got {:?}", other),
    }
}

#[tokio::test]
async fn test_correct_dimensions() {
    let mut inputs = default_inputs(1, 32, 16);
    let result = OpImageSimulationHydraulicErosion::run(&mut inputs).await.unwrap();
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
    let r1 = OpImageSimulationHydraulicErosion::run(&mut default_inputs(7, 16, 16)).await.unwrap();
    let r2 = OpImageSimulationHydraulicErosion::run(&mut default_inputs(7, 16, 16)).await.unwrap();
    assert_eq!(
        image_pixels(&r1, 0),
        image_pixels(&r2, 0),
        "height output is not deterministic"
    );
}

#[tokio::test]
async fn test_different_seeds_differ() {
    let r1 = OpImageSimulationHydraulicErosion::run(&mut default_inputs(1, 16, 16)).await.unwrap();
    let r2 = OpImageSimulationHydraulicErosion::run(&mut default_inputs(42, 16, 16)).await.unwrap();
    assert_ne!(
        image_pixels(&r1, 0),
        image_pixels(&r2, 0),
        "different seeds should produce different height output"
    );
}

#[tokio::test]
async fn test_connected_height_map_erodes() {
    // A connected ramp is the starting terrain; after 2000 droplets the
    // eroded height must differ from the normalized starting terrain.
    let ramp = ramp_image(16, 16);
    let expected_start: Vec<Vec<f32>> = ramp
        .pixels()
        .map(|p| vec![crate::color::color_spaces::rgb_linear::linear_to_nonlinear_srgb(p[0])])
        .collect();

    let mut inputs = make_inputs(3, 16, 16, Some(ramp), 2000);
    let result = OpImageSimulationHydraulicErosion::run(&mut inputs).await;
    assert!(result.is_ok(), "run failed: {:?}", result.err());
    let result = result.unwrap();

    assert_ne!(
        image_pixels(&result, 0),
        expected_start,
        "eroded height should differ from the starting terrain"
    );
}

#[tokio::test]
async fn test_zero_droplets_is_noop() {
    // With no droplets the height output must equal the normalized starting
    // terrain exactly (the ramp already spans 0..=1, so normalization is
    // identity).
    let ramp = ramp_image(16, 16);
    let expected: Vec<Vec<f32>> = ramp
        .pixels()
        .map(|p| vec![crate::color::color_spaces::rgb_linear::linear_to_nonlinear_srgb(p[0])])
        .collect();

    let mut inputs = make_inputs(1, 16, 16, Some(ramp), 0);
    let result = OpImageSimulationHydraulicErosion::run(&mut inputs).await.unwrap();

    assert_eq!(
        image_pixels(&result, 0),
        expected,
        "zero droplets should leave the normalized terrain unchanged"
    );
}

/// Renders a 512x512 PNG of the eroded height at the TRUE defaults from
/// `create_inputs()` and reports the render time, plus a 2x2 tiling mosaic.
/// Run with `cargo test -p mangler_core hydraulic_erosion::tests::render_preview -- --ignored --nocapture`.
#[tokio::test]
#[ignore]
async fn render_preview() {
    let mut inputs = OpImageSimulationHydraulicErosion::create_inputs();
    let start = std::time::Instant::now();
    let result = OpImageSimulationHydraulicErosion::run(&mut inputs).await.unwrap();
    println!("hydraulic erosion 512x512 default render: {:?}", start.elapsed());

    let dir = "/private/tmp/claude-501/-Users-danielphillips-rust-nodemangler/36d7531d-c5e6-4f56-ae8b-faf4639bad40/scratchpad/previews";
    let data = match &result.responses[0].value {
        Value::Image { data, .. } => { data.to_dynamic().save(format!("{dir}/hydro_height.png")).unwrap(); data.clone() }
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
    tile.to_dynamic().save(format!("{dir}/hydro_height_tile.png")).unwrap();
}
