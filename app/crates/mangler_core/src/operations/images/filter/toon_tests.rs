//! Tests for the toon / cel-shade filter operation.

use super::*;

use crate::color::Color;
use crate::float_image::FloatImage;
use crate::get_id;
use crate::input::Input;
use crate::value::Value;
use std::sync::Arc;

fn gradient_image(w: u32, h: u32) -> Arc<FloatImage> {
    let mut img = FloatImage::new(w, h, 4);
    for y in 0..h {
        for x in 0..w {
            let r = x as f32 / w.max(1) as f32;
            let g = y as f32 / h.max(1) as f32;
            img.put_pixel(x, y, &[r, g, 0.5, 1.0]);
        }
    }
    Arc::new(img)
}

/// Default-ish inputs but with smoothing=0 and edge_strength=0 so tests can
/// reason about exact pixel values without the edge overlay step interfering.
fn default_inputs(img: Value) -> Vec<Input> {
    vec![
        Input::new("image".to_string(), img, None, None),
        Input::new("levels".to_string(), Value::Integer(4), None, None),
        Input::new("smoothing".to_string(), Value::Integer(0), None, None),
        Input::new("edge thickness".to_string(), Value::Integer(1), None, None),
        Input::new("edge color".to_string(), Value::Color(Color::default()), None, None),
        Input::new("edge strength".to_string(), Value::Decimal(0.0), None, None),
    ]
}

#[tokio::test]
async fn test_toon_settings() {
    let s = OpImageAdjustmentToon::settings();
    assert_eq!(s.name, "toon");
    assert_eq!(OpImageAdjustmentToon::create_inputs().len(), 6);
    assert_eq!(OpImageAdjustmentToon::create_outputs().len(), 1);
}

#[tokio::test]
async fn test_toon_preserves_dimensions() {
    let mut inputs = default_inputs(Value::Image { data: gradient_image(16, 12), change_id: get_id() });
    let result = OpImageAdjustmentToon::run(&mut inputs).await.unwrap();
    match &result.responses[0].value {
        Value::Image { data, .. } => {
            assert_eq!(data.width(), 16);
            assert_eq!(data.height(), 12);
            assert_eq!(data.channels(), 4);
        }
        other => panic!("Expected Image, got {:?}", other),
    }
}

#[tokio::test]
async fn test_toon_quantizes_lightness() {
    // Pure-grey gradient with smoothing=0, edges off, levels=2 → every pixel
    // should land on lightness 0 or lightness 1 (i.e. solid black or solid white).
    let mut img = FloatImage::new(8, 8, 4);
    for y in 0..8 {
        for x in 0..8 {
            let v = x as f32 / 7.0;
            img.put_pixel(x, y, &[v, v, v, 1.0]);
        }
    }
    let mut inputs = default_inputs(Value::Image { data: Arc::new(img), change_id: get_id() });
    inputs[1] = Input::new("levels".to_string(), Value::Integer(2), None, None);
    let result = OpImageAdjustmentToon::run(&mut inputs).await.unwrap();
    match &result.responses[0].value {
        Value::Image { data, .. } => {
            for pixel in data.pixels() {
                let v = pixel[0];
                assert!((v - 0.0).abs() < 1e-3 || (v - 1.0).abs() < 1e-3,
                        "lightness not quantized: {}", v);
                // grey input must stay grey through HSL round-trip
                assert!((pixel[0] - pixel[1]).abs() < 1e-3, "channels diverged: {:?}", pixel);
                assert!((pixel[1] - pixel[2]).abs() < 1e-3, "channels diverged: {:?}", pixel);
            }
        }
        other => panic!("Expected Image, got {:?}", other),
    }
}

#[tokio::test]
async fn test_toon_preserves_hue() {
    // A solid red image at lightness 0.5 should still be red after the HSL
    // round-trip — only the lightness gets quantized, hue must survive.
    let img = Arc::new(FloatImage::from_pixel(8, 8, 4, &[1.0, 0.0, 0.0, 1.0]));
    let mut inputs = default_inputs(Value::Image { data: img, change_id: get_id() });
    let result = OpImageAdjustmentToon::run(&mut inputs).await.unwrap();
    match &result.responses[0].value {
        Value::Image { data, .. } => {
            let p = data.get_pixel(4, 4);
            // R should still dominate (hue preserved); G and B should stay near 0
            assert!(p[0] > 0.9, "red channel lost: {:?}", p);
            assert!(p[1] < 0.1, "green leaked in: {:?}", p);
            assert!(p[2] < 0.1, "blue leaked in: {:?}", p);
        }
        other => panic!("Expected Image, got {:?}", other),
    }
}

#[tokio::test]
async fn test_toon_smoothing_blurs_color() {
    // With smoothing > 0, a sharp red/blue boundary should mix near the seam.
    let mut img = FloatImage::new(16, 16, 4);
    for y in 0..16 {
        for x in 0..16 {
            if x < 8 {
                img.put_pixel(x, y, &[1.0, 0.0, 0.0, 1.0]);
            } else {
                img.put_pixel(x, y, &[0.0, 0.0, 1.0, 1.0]);
            }
        }
    }
    let mut inputs = default_inputs(Value::Image { data: Arc::new(img), change_id: get_id() });
    // levels=8 keeps quantization fine enough that the smoothed midtone shows up
    inputs[1] = Input::new("levels".to_string(), Value::Integer(8), None, None);
    inputs[2] = Input::new("smoothing".to_string(), Value::Integer(3), None, None);
    let result = OpImageAdjustmentToon::run(&mut inputs).await.unwrap();
    match &result.responses[0].value {
        Value::Image { data, .. } => {
            // pixel right next to the seam should pick up some blue from the right side
            let p = data.get_pixel(7, 8);
            assert!(p[2] > 0.05, "smoothing didn't bleed blue across the seam: {:?}", p);
        }
        other => panic!("Expected Image, got {:?}", other),
    }
}

#[tokio::test]
async fn test_toon_edges_on_cel_boundary() {
    // A black/white step quantizes to two cel bands meeting at x=8 (with
    // levels=2). The pixels right on either side of the boundary should be
    // tinted by the red edge color.
    let mut img = FloatImage::new(32, 32, 4);
    for y in 0..32 {
        for x in 0..32 {
            let v = if x < 16 { 0.0 } else { 1.0 };
            img.put_pixel(x, y, &[v, v, v, 1.0]);
        }
    }
    let red = Color::from_srgb_float(1.0, 0.0, 0.0, 1.0);
    let mut inputs = vec![
        Input::new("image".to_string(), Value::Image { data: Arc::new(img), change_id: get_id() }, None, None),
        Input::new("levels".to_string(), Value::Integer(2), None, None),
        Input::new("smoothing".to_string(), Value::Integer(0), None, None),
        Input::new("edge thickness".to_string(), Value::Decimal(0.0), None, None),
        Input::new("edge color".to_string(), Value::Color(red), None, None),
        Input::new("edge strength".to_string(), Value::Decimal(1.0), None, None),
    ];
    let result = OpImageAdjustmentToon::run(&mut inputs).await.unwrap();
    match &result.responses[0].value {
        Value::Image { data, .. } => {
            // pixels right on the cel boundary become solid edge color (red)
            let left_edge = data.get_pixel(15, 16);
            let right_edge = data.get_pixel(16, 16);
            assert!(left_edge[0] > 0.95 && left_edge[1] < 0.05 && left_edge[2] < 0.05,
                    "left-of-boundary not red: {:?}", left_edge);
            assert!(right_edge[0] > 0.95 && right_edge[1] < 0.05 && right_edge[2] < 0.05,
                    "right-of-boundary not red: {:?}", right_edge);
            // pixels well away from the boundary keep their quantized colour
            let far = data.get_pixel(0, 16);
            assert!(far[0] < 0.05, "far left tinted red: {:?}", far);
        }
        other => panic!("Expected Image, got {:?}", other),
    }
}

#[tokio::test]
async fn test_toon_edge_thickness_widens_outline() {
    // With thickness > 0 the binary edge mask is blurred so adjacent pixels
    // also pick up some edge tint. Verify that pixels two columns away from
    // the cel boundary get a non-trivial red contribution.
    let mut img = FloatImage::new(32, 32, 4);
    for y in 0..32 {
        for x in 0..32 {
            let v = if x < 16 { 0.0 } else { 1.0 };
            img.put_pixel(x, y, &[v, v, v, 1.0]);
        }
    }
    let red = Color::from_srgb_float(1.0, 0.0, 0.0, 1.0);
    let mut inputs = vec![
        Input::new("image".to_string(), Value::Image { data: Arc::new(img), change_id: get_id() }, None, None),
        Input::new("levels".to_string(), Value::Integer(2), None, None),
        Input::new("smoothing".to_string(), Value::Integer(0), None, None),
        Input::new("edge thickness".to_string(), Value::Decimal(2.0), None, None),
        Input::new("edge color".to_string(), Value::Color(red), None, None),
        Input::new("edge strength".to_string(), Value::Decimal(1.0), None, None),
    ];
    let result = OpImageAdjustmentToon::run(&mut inputs).await.unwrap();
    match &result.responses[0].value {
        Value::Image { data, .. } => {
            // a pixel two columns into the white side should still pick up red
            let p = data.get_pixel(18, 16);
            assert!(p[0] > 0.5 && p[2] < 0.6, "thickened edge didn't reach 2px out: {:?}", p);
        }
        other => panic!("Expected Image, got {:?}", other),
    }
}

#[tokio::test]
async fn test_toon_edge_strength_zero_skips_edges() {
    // edge_strength=0 should leave the quantized image untouched (no edge tint).
    let mut img = FloatImage::new(16, 16, 4);
    for y in 0..16 {
        for x in 0..16 {
            let v = if x < 8 { 0.0 } else { 1.0 };
            img.put_pixel(x, y, &[v, v, v, 1.0]);
        }
    }
    let red = Color::from_srgb_float(1.0, 0.0, 0.0, 1.0);
    let mut inputs = default_inputs(Value::Image { data: Arc::new(img), change_id: get_id() });
    inputs[4] = Input::new("edge color".to_string(), Value::Color(red), None, None);
    inputs[5] = Input::new("edge strength".to_string(), Value::Decimal(0.0), None, None);
    let result = OpImageAdjustmentToon::run(&mut inputs).await.unwrap();
    match &result.responses[0].value {
        Value::Image { data, .. } => {
            // confirm no red leaked anywhere — every pixel stays grey
            for pixel in data.pixels() {
                assert!((pixel[0] - pixel[1]).abs() < 1e-3, "edge red leaked: {:?}", pixel);
                assert!((pixel[1] - pixel[2]).abs() < 1e-3, "edge red leaked: {:?}", pixel);
            }
        }
        other => panic!("Expected Image, got {:?}", other),
    }
}

#[tokio::test]
async fn test_toon_output_range() {
    let mut inputs = default_inputs(Value::Image { data: gradient_image(8, 8), change_id: get_id() });
    let result = OpImageAdjustmentToon::run(&mut inputs).await.unwrap();
    match &result.responses[0].value {
        Value::Image { data, .. } => {
            for pixel in data.pixels() {
                for c in 0..pixel.len() {
                    assert!(pixel[c] >= 0.0 && pixel[c] <= 1.0, "out of range: {}", pixel[c]);
                }
            }
        }
        other => panic!("Expected Image, got {:?}", other),
    }
}
