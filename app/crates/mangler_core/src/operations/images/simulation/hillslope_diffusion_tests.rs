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
    iterations: i32,
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
        Input::new("iterations".to_string(), Value::Integer(iterations), None, None),
        Input::new("creep rate".to_string(), Value::Decimal(0.5), None, None),
        Input::new("critical slope".to_string(), Value::Decimal(1.0), None, None),
        Input::new("octaves".to_string(), Value::Integer(6), None, None),
        Input::new("frequency".to_string(), Value::Decimal(5.0), None, None),
    ]
}

/// Default inputs with an unconnected height map and a small iteration count.
fn default_inputs(seed: i32, width: i32, height: i32) -> Vec<Input> {
    make_inputs(seed, width, height, None, 50)
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
    let s = OpImageSimulationHillslopeDiffusion::settings();
    assert_eq!(s.name, "hillslope diffusion");
    assert_eq!(OpImageSimulationHillslopeDiffusion::create_inputs().len(), 9);
    assert_eq!(OpImageSimulationHillslopeDiffusion::create_outputs().len(), 1);
}

#[tokio::test]
async fn test_run_basic() {
    let mut inputs = default_inputs(1, 16, 16);
    let result = OpImageSimulationHillslopeDiffusion::run(&mut inputs).await;
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
    let result = OpImageSimulationHillslopeDiffusion::run(&mut inputs).await.unwrap();
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
    let r1 = OpImageSimulationHillslopeDiffusion::run(&mut default_inputs(7, 16, 16)).await.unwrap();
    let r2 = OpImageSimulationHillslopeDiffusion::run(&mut default_inputs(7, 16, 16)).await.unwrap();
    assert_eq!(
        image_pixels(&r1, 0),
        image_pixels(&r2, 0),
        "height output is not deterministic"
    );
}

#[tokio::test]
async fn test_different_seeds_differ() {
    let r1 = OpImageSimulationHillslopeDiffusion::run(&mut default_inputs(1, 16, 16)).await.unwrap();
    let r2 = OpImageSimulationHillslopeDiffusion::run(&mut default_inputs(42, 16, 16)).await.unwrap();
    assert_ne!(
        image_pixels(&r1, 0),
        image_pixels(&r2, 0),
        "different seeds should produce different height output"
    );
}

#[tokio::test]
async fn test_zero_iterations_passthrough() {
    // With zero iterations the diffusion loop never runs, so the height
    // output must equal the normalized starting terrain. The ramp already
    // spans 0..=1, so normalization is identity and only the sRGB encode
    // separates the raw ramp values from the output pixels.
    let ramp = ramp_image(16, 16);
    let expected: Vec<Vec<f32>> = ramp
        .pixels()
        .map(|p| vec![crate::color::color_spaces::rgb_linear::linear_to_nonlinear_srgb(p[0])])
        .collect();

    let mut inputs = make_inputs(1, 16, 16, Some(ramp), 0);
    let result = OpImageSimulationHillslopeDiffusion::run(&mut inputs).await.unwrap();
    let actual = image_pixels(&result, 0);

    assert_eq!(actual.len(), expected.len());
    for (a, e) in actual.iter().zip(expected.iter()) {
        assert_eq!(a.len(), e.len());
        for (av, ev) in a.iter().zip(e.iter()) {
            assert!(
                (av - ev).abs() < 1e-4,
                "zero-iteration output should match the normalized starting terrain: got {av}, expected {ev}"
            );
        }
    }
}

#[tokio::test]
async fn test_smoothing_reduces_variation() {
    // Total variation (sum of absolute right- and below-neighbor
    // differences) of the fallback terrain should drop after diffusion
    // rounds crests and relaxes slopes.
    fn total_variation(pixels: &[Vec<f32>], w: usize, h: usize) -> f64 {
        let mut tv = 0.0;
        for y in 0..h {
            for x in 0..w {
                let v = pixels[y * w + x][0] as f64;
                if x + 1 < w {
                    tv += (pixels[y * w + x + 1][0] as f64 - v).abs();
                }
                if y + 1 < h {
                    tv += (pixels[(y + 1) * w + x][0] as f64 - v).abs();
                }
            }
        }
        tv
    }

    let r0 = OpImageSimulationHillslopeDiffusion::run(&mut make_inputs(3, 64, 64, None, 0)).await.unwrap();
    let r200 = OpImageSimulationHillslopeDiffusion::run(&mut make_inputs(3, 64, 64, None, 200)).await.unwrap();

    let tv0 = total_variation(&image_pixels(&r0, 0), 64, 64);
    let tv200 = total_variation(&image_pixels(&r200, 0), 64, 64);

    assert!(
        tv200 < tv0,
        "diffusion should reduce total variation: tv0={tv0}, tv200={tv200}"
    );
}

#[tokio::test]
async fn test_connected_map_used() {
    // A connected ramp is the starting terrain; after 200 iterations the
    // diffused height must differ from the 0-iteration (passthrough) output.
    let ramp = ramp_image(16, 16);
    let r0 = OpImageSimulationHillslopeDiffusion::run(&mut make_inputs(3, 16, 16, Some(ramp.clone()), 0)).await.unwrap();
    let r200 = OpImageSimulationHillslopeDiffusion::run(&mut make_inputs(3, 16, 16, Some(ramp), 200)).await.unwrap();

    assert_ne!(
        image_pixels(&r0, 0),
        image_pixels(&r200, 0),
        "diffused height should differ from the connected starting terrain"
    );
}

/// Renders a 256x256 BMP of the diffused height at the TRUE defaults from
/// `create_inputs()` (width/height overridden to 256) and reports the render
/// time, plus a 2x2 tiling mosaic.
/// Run with `cargo test -p mangler_core hillslope_diffusion::tests::render_preview -- --ignored --nocapture`.
#[tokio::test]
#[ignore]
async fn render_preview() {
    let mut inputs = OpImageSimulationHillslopeDiffusion::create_inputs();
    inputs[1].value = Value::Integer(256);
    inputs[2].value = Value::Integer(256);
    let start = std::time::Instant::now();
    let result = OpImageSimulationHillslopeDiffusion::run(&mut inputs).await.unwrap();
    println!("hillslope diffusion 256x256 default render: {:?}", start.elapsed());

    let dir = "/private/tmp/claude-501/-Users-danielphillips-rust-nodemangler/51ed0e89-32e9-4cf5-83e1-33f0c21f455e/scratchpad/previews";
    std::fs::create_dir_all(dir).ok();
    let data = match &result.responses[0].value {
        Value::Image { data, .. } => { data.to_dynamic().save(format!("{dir}/hillslope_height.bmp")).unwrap(); data.clone() }
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
    tile.to_dynamic().save(format!("{dir}/hillslope_height_tile.bmp")).unwrap();
}
