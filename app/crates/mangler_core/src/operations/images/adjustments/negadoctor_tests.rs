//! Tests for the negadoctor (film-negative inversion) operation.

use super::*;

use crate::color::Color;
use crate::float_image::FloatImage;
use crate::get_id;
use crate::input::Input;
use crate::value::Value;
use std::sync::Arc;

fn inputs(image: Value, base: Color, dynamic_range: f32, brightness: f32) -> Vec<Input> {
    vec![
        Input::new("image".to_string(), image, None, None),
        Input::new("film base".to_string(), Value::Color(base), None, None),
        Input::new("dynamic range".to_string(), Value::Decimal(dynamic_range as f32), None, None),
        Input::new("brightness".to_string(), Value::Decimal(brightness as f32), None, None),
    ]
}

fn solid_image(w: u32, h: u32, rgba: [f32; 4]) -> Arc<FloatImage> {
    Arc::new(FloatImage::from_pixel(w, h, 4, &rgba))
}

const DEFAULT_BASE: Color = Color { r: 1.0, g: 0.55, b: 0.32, a: 1.0 };

#[tokio::test]
async fn settings_and_ports() {
    assert_eq!(OpImageAdjustmentNegadoctor::settings().name, "negadoctor");
    assert_eq!(OpImageAdjustmentNegadoctor::create_inputs().len(), 4);
    assert_eq!(OpImageAdjustmentNegadoctor::create_outputs().len(), 1);
}

#[tokio::test]
async fn film_base_pixel_maps_to_black() {
    let src = solid_image(2, 2, [DEFAULT_BASE.r, DEFAULT_BASE.g, DEFAULT_BASE.b, 1.0]);
    let mut ins = inputs(Value::Image { data: src, change_id: get_id() }, DEFAULT_BASE, 1.5, 0.0);
    let result = OpImageAdjustmentNegadoctor::run(&mut ins).await.unwrap();
    let Value::Image { data, .. } = &result.responses[0].value else { panic!() };
    let px = data.get_pixel(0, 0);
    for c in 0..3 {
        assert!(px[c] < 0.01, "film-base pixel should map near black on channel {c}: {px:?}");
    }
}

#[tokio::test]
async fn dark_negative_pixel_maps_bright() {
    // A near-black negative pixel (dense, i.e. far from the film base) should
    // invert to a near-white positive pixel.
    let src = solid_image(2, 2, [0.001, 0.001, 0.001, 1.0]);
    let mut ins = inputs(Value::Image { data: src, change_id: get_id() }, DEFAULT_BASE, 1.5, 0.0);
    let result = OpImageAdjustmentNegadoctor::run(&mut ins).await.unwrap();
    let Value::Image { data, .. } = &result.responses[0].value else { panic!() };
    let px = data.get_pixel(0, 0);
    for c in 0..3 {
        assert!(px[c] > 0.95, "near-black negative pixel should invert near-white on channel {c}: {px:?}");
    }
}

#[tokio::test]
async fn output_is_monotonically_decreasing_in_input() {
    // Uniform base so all channels behave identically; sample increasing
    // input values approaching the base and confirm the output strictly
    // decreases (using gamma=1 to stay clear of the [0,1] clamp).
    let base = Color { r: 1.0, g: 1.0, b: 1.0, a: 1.0 };
    // Stay above ~base/2 so `ratio - 1` doesn't saturate at the [0,1] clamp
    // (below that the output is pinned at 1 for every sample, which is
    // still monotonic non-increasing but not useful for a strict check).
    let samples = [0.55, 0.65, 0.75, 0.85, 0.95, 0.99];
    let mut outputs = Vec::new();
    for &v in &samples {
        let src = solid_image(1, 1, [v, v, v, 1.0]);
        let mut ins = inputs(Value::Image { data: src, change_id: get_id() }, base, 1.0, 0.0);
        let result = OpImageAdjustmentNegadoctor::run(&mut ins).await.unwrap();
        let Value::Image { data, .. } = &result.responses[0].value else { panic!() };
        outputs.push(data.get_pixel(0, 0)[0]);
    }
    for w in outputs.windows(2) {
        assert!(w[0] > w[1], "output should decrease monotonically as input rises: {outputs:?}");
    }
}

#[tokio::test]
async fn alpha_preserved_on_rgba() {
    let src = solid_image(3, 3, [0.4, 0.5, 0.6, 0.77]);
    let mut ins = inputs(Value::Image { data: src, change_id: get_id() }, DEFAULT_BASE, 1.5, 0.2);
    let result = OpImageAdjustmentNegadoctor::run(&mut ins).await.unwrap();
    let Value::Image { data, .. } = &result.responses[0].value else { panic!() };
    for y in 0..3 {
        for x in 0..3 {
            assert!((data.get_pixel(x, y)[3] - 0.77).abs() < 1e-6, "alpha changed at ({x},{y})");
        }
    }
}

#[tokio::test]
async fn grayscale_uses_base_luma() {
    let base_luma = 0.2126 * DEFAULT_BASE.r + 0.7152 * DEFAULT_BASE.g + 0.0722 * DEFAULT_BASE.b;
    let img = FloatImage::from_pixel(2, 2, 1, &[base_luma]);
    let mut ins = inputs(Value::Image { data: Arc::new(img), change_id: get_id() }, DEFAULT_BASE, 1.5, 0.0);
    let result = OpImageAdjustmentNegadoctor::run(&mut ins).await;
    assert!(result.is_ok(), "grayscale negadoctor failed: {:?}", result.err());
    let Value::Image { data, .. } = &result.unwrap().responses[0].value else { panic!() };
    assert!(data.get_pixel(0, 0)[0] < 0.01, "pixel at the base luma should map near black: {:?}", data.get_pixel(0, 0));
}
