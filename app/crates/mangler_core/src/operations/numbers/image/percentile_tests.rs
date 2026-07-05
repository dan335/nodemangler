use super::*;
use crate::float_image::FloatImage;
use crate::get_id;
use crate::input::Input;
use crate::value::Value;
use std::sync::Arc;

fn inputs_with(img: FloatImage, percentile: f32) -> Vec<Input> {
    vec![
        Input::new("image".to_string(), Value::Image { data: Arc::new(img), change_id: get_id() }, None, None),
        Input::new("percentile".to_string(), Value::Decimal(percentile), None, None),
    ]
}

fn dec(v: &Value) -> f32 {
    match v { Value::Decimal(d) => *d, other => panic!("expected Decimal, got {:?}", other) }
}

/// A 0.0, 0.25, 0.5, 0.75, 1.0 ramp for percentile sampling.
fn ramp() -> FloatImage {
    let mut img = FloatImage::new(5, 1, 1);
    img.put_pixel(0, 0, &[0.0]);
    img.put_pixel(1, 0, &[0.25]);
    img.put_pixel(2, 0, &[0.5]);
    img.put_pixel(3, 0, &[0.75]);
    img.put_pixel(4, 0, &[1.0]);
    img
}

#[tokio::test]
async fn test_percentile_settings() {
    let s = OpNumberImagePercentile::settings();
    assert_eq!(s.name, "percentile");
    assert_eq!(OpNumberImagePercentile::create_inputs().len(), 2);
    assert_eq!(OpNumberImagePercentile::create_outputs().len(), 1);
}

#[tokio::test]
async fn test_percentile_extremes() {
    // 0th percentile → darkest, 100th → brightest
    let mut inputs = inputs_with(ramp(), 0.0);
    let r = OpNumberImagePercentile::run(&mut inputs).await.unwrap();
    assert!((dec(&r.responses[0].value) - 0.0).abs() < 1e-6);

    let mut inputs = inputs_with(ramp(), 100.0);
    let r = OpNumberImagePercentile::run(&mut inputs).await.unwrap();
    assert!((dec(&r.responses[0].value) - 1.0).abs() < 1e-6);
}

#[tokio::test]
async fn test_percentile_median_is_middle() {
    // n=5, p=50 → idx = round(0.5 * 4) = 2 → 0.5
    let mut inputs = inputs_with(ramp(), 50.0);
    let r = OpNumberImagePercentile::run(&mut inputs).await.unwrap();
    assert!((dec(&r.responses[0].value) - 0.5).abs() < 1e-6);
}

#[tokio::test]
async fn test_percentile_clamps_out_of_range() {
    // percentile above 100 clamps to the brightest pixel
    let mut inputs = inputs_with(ramp(), 250.0);
    let r = OpNumberImagePercentile::run(&mut inputs).await.unwrap();
    assert!((dec(&r.responses[0].value) - 1.0).abs() < 1e-6);
}
