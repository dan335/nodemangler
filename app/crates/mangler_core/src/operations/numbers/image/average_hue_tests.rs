use super::*;
use crate::float_image::FloatImage;
use crate::get_id;
use crate::input::Input;
use crate::value::Value;
use std::sync::Arc;

fn image_input(img: FloatImage) -> Vec<Input> {
    vec![Input::new("image".to_string(), Value::Image { data: Arc::new(img), change_id: get_id() }, None, None)]
}

fn dec(v: &Value) -> f32 {
    match v { Value::Decimal(d) => *d, other => panic!("expected Decimal, got {:?}", other) }
}

#[tokio::test]
async fn test_average_hue_settings() {
    let s = OpNumberImageAverageHue::settings();
    assert_eq!(s.name, "average hue");
    assert_eq!(OpNumberImageAverageHue::create_outputs().len(), 3);
}

#[tokio::test]
async fn test_average_hue_pure_red() {
    // pure red -> hue ~0 degrees, saturation 1, value 1
    let img = FloatImage::from_pixel(4, 4, 3, &[1.0, 0.0, 0.0]);
    let mut inputs = image_input(img);
    let r = OpNumberImageAverageHue::run(&mut inputs).await.unwrap();
    let hue = dec(&r.responses[0].value);
    assert!(hue < 1.0 || hue > 359.0, "expected hue near 0, got {}", hue);
    assert!((dec(&r.responses[1].value) - 1.0).abs() < 1e-4);
    assert!((dec(&r.responses[2].value) - 1.0).abs() < 1e-4);
}

#[tokio::test]
async fn test_average_hue_green() {
    // pure green -> hue ~120 degrees
    let img = FloatImage::from_pixel(4, 4, 3, &[0.0, 1.0, 0.0]);
    let mut inputs = image_input(img);
    let r = OpNumberImageAverageHue::run(&mut inputs).await.unwrap();
    assert!((dec(&r.responses[0].value) - 120.0).abs() < 1.0);
}

#[tokio::test]
async fn test_average_hue_gray_has_no_hue() {
    // gray -> zero saturation everywhere -> hue direction undefined -> 0
    let img = FloatImage::from_pixel(4, 4, 3, &[0.5, 0.5, 0.5]);
    let mut inputs = image_input(img);
    let r = OpNumberImageAverageHue::run(&mut inputs).await.unwrap();
    assert!((dec(&r.responses[0].value)).abs() < 1e-6);
    assert!((dec(&r.responses[1].value)).abs() < 1e-4);
    assert!((dec(&r.responses[2].value) - 0.5).abs() < 1e-4);
}
