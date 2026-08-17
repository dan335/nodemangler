use super::*;

use crate::float_image::FloatImage;
use crate::get_id;
use crate::input::Input;
use crate::value::Value;
use std::sync::Arc;

/// Creates a test FloatImage with an x/y gradient pattern (4 channels).
fn test_image(w: u32, h: u32) -> Arc<FloatImage> {
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

/// Creates a Value::Image from a test gradient image.
fn image_input(w: u32, h: u32) -> Value {
    Value::Image { data: test_image(w, h), change_id: get_id() }
}

/// Builds the input list for a crop of an `w`x`h` gradient image, with the crop
/// region given as 0-1 fractions of the source size. Aspect defaults to free.
fn crop_inputs(w: u32, h: u32, x: f32, y: f32, cw: f32, ch: f32) -> Vec<Input> {
    crop_inputs_aspect(w, h, x, y, cw, ch, 0, 0)
}

/// Same as [`crop_inputs`] with an explicit aspect-lock pair.
fn crop_inputs_aspect(
    w: u32,
    h: u32,
    x: f32,
    y: f32,
    cw: f32,
    ch: f32,
    ratio_w: i32,
    ratio_h: i32,
) -> Vec<Input> {
    vec![
        Input::new("image".to_string(), image_input(w, h), None, None),
        Input::new("x".to_string(), Value::Decimal(x), None, None),
        Input::new("y".to_string(), Value::Decimal(y), None, None),
        Input::new("width".to_string(), Value::Decimal(cw), None, None),
        Input::new("height".to_string(), Value::Decimal(ch), None, None),
        Input::new("aspect w".to_string(), Value::Integer(ratio_w), None, None),
        Input::new("aspect h".to_string(), Value::Integer(ratio_h), None, None),
    ]
}

#[tokio::test]
async fn test_crop_settings() {
    let s = OpImageTransformCrop::settings();
    assert_eq!(s.name, "crop");
    assert_eq!(OpImageTransformCrop::create_inputs().len(), 7);
    assert_eq!(OpImageTransformCrop::create_outputs().len(), 3);
    let inputs = OpImageTransformCrop::create_inputs();
    assert_eq!(inputs[5].name, "aspect w");
    assert_eq!(inputs[6].name, "aspect h");
    assert!(matches!(inputs[5].value, Value::Integer(0)));
    assert!(matches!(inputs[6].value, Value::Integer(0)));
}

#[tokio::test]
async fn test_crop() {
    let mut inputs = crop_inputs(8, 8, 0.125, 0.125, 0.5, 0.5);
    let result = OpImageTransformCrop::run(&mut inputs).await.unwrap();
    assert_eq!(result.responses.len(), 3);
    match &result.responses[0].value {
        Value::Image { .. } => {}
        other => panic!("Expected Image, got {:?}", other),
    }
}

#[tokio::test]
async fn test_crop_output_dimensions() {
    // Fractions resolve against the source size: half of 8 = 4, 3/8 of 8 = 3.
    let mut inputs = crop_inputs(8, 8, 0.0, 0.0, 0.5, 0.375);
    let result = OpImageTransformCrop::run(&mut inputs).await.unwrap();
    match &result.responses[0].value {
        Value::Image { data, .. } => {
            assert_eq!(data.width(), 4);
            assert_eq!(data.height(), 3);
        }
        other => panic!("Expected Image, got {:?}", other),
    }
    // The width/height outputs report real pixels, not fractions.
    match (&result.responses[1].value, &result.responses[2].value) {
        (Value::Integer(w), Value::Integer(h)) => {
            assert_eq!(*w, 4);
            assert_eq!(*h, 3);
        }
        other => panic!("Expected Integer width/height, got {:?}", other),
    }
}

#[tokio::test]
async fn test_crop_is_resolution_independent() {
    // The same fractions must frame the same relative region at any input size.
    for (w, h, expect_w, expect_h) in [(8u32, 8u32, 2u32, 2u32), (64, 64, 16, 16), (100, 40, 25, 10)] {
        let mut inputs = crop_inputs(w, h, 0.25, 0.25, 0.25, 0.25);
        let result = OpImageTransformCrop::run(&mut inputs).await.unwrap();
        match &result.responses[0].value {
            Value::Image { data, .. } => {
                assert_eq!(data.width(), expect_w, "width for {}x{} source", w, h);
                assert_eq!(data.height(), expect_h, "height for {}x{} source", w, h);
            }
            other => panic!("Expected Image, got {:?}", other),
        }
    }
}

#[tokio::test]
async fn test_crop_offorigin_clips_to_edge() {
    // A region extending past the right/bottom edge must clip to what remains
    // past the origin, not edge-replicate past-the-edge pixels.
    let mut inputs = crop_inputs(8, 8, 0.75, 0.625, 1.0, 1.0);
    let result = OpImageTransformCrop::run(&mut inputs).await.unwrap();
    match &result.responses[0].value {
        Value::Image { data, .. } => {
            assert_eq!(data.width(), 2, "width should clip to img_w - x");
            assert_eq!(data.height(), 3, "height should clip to img_h - y");
        }
        other => panic!("Expected Image, got {:?}", other),
    }
}

#[tokio::test]
async fn test_crop_full_image() {
    // A full-frame crop gives back the same dimensions and the same pixels.
    let source = test_image(8, 8);
    let mut inputs = vec![
        Input::new("image".to_string(), Value::Image { data: source.clone(), change_id: get_id() }, None, None),
        Input::new("x".to_string(), Value::Decimal(0.0), None, None),
        Input::new("y".to_string(), Value::Decimal(0.0), None, None),
        Input::new("width".to_string(), Value::Decimal(1.0), None, None),
        Input::new("height".to_string(), Value::Decimal(1.0), None, None),
        Input::new("aspect w".to_string(), Value::Integer(0), None, None),
        Input::new("aspect h".to_string(), Value::Integer(0), None, None),
    ];
    let result = OpImageTransformCrop::run(&mut inputs).await.unwrap();
    match &result.responses[0].value {
        Value::Image { data, .. } => {
            assert_eq!(data.width(), 8);
            assert_eq!(data.height(), 8);
            assert_eq!(data.get_pixel(5, 6), source.get_pixel(5, 6));
        }
        other => panic!("Expected Image, got {:?}", other),
    }
}

#[tokio::test]
async fn test_crop_copies_the_right_region() {
    // The crop's top-left pixel must be the source pixel at the fractional origin.
    let source = test_image(8, 8);
    let mut inputs = vec![
        Input::new("image".to_string(), Value::Image { data: source.clone(), change_id: get_id() }, None, None),
        Input::new("x".to_string(), Value::Decimal(0.25), None, None),
        Input::new("y".to_string(), Value::Decimal(0.5), None, None),
        Input::new("width".to_string(), Value::Decimal(0.25), None, None),
        Input::new("height".to_string(), Value::Decimal(0.25), None, None),
        Input::new("aspect w".to_string(), Value::Integer(0), None, None),
        Input::new("aspect h".to_string(), Value::Integer(0), None, None),
    ];
    let result = OpImageTransformCrop::run(&mut inputs).await.unwrap();
    match &result.responses[0].value {
        Value::Image { data, .. } => {
            assert_eq!(data.get_pixel(0, 0), source.get_pixel(2, 4));
            assert_eq!(data.get_pixel(1, 1), source.get_pixel(3, 5));
        }
        other => panic!("Expected Image, got {:?}", other),
    }
}

#[tokio::test]
async fn test_crop_zero_size_keeps_one_pixel() {
    // A zero (or negative) size must still produce a valid 1x1 image.
    let mut inputs = crop_inputs(8, 8, 0.5, 0.5, 0.0, 0.0);
    let result = OpImageTransformCrop::run(&mut inputs).await.unwrap();
    match &result.responses[0].value {
        Value::Image { data, .. } => {
            assert_eq!(data.width(), 1);
            assert_eq!(data.height(), 1);
        }
        other => panic!("Expected Image, got {:?}", other),
    }
}

#[tokio::test]
async fn test_crop_origin_past_edge_clamps_inside() {
    // x/y of 1.0 lands on the last row/column rather than off the image.
    let mut inputs = crop_inputs(8, 8, 1.0, 1.0, 1.0, 1.0);
    let result = OpImageTransformCrop::run(&mut inputs).await.unwrap();
    match &result.responses[0].value {
        Value::Image { data, .. } => {
            assert_eq!(data.width(), 1);
            assert_eq!(data.height(), 1);
        }
        other => panic!("Expected Image, got {:?}", other),
    }
}

#[tokio::test]
async fn test_crop_preserves_channel_count() {
    for channels in [1u32, 2, 3, 4] {
        let mut img = FloatImage::new(8, 8, channels);
        for y in 0..8 {
            for x in 0..8 {
                let px: Vec<f32> = (0..channels).map(|c| (x + y + c) as f32 * 0.01).collect();
                img.put_pixel(x, y, &px);
            }
        }
        let mut inputs = vec![
            Input::new("image".to_string(), Value::Image { data: Arc::new(img), change_id: get_id() }, None, None),
            Input::new("x".to_string(), Value::Decimal(0.0), None, None),
            Input::new("y".to_string(), Value::Decimal(0.0), None, None),
            Input::new("width".to_string(), Value::Decimal(0.5), None, None),
            Input::new("height".to_string(), Value::Decimal(0.5), None, None),
            Input::new("aspect w".to_string(), Value::Integer(0), None, None),
            Input::new("aspect h".to_string(), Value::Integer(0), None, None),
        ];
        let result = OpImageTransformCrop::run(&mut inputs).await.unwrap();
        match &result.responses[0].value {
            Value::Image { data, .. } => assert_eq!(data.channels(), channels),
            other => panic!("Expected Image, got {:?}", other),
        }
    }
}

#[test]
fn resolve_crop_free_matches_historical_rounding() {
    // Unconstrained (either side 0) must reproduce the old far-edge-from-origin
    // rounding, including the 1-pixel floor and the clip-at-edge behaviour.
    let p = resolve_crop(0.25, 0.25, 0.5, 0.5, 0, 0, 512, 256);
    assert_eq!(p, PixelCrop { x: 128, y: 64, w: 256, h: 128 });
    let p = resolve_crop(0.75, 0.625, 1.0, 1.0, 0, 0, 8, 8);
    assert_eq!(p, PixelCrop { x: 6, y: 5, w: 2, h: 3 });
    let p = resolve_crop(0.5, 0.5, 0.0, 0.0, 16, 0, 8, 8);
    assert_eq!(p, PixelCrop { x: 4, y: 4, w: 1, h: 1 });
}

#[test]
fn resolve_crop_square_lock_centers_in_a_wide_image() {
    // Full-frame 8×4 locked to 1:1: largest square is 4×4, centered at x = 2.
    let p = resolve_crop(0.0, 0.0, 1.0, 1.0, 1, 1, 8, 4);
    assert_eq!(p, PixelCrop { x: 2, y: 0, w: 4, h: 4 });
}

#[test]
fn resolve_crop_sixteen_nine_on_a_square() {
    // Full-frame 8×8 locked to 16:9. The closest integer fit that keeps the
    // width is 8×5 (round(8×9/16) = 5), centered at y = 1.
    let p = resolve_crop(0.0, 0.0, 1.0, 1.0, 16, 9, 8, 8);
    assert_eq!(p, PixelCrop { x: 0, y: 1, w: 8, h: 5 });
}

#[test]
fn resolve_crop_already_on_ratio_keeps_its_origin() {
    // 0.5 × 0.25 of an 8×8 is 4×2 — already 2:1 — so the lock must not
    // recenter a box that is already correct.
    let p = resolve_crop(0.0, 0.0, 0.5, 0.25, 2, 1, 8, 8);
    assert_eq!(p, PixelCrop { x: 0, y: 0, w: 4, h: 2 });
}

#[test]
fn resolve_crop_offorigin_window_shrinks_and_stays_inside() {
    // A 2×2 corner leftover cannot hold a 16:9 at full size; the fit must
    // shrink (2×1) and remain inside the source.
    let p = resolve_crop(0.75, 0.75, 1.0, 1.0, 16, 9, 8, 8);
    assert_eq!(p.w, 2);
    assert_eq!(p.h, 1);
    assert!(p.x >= 0 && p.y >= 0);
    assert!(p.x + p.w <= 8 && p.y + p.h <= 8);
    assert!(p.w >= 1 && p.h >= 1);
}

#[tokio::test]
async fn test_crop_aspect_square_centers_in_wide_image() {
    let mut inputs = crop_inputs_aspect(8, 4, 0.0, 0.0, 1.0, 1.0, 1, 1);
    let result = OpImageTransformCrop::run(&mut inputs).await.unwrap();
    match &result.responses[0].value {
        Value::Image { data, .. } => {
            assert_eq!(data.width(), 4);
            assert_eq!(data.height(), 4);
            // Centered: output (0,0) is source (2,0).
            let source = test_image(8, 4);
            assert_eq!(data.get_pixel(0, 0), source.get_pixel(2, 0));
        }
        other => panic!("Expected Image, got {:?}", other),
    }
    match (&result.responses[1].value, &result.responses[2].value) {
        (Value::Integer(w), Value::Integer(h)) => {
            assert_eq!((*w, *h), (4, 4));
        }
        other => panic!("Expected Integer width/height, got {other:?}"),
    }
}

#[tokio::test]
async fn test_crop_aspect_sixteen_nine_on_square() {
    let mut inputs = crop_inputs_aspect(8, 8, 0.0, 0.0, 1.0, 1.0, 16, 9);
    let result = OpImageTransformCrop::run(&mut inputs).await.unwrap();
    match &result.responses[0].value {
        Value::Image { data, .. } => {
            assert_eq!((data.width(), data.height()), (8, 5));
        }
        other => panic!("Expected Image, got {:?}", other),
    }
}

#[tokio::test]
async fn test_crop_aspect_already_on_ratio_does_not_recenter() {
    let mut inputs = crop_inputs_aspect(8, 8, 0.0, 0.0, 0.5, 0.25, 2, 1);
    let result = OpImageTransformCrop::run(&mut inputs).await.unwrap();
    match &result.responses[0].value {
        Value::Image { data, .. } => {
            assert_eq!((data.width(), data.height()), (4, 2));
            // Origin kept: output (0,0) is source (0,0), not a recentered (0, 3).
            let source = test_image(8, 8);
            assert_eq!(data.get_pixel(0, 0), source.get_pixel(0, 0));
        }
        other => panic!("Expected Image, got {:?}", other),
    }
}

#[tokio::test]
async fn test_crop_aspect_one_side_zero_is_free() {
    // A wired-style half-pair (16, 0) must not lock — full-frame stays full-frame.
    let mut inputs = crop_inputs_aspect(8, 8, 0.0, 0.0, 1.0, 1.0, 16, 0);
    let result = OpImageTransformCrop::run(&mut inputs).await.unwrap();
    match &result.responses[0].value {
        Value::Image { data, .. } => {
            assert_eq!((data.width(), data.height()), (8, 8));
        }
        other => panic!("Expected Image, got {:?}", other),
    }
}
