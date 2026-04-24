//! Tests for the gradient dynamic operation.

use super::*;

use crate::float_image::FloatImage;
use crate::get_id;
use crate::input::Input;
use crate::value::Value;
use std::sync::Arc;

/// 16-wide red-to-blue gradient.
fn red_to_blue() -> Arc<FloatImage> {
    let w = 16u32;
    let mut img = FloatImage::new(w, 1, 4);
    for x in 0..w {
        let t = x as f32 / (w - 1) as f32;
        img.put_pixel(x, 0, &[1.0 - t, 0.0, t, 1.0]);
    }
    Arc::new(img)
}

#[tokio::test]
async fn zero_strength_matches_gradient_map() {
    // Strength 0 → same output as the classic gradient_map.
    let src = Arc::new(FloatImage::from_pixel(4, 4, 1, &[0.0])); // luminance 0
    let field = Arc::new(FloatImage::from_pixel(4, 4, 3, &[1.0, 0.0, 1.0])); // max x
    let mut inputs = vec![
        Input::new("image".into(), Value::Image { data: src, change_id: get_id() }, None, None),
        Input::new("gradient".into(), Value::Image { data: red_to_blue(), change_id: get_id() }, None, None),
        Input::new("vector field".into(), Value::Image { data: field, change_id: get_id() }, None, None),
        Input::new("strength".into(), Value::Decimal(0.0), None, None),
        Input::new("angle".into(), Value::Decimal(0.0), None, None),
    ];
    let r = OpImageAdjustmentGradientDynamic::run(&mut inputs).await.unwrap();
    let Value::Image { data, .. } = &r.responses[0].value else { panic!() };
    let px = data.get_pixel(0, 0);
    assert!(px[0] > 0.9);  // red end
    assert!(px[2] < 0.1);  // not blue
}

#[tokio::test]
async fn field_shifts_sample_position() {
    // Luminance 0 with strong +x field and strength 1 → sampled forward toward blue.
    let src = Arc::new(FloatImage::from_pixel(4, 4, 1, &[0.0]));
    let field = Arc::new(FloatImage::from_pixel(4, 4, 3, &[1.0, 0.5, 1.0])); // x = 1, y = 0
    let mut inputs = vec![
        Input::new("image".into(), Value::Image { data: src, change_id: get_id() }, None, None),
        Input::new("gradient".into(), Value::Image { data: red_to_blue(), change_id: get_id() }, None, None),
        Input::new("vector field".into(), Value::Image { data: field, change_id: get_id() }, None, None),
        Input::new("strength".into(), Value::Decimal(1.0), None, None),
        Input::new("angle".into(), Value::Decimal(0.0), None, None),
    ];
    let r = OpImageAdjustmentGradientDynamic::run(&mut inputs).await.unwrap();
    let Value::Image { data, .. } = &r.responses[0].value else { panic!() };
    let px = data.get_pixel(0, 0);
    // +1 field × 1 strength = shift by +1 → wraps to the far end of the gradient (blue).
    assert!(px[2] > 0.8, "expected blue shift, got B={}", px[2]);
}
