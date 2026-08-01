//! Tests for the defringe (hue-targeted edge desaturation) operation.

use super::*;

use crate::float_image::FloatImage;
use crate::get_id;
use crate::input::Input;
use crate::value::Value;
use std::sync::Arc;

fn inputs(image: Value, amount: f32, threshold: f32, purple: bool, green: bool) -> Vec<Input> {
    vec![
        Input::new("image".to_string(), image, None, None),
        Input::new("amount".to_string(), Value::Decimal(amount as f32), None, None),
        Input::new("edge threshold".to_string(), Value::Decimal(threshold as f32), None, None),
        Input::new("purple".to_string(), Value::Bool(purple), None, None),
        Input::new("green".to_string(), Value::Bool(green), None, None),
    ]
}

/// Builds a 6x1 image: columns 0-2 black, columns 3-5 a purple colour (hue
/// ~295 degrees). Column 3 sits right at the black/purple edge (strong
/// Sobel gradient); column 4 is flanked by purple on both sides (flat, no
/// gradient).
fn edge_and_flat_image() -> (Arc<FloatImage>, (f32, f32, f32)) {
    let purple = hsl_to_rgb(295.0, 1.0, 0.5);
    let mut img = FloatImage::new(6, 1, 4);
    for x in 0..3 {
        img.put_pixel(x, 0, &[0.0, 0.0, 0.0, 1.0]);
    }
    for x in 3..6 {
        img.put_pixel(x, 0, &[purple.0, purple.1, purple.2, 1.0]);
    }
    (Arc::new(img), purple)
}

#[tokio::test]
async fn settings_and_ports() {
    assert_eq!(OpImageAdjustmentDefringe::settings().name, "defringe");
    assert_eq!(OpImageAdjustmentDefringe::create_inputs().len(), 5);
    assert_eq!(OpImageAdjustmentDefringe::create_outputs().len(), 1);
}

#[tokio::test]
async fn zero_amount_is_passthrough() {
    let (src, _) = edge_and_flat_image();
    let mut ins = inputs(Value::Image { data: src.clone(), change_id: get_id() }, 0.0, 0.1, true, true);
    let result = OpImageAdjustmentDefringe::run(&mut ins).await.unwrap();
    let Value::Image { data, .. } = &result.responses[0].value else { panic!() };
    assert!(Arc::ptr_eq(&src, data), "amount=0 should pass the original Arc through");
}

#[tokio::test]
async fn both_bands_off_is_passthrough() {
    let (src, _) = edge_and_flat_image();
    let mut ins = inputs(Value::Image { data: src.clone(), change_id: get_id() }, 1.0, 0.1, false, false);
    let result = OpImageAdjustmentDefringe::run(&mut ins).await.unwrap();
    let Value::Image { data, .. } = &result.responses[0].value else { panic!() };
    assert!(Arc::ptr_eq(&src, data), "both bands disabled should pass the original Arc through");
}

#[tokio::test]
async fn purple_edge_pixel_desaturated_flat_pixel_untouched() {
    let (src, purple) = edge_and_flat_image();
    let mut ins = inputs(Value::Image { data: src.clone(), change_id: get_id() }, 1.0, 0.1, true, true);
    let result = OpImageAdjustmentDefringe::run(&mut ins).await.unwrap();
    let Value::Image { data, .. } = &result.responses[0].value else { panic!() };

    let (_, edge_s, _) = rgb_to_hsl(data.get_pixel(3, 0)[0], data.get_pixel(3, 0)[1], data.get_pixel(3, 0)[2]);
    let (_, orig_s, _) = rgb_to_hsl(purple.0, purple.1, purple.2);
    assert!(edge_s < orig_s - 0.1, "purple pixel at a strong edge should be desaturated: {edge_s} vs {orig_s}");

    let flat = data.get_pixel(4, 0);
    assert!((flat[0] - purple.0).abs() < 1e-4 && (flat[1] - purple.1).abs() < 1e-4 && (flat[2] - purple.2).abs() < 1e-4, "flat-area purple pixel should be untouched: {flat:?} vs {purple:?}");
}

#[tokio::test]
async fn disabled_band_leaves_matching_pixel_untouched() {
    // Purple band disabled, green enabled: the purple edge pixel doesn't
    // match the (enabled) green band, so it should stay untouched even
    // though it sits on a strong edge and amount > 0.
    let (src, purple) = edge_and_flat_image();
    let mut ins = inputs(Value::Image { data: src.clone(), change_id: get_id() }, 1.0, 0.1, false, true);
    let result = OpImageAdjustmentDefringe::run(&mut ins).await.unwrap();
    let Value::Image { data, .. } = &result.responses[0].value else { panic!() };
    let edge = data.get_pixel(3, 0);
    assert!((edge[0] - purple.0).abs() < 1e-4 && (edge[1] - purple.1).abs() < 1e-4 && (edge[2] - purple.2).abs() < 1e-4, "purple pixel should be untouched when only the green band is enabled: {edge:?}");
}

#[tokio::test]
async fn grayscale_passthrough() {
    let img = FloatImage::from_pixel(4, 4, 1, &[0.5]);
    let src = Arc::new(img);
    let mut ins = inputs(Value::Image { data: src.clone(), change_id: get_id() }, 1.0, 0.1, true, true);
    let result = OpImageAdjustmentDefringe::run(&mut ins).await.unwrap();
    let Value::Image { data, .. } = &result.responses[0].value else { panic!() };
    assert!(Arc::ptr_eq(&src, data), "grayscale image should pass through unchanged");
}

#[tokio::test]
async fn alpha_preserved_on_rgba() {
    let purple = hsl_to_rgb(295.0, 1.0, 0.5);
    let mut img = FloatImage::new(3, 1, 4);
    img.put_pixel(0, 0, &[0.0, 0.0, 0.0, 0.55]);
    img.put_pixel(1, 0, &[purple.0, purple.1, purple.2, 0.55]);
    img.put_pixel(2, 0, &[purple.0, purple.1, purple.2, 0.55]);
    let src = Arc::new(img);
    let mut ins = inputs(Value::Image { data: src.clone(), change_id: get_id() }, 1.0, 0.1, true, true);
    let result = OpImageAdjustmentDefringe::run(&mut ins).await.unwrap();
    let Value::Image { data, .. } = &result.responses[0].value else { panic!() };
    for x in 0..3 {
        assert!((data.get_pixel(x, 0)[3] - 0.55).abs() < 1e-6, "alpha changed at pixel {x}");
    }
}
