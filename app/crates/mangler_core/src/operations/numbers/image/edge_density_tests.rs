use super::*;
use crate::float_image::FloatImage;
use crate::get_id;
use crate::input::Input;
use crate::value::Value;
use std::sync::Arc;

fn inputs(img: FloatImage, threshold: f32) -> Vec<Input> {
    vec![
        Input::new("image".to_string(), Value::Image { data: Arc::new(img), change_id: get_id() }, None, None),
        Input::new("threshold".to_string(), Value::Decimal(threshold), None, None),
    ]
}

fn dec(v: &Value) -> f32 {
    match v { Value::Decimal(d) => *d, other => panic!("expected Decimal, got {:?}", other) }
}

#[tokio::test]
async fn test_edge_density_settings() {
    let s = OpNumberImageEdgeDensity::settings();
    assert_eq!(s.name, "edge density");
    assert_eq!(OpNumberImageEdgeDensity::create_inputs().len(), 2);
    assert_eq!(OpNumberImageEdgeDensity::create_outputs().len(), 1);
}

#[tokio::test]
async fn test_edge_density_flat_is_zero() {
    let img = FloatImage::from_pixel(8, 8, 1, &[0.5]);
    let mut inp = inputs(img, 0.2);
    let r = OpNumberImageEdgeDensity::run(&mut inp).await.unwrap();
    assert!(dec(&r.responses[0].value).abs() < 1e-6);
}

#[tokio::test]
async fn test_edge_density_vertical_edge() {
    // A half-black / half-white split has a real vertical edge; the columns
    // straddling the boundary fire on Sobel. (A per-pixel checkerboard would
    // NOT: its symmetric neighbourhood makes the Sobel response cancel to zero.)
    let mut img = FloatImage::new(8, 8, 1);
    for y in 0..8 {
        for x in 0..8 {
            let v = if x < 4 { 0.0 } else { 1.0 };
            img.put_pixel(x, y, &[v]);
        }
    }
    let mut inp = inputs(img, 0.2);
    let r = OpNumberImageEdgeDensity::run(&mut inp).await.unwrap();
    // Two interior columns straddle the edge -> ~1/3 of interior pixels.
    let density = dec(&r.responses[0].value);
    assert!(density > 0.2, "expected a clear edge, got density {density}");
}

#[tokio::test]
async fn test_edge_density_tiny_image_zero() {
    let img = FloatImage::from_pixel(2, 2, 1, &[0.5]);
    let mut inp = inputs(img, 0.2);
    let r = OpNumberImageEdgeDensity::run(&mut inp).await.unwrap();
    assert!(dec(&r.responses[0].value).abs() < 1e-9);
}
