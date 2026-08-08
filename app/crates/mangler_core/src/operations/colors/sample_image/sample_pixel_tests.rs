use super::*;
use crate::float_image::FloatImage;
use crate::get_id;
use crate::input::Input;
use crate::value::Value;
use std::sync::Arc;

fn inputs_at(img: FloatImage, x: f32, y: f32, diameter: i32) -> Vec<Input> {
    vec![
        Input::new("image".to_string(), Value::Image { data: Arc::new(img), change_id: get_id() }, None, None),
        Input::new("x".to_string(), Value::Decimal(x), None, None),
        Input::new("y".to_string(), Value::Decimal(y), None, None),
        Input::new("diameter".to_string(), Value::Integer(diameter), None, None),
    ]
}

fn dec(v: &Value) -> f32 {
    match v { Value::Decimal(d) => *d, other => panic!("expected Decimal, got {:?}", other) }
}

#[tokio::test]
async fn test_sample_pixel_settings() {
    let s = OpColorSampleSamplePixel::settings();
    assert_eq!(s.name, "sample pixel");
    assert_eq!(OpColorSampleSamplePixel::create_inputs().len(), 4);
    assert_eq!(OpColorSampleSamplePixel::create_outputs().len(), 5);
    // Diameter is last; gizmo still points at x=1, y=2.
    assert_eq!(OpColorSampleSamplePixel::create_inputs()[3].name, "diameter");
}

#[tokio::test]
async fn test_sample_pixel_uniform_rgba() {
    let img = FloatImage::from_pixel(4, 4, 4, &[0.2, 0.4, 0.6, 0.8]);
    let mut inputs = inputs_at(img, 0.5, 0.5, 1);
    let r = OpColorSampleSamplePixel::run(&mut inputs).await.unwrap();
    assert!((dec(&r.responses[1].value) - 0.2).abs() < 1e-5);
    assert!((dec(&r.responses[2].value) - 0.4).abs() < 1e-5);
    assert!((dec(&r.responses[3].value) - 0.6).abs() < 1e-5);
    assert!((dec(&r.responses[4].value) - 0.8).abs() < 1e-5);
    match &r.responses[0].value {
        Value::Color(c) => {
            assert!((c.r - 0.2).abs() < 1e-5);
            assert!((c.a - 0.8).abs() < 1e-5);
        }
        other => panic!("expected Color, got {:?}", other),
    }
}

#[tokio::test]
async fn test_sample_pixel_grayscale_alpha_defaults_one() {
    let img = FloatImage::from_pixel(2, 2, 1, &[0.5]);
    let mut inputs = inputs_at(img, 0.0, 0.0, 1);
    let r = OpColorSampleSamplePixel::run(&mut inputs).await.unwrap();
    // grayscale replicated across rgb, alpha defaults to 1
    assert!((dec(&r.responses[1].value) - 0.5).abs() < 1e-5);
    assert!((dec(&r.responses[2].value) - 0.5).abs() < 1e-5);
    assert!((dec(&r.responses[3].value) - 0.5).abs() < 1e-5);
    assert!((dec(&r.responses[4].value) - 1.0).abs() < 1e-5);
}

#[tokio::test]
async fn test_sample_pixel_corners_of_gradient() {
    // Horizontal gradient: left column 0.0, right column 1.0 (grayscale).
    let mut img = FloatImage::new(2, 1, 1);
    img.put_pixel(0, 0, &[0.0]);
    img.put_pixel(1, 0, &[1.0]);
    let mut left = inputs_at(img.clone(), 0.0, 0.5, 1);
    let rl = OpColorSampleSamplePixel::run(&mut left).await.unwrap();
    assert!(dec(&rl.responses[1].value).abs() < 1e-5);
    let mut right = inputs_at(img, 1.0, 0.5, 1);
    let rr = OpColorSampleSamplePixel::run(&mut right).await.unwrap();
    assert!((dec(&rr.responses[1].value) - 1.0).abs() < 1e-5);
}

#[tokio::test]
async fn test_sample_pixel_diameter_averages_neighbours() {
    // 3×3: centre white, everything else black. A diameter-1 sample at the
    // centre is white; diameter 3 covers the whole 3×3 disk (radius 1.5 includes
    // all 8 neighbours) and averages to 1/9 white.
    let mut img = FloatImage::new(3, 3, 1);
    for y in 0..3 {
        for x in 0..3 {
            let v = if x == 1 && y == 1 { 1.0 } else { 0.0 };
            img.put_pixel(x, y, &[v]);
        }
    }
    let mut point = inputs_at(img.clone(), 0.5, 0.5, 1);
    let rp = OpColorSampleSamplePixel::run(&mut point).await.unwrap();
    assert!((dec(&rp.responses[1].value) - 1.0).abs() < 1e-4, "diameter 1 should hit the white centre");

    let mut area = inputs_at(img, 0.5, 0.5, 3);
    let ra = OpColorSampleSamplePixel::run(&mut area).await.unwrap();
    let avg = dec(&ra.responses[1].value);
    assert!(
        (avg - 1.0 / 9.0).abs() < 1e-4,
        "diameter 3 should average the 3×3 block to 1/9, got {avg}"
    );
}

#[tokio::test]
async fn test_sample_pixel_diameter_clips_at_edge() {
    // Solid white 2×2. Sampling the top-left corner with a large disk still
    // only sees white (edge clip, no OOB).
    let img = FloatImage::from_pixel(2, 2, 1, &[1.0]);
    let mut inputs = inputs_at(img, 0.0, 0.0, 64);
    let r = OpColorSampleSamplePixel::run(&mut inputs).await.unwrap();
    assert!((dec(&r.responses[1].value) - 1.0).abs() < 1e-5);
}

#[tokio::test]
async fn test_sample_pixel_diameter_premultiplied_no_bleed() {
    // Left half opaque red, right half fully transparent with hidden green.
    // Averaging the boundary must not pull green into the result.
    let mut img = FloatImage::new(4, 1, 4);
    img.put_pixel(0, 0, &[1.0, 0.0, 0.0, 1.0]);
    img.put_pixel(1, 0, &[1.0, 0.0, 0.0, 1.0]);
    img.put_pixel(2, 0, &[0.0, 1.0, 0.0, 0.0]);
    img.put_pixel(3, 0, &[0.0, 1.0, 0.0, 0.0]);
    // Centre between the two reds (x=0.5 → pixel coord 1.5 on a 4-wide image
    // with centres basis: 0.5 * 3 = 1.5). Diameter 3 reaches reds + transparent.
    let mut inputs = inputs_at(img, 0.5, 0.0, 3);
    let r = OpColorSampleSamplePixel::run(&mut inputs).await.unwrap();
    let red = dec(&r.responses[1].value);
    let green = dec(&r.responses[2].value);
    assert!(red > 0.9, "result should stay red, got r={red}");
    assert!(green < 0.05, "hidden green must not bleed, got g={green}");
}
