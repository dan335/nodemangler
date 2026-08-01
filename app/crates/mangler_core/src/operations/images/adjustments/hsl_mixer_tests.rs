//! Tests for the HSL mixer (per-hue-band HSL) adjustment operation.

use super::*;

use crate::float_image::FloatImage;
use crate::get_id;
use crate::input::Input;
use crate::value::Value;
use std::sync::Arc;

/// Builds the 25 inputs (image + 24 band sliders, all defaulting to 0) with
/// one band's hue/saturation/lightness overridden.
fn inputs_with_band(image: Value, band_index: usize, hue: f32, sat: f32, light: f32) -> Vec<Input> {
    let mut ins = vec![Input::new("image".to_string(), image, None, None)];
    for i in 0..8 {
        let (h, s, l) = if i == band_index { (hue, sat, light) } else { (0.0, 0.0, 0.0) };
        ins.push(Input::new(format!("{} hue", BAND_NAMES[i]), Value::Decimal(h as f32), None, None));
        ins.push(Input::new(format!("{} saturation", BAND_NAMES[i]), Value::Decimal(s as f32), None, None));
        ins.push(Input::new(format!("{} lightness", BAND_NAMES[i]), Value::Decimal(l as f32), None, None));
    }
    ins
}

fn all_zero_inputs(image: Value) -> Vec<Input> {
    inputs_with_band(image, 0, 0.0, 0.0, 0.0)
}

fn solid_image(w: u32, h: u32, rgba: [f32; 4]) -> Arc<FloatImage> {
    Arc::new(FloatImage::from_pixel(w, h, 4, &rgba))
}

#[tokio::test]
async fn settings_and_ports() {
    assert_eq!(OpImageAdjustmentHslMixer::settings().name, "hsl mixer");
    assert_eq!(OpImageAdjustmentHslMixer::create_inputs().len(), 25);
    assert_eq!(OpImageAdjustmentHslMixer::create_outputs().len(), 1);
}

#[tokio::test]
async fn all_hidden_in_graph() {
    let inputs = OpImageAdjustmentHslMixer::create_inputs();
    assert!(!inputs[0].hide_in_graph, "image input should stay visible");
    for input in &inputs[1..] {
        assert!(input.hide_in_graph, "band input '{}' should be hidden in the graph", input.name);
    }
}

#[tokio::test]
async fn all_zero_is_passthrough() {
    let src = solid_image(4, 4, [1.0, 0.0, 0.0, 1.0]);
    let mut ins = all_zero_inputs(Value::Image { data: src.clone(), change_id: get_id() });
    let result = OpImageAdjustmentHslMixer::run(&mut ins).await.unwrap();
    let Value::Image { data, .. } = &result.responses[0].value else { panic!() };
    assert!(Arc::ptr_eq(&src, data), "all-zero bands should pass the original Arc through");
}

#[tokio::test]
async fn red_band_hue_shift_moves_red_leaves_blue() {
    // Two-pixel image: pure red at (0,0), pure blue at (1,0).
    let mut img = FloatImage::new(2, 1, 4);
    img.put_pixel(0, 0, &[1.0, 0.0, 0.0, 1.0]);
    img.put_pixel(1, 0, &[0.0, 0.0, 1.0, 1.0]);
    let src = Arc::new(img);

    // band_index 0 = red, shift hue +30 degrees (toward orange).
    let mut ins = inputs_with_band(Value::Image { data: src.clone(), change_id: get_id() }, 0, 30.0, 0.0, 0.0);
    let result = OpImageAdjustmentHslMixer::run(&mut ins).await.unwrap();
    let Value::Image { data, .. } = &result.responses[0].value else { panic!() };

    let red_out = data.get_pixel(0, 0);
    // Red shifted toward orange should gain green while staying red-heavy.
    assert!(red_out[1] > 0.05, "red band hue shift should move the red pixel's hue (green channel rose): {red_out:?}");
    assert!(red_out[0] > 0.5, "red pixel should stay red-dominant after a +30 degree shift: {red_out:?}");

    let blue_out = data.get_pixel(1, 0);
    assert!((blue_out[0] - 0.0).abs() < 1e-4 && (blue_out[2] - 1.0).abs() < 1e-4, "blue pixel should be untouched by the red band: {blue_out:?}");
}

#[tokio::test]
async fn red_band_saturation_desaturates_only_red() {
    let mut img = FloatImage::new(2, 1, 4);
    img.put_pixel(0, 0, &[1.0, 0.0, 0.0, 1.0]);
    img.put_pixel(1, 0, &[0.0, 0.0, 1.0, 1.0]);
    let src = Arc::new(img);

    // band_index 0 = red, full desaturation.
    let mut ins = inputs_with_band(Value::Image { data: src.clone(), change_id: get_id() }, 0, 0.0, -1.0, 0.0);
    let result = OpImageAdjustmentHslMixer::run(&mut ins).await.unwrap();
    let Value::Image { data, .. } = &result.responses[0].value else { panic!() };

    let red_out = data.get_pixel(0, 0);
    assert!((red_out[0] - red_out[1]).abs() < 1e-3 && (red_out[1] - red_out[2]).abs() < 1e-3, "fully desaturated red pixel should be gray: {red_out:?}");

    let blue_out = data.get_pixel(1, 0);
    assert!((blue_out[0] - 0.0).abs() < 1e-4 && (blue_out[2] - 1.0).abs() < 1e-4, "blue pixel should keep full saturation: {blue_out:?}");
}

#[tokio::test]
async fn grayscale_passthrough() {
    let img = FloatImage::from_pixel(4, 4, 1, &[0.5]);
    let src = Arc::new(img);
    let mut ins = inputs_with_band(Value::Image { data: src.clone(), change_id: get_id() }, 3, 30.0, 1.0, 0.5);
    let result = OpImageAdjustmentHslMixer::run(&mut ins).await.unwrap();
    let Value::Image { data, .. } = &result.responses[0].value else { panic!() };
    assert!(Arc::ptr_eq(&src, data), "grayscale image should pass through unchanged");
}

#[tokio::test]
async fn alpha_preserved_on_rgba() {
    let mut img = FloatImage::new(2, 1, 4);
    img.put_pixel(0, 0, &[1.0, 0.0, 0.0, 0.42]);
    img.put_pixel(1, 0, &[0.0, 1.0, 0.0, 0.42]);
    let src = Arc::new(img);
    let mut ins = inputs_with_band(Value::Image { data: src.clone(), change_id: get_id() }, 0, 30.0, 0.5, 0.2);
    let result = OpImageAdjustmentHslMixer::run(&mut ins).await.unwrap();
    let Value::Image { data, .. } = &result.responses[0].value else { panic!() };
    for x in 0..2 {
        assert!((data.get_pixel(x, 0)[3] - 0.42).abs() < 1e-6, "alpha changed at pixel {x}");
    }
}
