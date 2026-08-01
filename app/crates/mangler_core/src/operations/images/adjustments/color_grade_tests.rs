//! Tests for the color grade (three-way shadows/midtones/highlights) operation.

use super::*;

use crate::float_image::FloatImage;
use crate::get_id;
use crate::input::Input;
use crate::value::Value;
use std::sync::Arc;

#[allow(clippy::too_many_arguments)]
fn inputs(
    image: Value,
    sh: (f32, f32, f32),
    mid: (f32, f32, f32),
    hi: (f32, f32, f32),
    blending: f32,
    balance: f32,
) -> Vec<Input> {
    let mut ins = vec![Input::new("image".to_string(), image, None, None)];
    for (h, s, l) in [sh, mid, hi] {
        ins.push(Input::new("hue".to_string(), Value::Decimal(h as f32), None, None));
        ins.push(Input::new("saturation".to_string(), Value::Decimal(s as f32), None, None));
        ins.push(Input::new("luminance".to_string(), Value::Decimal(l as f32), None, None));
    }
    ins.push(Input::new("blending".to_string(), Value::Decimal(blending as f32), None, None));
    ins.push(Input::new("balance".to_string(), Value::Decimal(balance as f32), None, None));
    ins
}

fn solid_image(w: u32, h: u32, rgba: [f32; 4]) -> Arc<FloatImage> {
    Arc::new(FloatImage::from_pixel(w, h, 4, &rgba))
}

#[tokio::test]
async fn settings_and_ports() {
    assert_eq!(OpImageAdjustmentColorGrade::settings().name, "color grade");
    assert_eq!(OpImageAdjustmentColorGrade::create_inputs().len(), 12);
    assert_eq!(OpImageAdjustmentColorGrade::create_outputs().len(), 1);
}

#[tokio::test]
async fn all_neutral_is_passthrough() {
    let src = solid_image(4, 4, [0.3, 0.3, 0.3, 1.0]);
    // Hues, blending, and balance are non-default but sats/lums are all 0.
    let mut ins = inputs(
        Value::Image { data: src.clone(), change_id: get_id() },
        (123.0, 0.0, 0.0),
        (45.0, 0.0, 0.0),
        (200.0, 0.0, 0.0),
        0.9,
        0.5,
    );
    let result = OpImageAdjustmentColorGrade::run(&mut ins).await.unwrap();
    let Value::Image { data, .. } = &result.responses[0].value else { panic!() };
    assert!(Arc::ptr_eq(&src, data), "zero sat/lum should pass the original Arc through");
}

#[tokio::test]
async fn shadow_tint_colors_dark_gray_leaves_white_alone() {
    let mut img = FloatImage::new(2, 1, 4);
    img.put_pixel(0, 0, &[0.1, 0.1, 0.1, 1.0]); // dark gray
    img.put_pixel(1, 0, &[1.0, 1.0, 1.0, 1.0]); // white
    let src = Arc::new(img);

    // Shadows tinted blue-ish (hue 200), full saturation, no luminance shift.
    let mut ins = inputs(
        Value::Image { data: src.clone(), change_id: get_id() },
        (200.0, 1.0, 0.0),
        (0.0, 0.0, 0.0),
        (0.0, 0.0, 0.0),
        0.5,
        0.0,
    );
    let result = OpImageAdjustmentColorGrade::run(&mut ins).await.unwrap();
    let Value::Image { data, .. } = &result.responses[0].value else { panic!() };

    let dark_out = data.get_pixel(0, 0);
    assert!(dark_out[2] > dark_out[0], "dark gray pixel should be tinted toward blue: {dark_out:?}");
    assert!((dark_out[0] - dark_out[1]).abs() > 1e-4 || (dark_out[1] - dark_out[2]).abs() > 1e-4, "dark pixel should no longer be neutral gray: {dark_out:?}");

    let white_out = data.get_pixel(1, 0);
    assert!((white_out[0] - 1.0).abs() < 1e-4 && (white_out[1] - 1.0).abs() < 1e-4 && (white_out[2] - 1.0).abs() < 1e-4, "white pixel should be left nearly alone: {white_out:?}");
}

#[tokio::test]
async fn balance_shifts_which_pixels_are_tinted() {
    // A mid-gray pixel: with the shadow/highlight crossover pushed way up
    // (balance = 1) it falls fully in the shadow weight; pushed way down
    // (balance = -1) it falls fully outside the shadow weight.
    let pixel = [0.35, 0.35, 0.35, 1.0];

    let mut ins_hi_balance = inputs(
        Value::Image { data: solid_image(2, 2, pixel), change_id: get_id() },
        (0.0, 1.0, 0.0),
        (0.0, 0.0, 0.0),
        (0.0, 0.0, 0.0),
        0.0,
        1.0,
    );
    let result_hi = OpImageAdjustmentColorGrade::run(&mut ins_hi_balance).await.unwrap();
    let Value::Image { data: data_hi, .. } = &result_hi.responses[0].value else { panic!() };
    let out_hi = data_hi.get_pixel(0, 0);

    let mut ins_lo_balance = inputs(
        Value::Image { data: solid_image(2, 2, pixel), change_id: get_id() },
        (0.0, 1.0, 0.0),
        (0.0, 0.0, 0.0),
        (0.0, 0.0, 0.0),
        0.0,
        -1.0,
    );
    let result_lo = OpImageAdjustmentColorGrade::run(&mut ins_lo_balance).await.unwrap();
    let Value::Image { data: data_lo, .. } = &result_lo.responses[0].value else { panic!() };
    let out_lo = data_lo.get_pixel(0, 0);

    // balance=1 should pull the pixel into the (red-tinted) shadow band...
    assert!(out_hi[0] > out_hi[1] + 0.01, "balance=1 should tint this pixel toward red: {out_hi:?}");
    // ...while balance=-1 should leave it outside the shadow band, untinted.
    assert!((out_lo[0] - out_lo[1]).abs() < 1e-4 && (out_lo[1] - out_lo[2]).abs() < 1e-4, "balance=-1 should leave this pixel neutral: {out_lo:?}");
}

#[tokio::test]
async fn grayscale_applies_luminance_only() {
    let img = FloatImage::from_pixel(4, 4, 1, &[0.1]);
    let src = Arc::new(img);
    let mut ins = inputs(
        Value::Image { data: src.clone(), change_id: get_id() },
        (123.0, 0.0, 1.0), // hue ignored (no chroma); positive luminance should brighten
        (0.0, 0.0, 0.0),
        (0.0, 0.0, 0.0),
        0.5,
        0.0,
    );
    let result = OpImageAdjustmentColorGrade::run(&mut ins).await.unwrap();
    let Value::Image { data, .. } = &result.responses[0].value else { panic!() };
    assert!(data.get_pixel(0, 0)[0] > 0.1, "positive shadow luminance should brighten a dark grayscale pixel");
}

#[tokio::test]
async fn alpha_preserved_on_rgba() {
    let src = solid_image(4, 4, [0.2, 0.2, 0.2, 0.63]);
    let mut ins = inputs(
        Value::Image { data: src.clone(), change_id: get_id() },
        (10.0, 1.0, 0.5),
        (20.0, 0.5, -0.3),
        (30.0, 0.5, 0.2),
        0.3,
        0.1,
    );
    let result = OpImageAdjustmentColorGrade::run(&mut ins).await.unwrap();
    let Value::Image { data, .. } = &result.responses[0].value else { panic!() };
    for y in 0..4 {
        for x in 0..4 {
            assert!((data.get_pixel(x, y)[3] - 0.63).abs() < 1e-6, "alpha changed at ({x},{y})");
        }
    }
}
