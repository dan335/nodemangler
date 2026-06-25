//! Tests for the channel mixer.

use super::*;
use crate::float_image::FloatImage;
use crate::get_id;
use crate::input::Input;
use crate::value::Value;
use std::sync::Arc;

fn identity_inputs(img: Arc<FloatImage>) -> Vec<Input> {
    vec![
        Input::new("image".into(), Value::Image { data: img, change_id: get_id() }, None, None),
        Input::new("r from r".into(), Value::Decimal(1.0), None, None),
        Input::new("r from g".into(), Value::Decimal(0.0), None, None),
        Input::new("r from b".into(), Value::Decimal(0.0), None, None),
        Input::new("r bias".into(), Value::Decimal(0.0), None, None),
        Input::new("g from r".into(), Value::Decimal(0.0), None, None),
        Input::new("g from g".into(), Value::Decimal(1.0), None, None),
        Input::new("g from b".into(), Value::Decimal(0.0), None, None),
        Input::new("g bias".into(), Value::Decimal(0.0), None, None),
        Input::new("b from r".into(), Value::Decimal(0.0), None, None),
        Input::new("b from g".into(), Value::Decimal(0.0), None, None),
        Input::new("b from b".into(), Value::Decimal(1.0), None, None),
        Input::new("b bias".into(), Value::Decimal(0.0), None, None),
    ]
}

#[tokio::test]
async fn identity_is_passthrough() {
    let img = Arc::new(FloatImage::from_pixel(2, 2, 4, &[0.1, 0.2, 0.3, 0.4]));
    let mut inputs = identity_inputs(img);
    let r = OpImageChannelMixer::run(&mut inputs).await.unwrap();
    let Value::Image { data, .. } = &r.responses[0].value else { panic!() };
    let px = data.get_pixel(0, 0);
    assert!((px[0] - 0.1).abs() < 1e-5);
    assert!((px[1] - 0.2).abs() < 1e-5);
    assert!((px[2] - 0.3).abs() < 1e-5);
    assert!((px[3] - 0.4).abs() < 1e-5);
}

#[tokio::test]
async fn swap_r_and_b() {
    let img = Arc::new(FloatImage::from_pixel(1, 1, 4, &[0.9, 0.5, 0.1, 1.0]));
    let mut inputs = identity_inputs(img);
    // R = B; B = R.
    inputs[1].value = Value::Decimal(0.0);  // r from r
    inputs[3].value = Value::Decimal(1.0);  // r from b
    inputs[9].value = Value::Decimal(1.0);  // b from r
    inputs[11].value = Value::Decimal(0.0); // b from b
    let r = OpImageChannelMixer::run(&mut inputs).await.unwrap();
    let Value::Image { data, .. } = &r.responses[0].value else { panic!() };
    let px = data.get_pixel(0, 0);
    assert!((px[0] - 0.1).abs() < 1e-5);
    assert!((px[2] - 0.9).abs() < 1e-5);
}

#[tokio::test]
async fn output_clamped_to_unit_interval() {
    let img = Arc::new(FloatImage::from_pixel(1, 1, 4, &[1.0, 1.0, 1.0, 1.0]));
    let mut inputs = identity_inputs(img);
    inputs[4].value = Value::Decimal(5.0); // huge r bias
    let r = OpImageChannelMixer::run(&mut inputs).await.unwrap();
    let Value::Image { data, .. } = &r.responses[0].value else { panic!() };
    assert!((data.get_pixel(0, 0)[0] - 1.0).abs() < 1e-5);
}

#[tokio::test]
async fn grayscale_luminance_via_r_row() {
    // Solid red input; set R output to Rec.709 luminance weights on (R,G,B).
    let img = Arc::new(FloatImage::from_pixel(1, 1, 4, &[1.0, 0.0, 0.0, 1.0]));
    let mut inputs = identity_inputs(img);
    inputs[1].value = Value::Decimal(0.2126);
    inputs[2].value = Value::Decimal(0.7152);
    inputs[3].value = Value::Decimal(0.0722);
    let r = OpImageChannelMixer::run(&mut inputs).await.unwrap();
    let Value::Image { data, .. } = &r.responses[0].value else { panic!() };
    assert!((data.get_pixel(0, 0)[0] - 0.2126).abs() < 1e-4);
}
