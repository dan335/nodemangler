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

#[tokio::test]
async fn test_resize_fill_settings() {
    let s = OpImageTransformResizeFill::settings();
    assert_eq!(s.name, "resize fill");
    assert_eq!(OpImageTransformResizeFill::create_inputs().len(), 4);
    assert_eq!(OpImageTransformResizeFill::create_outputs().len(), 3);
}

#[tokio::test]
async fn test_resize_fill() {
    let mut inputs = vec![
        Input::new("image".to_string(), image_input(8, 8), None, None),
        Input::new("width".to_string(), Value::Integer(4), None, None),
        Input::new("height".to_string(), Value::Integer(4), None, None),
        Input::new("filter type".to_string(), Value::FilterType(image::imageops::FilterType::Gaussian), None, None),
    ];
    let result = OpImageTransformResizeFill::run(&mut inputs).await.unwrap();
    assert_eq!(result.responses.len(), 3);
    match &result.responses[0].value {
        Value::Image { .. } => {}
        other => panic!("Expected Image, got {:?}", other),
    }
}

#[tokio::test]
async fn test_resize_fill_1x1() {
    let mut inputs = vec![
        Input::new("image".to_string(), image_input(8, 8), None, None),
        Input::new("width".to_string(), Value::Integer(1), None, None),
        Input::new("height".to_string(), Value::Integer(1), None, None),
        Input::new("filter type".to_string(), Value::FilterType(image::imageops::FilterType::Nearest), None, None),
    ];
    let result = OpImageTransformResizeFill::run(&mut inputs).await;
    assert!(result.is_ok(), "resize_fill to 1x1 failed: {:?}", result.err());
}

#[tokio::test]
async fn test_resize_fill_gives_exact_dimensions() {
    // resize_to_fill crops and resizes to exactly the requested dimensions
    let mut inputs = vec![
        Input::new("image".to_string(), image_input(8, 4), None, None),
        Input::new("width".to_string(), Value::Integer(12), None, None),
        Input::new("height".to_string(), Value::Integer(6), None, None),
        Input::new("filter type".to_string(), Value::FilterType(image::imageops::FilterType::Nearest), None, None),
    ];
    let result = OpImageTransformResizeFill::run(&mut inputs).await.unwrap();
    match &result.responses[0].value {
        Value::Image { data, .. } => {
            assert_eq!(data.width(), 12, "resize_fill must give exact width");
            assert_eq!(data.height(), 6, "resize_fill must give exact height");
        }
        other => panic!("Expected Image, got {:?}", other),
    }
}

#[tokio::test]
async fn test_resize_fill_outputs_width_height() {
    let mut inputs = vec![
        Input::new("image".to_string(), image_input(8, 8), None, None),
        Input::new("width".to_string(), Value::Integer(5), None, None),
        Input::new("height".to_string(), Value::Integer(7), None, None),
        Input::new("filter type".to_string(), Value::FilterType(image::imageops::FilterType::Nearest), None, None),
    ];
    let result = OpImageTransformResizeFill::run(&mut inputs).await.unwrap();
    match &result.responses[1].value {
        Value::Integer(w) => assert_eq!(*w, 5),
        other => panic!("Expected Integer width output, got {:?}", other),
    }
    match &result.responses[2].value {
        Value::Integer(h) => assert_eq!(*h, 7),
        other => panic!("Expected Integer height output, got {:?}", other),
    }
}

#[tokio::test]
/// resize_fill must not bleed a transparent pixel's hidden colour into
/// opaque neighbours: black on hidden-white transparent background stays
/// black at the edges (the bug_02 white-fringe regression).
async fn test_resize_fill_no_hidden_color_bleed() {
    // Left half: opaque black. Right half: fully transparent, hidden colour white.
    let mut img = FloatImage::new(16, 16, 4);
    for y in 0..16 {
        for x in 0..16 {
            if x < 8 {
                img.put_pixel(x, y, &[0.0, 0.0, 0.0, 1.0]);
            } else {
                img.put_pixel(x, y, &[1.0, 1.0, 1.0, 0.0]);
            }
        }
    }
    let mut inputs = vec![
        Input::new("image".to_string(), Value::Image { data: Arc::new(img), change_id: get_id() }, None, None),
        Input::new("width".to_string(), Value::Integer(8), None, None),
        Input::new("height".to_string(), Value::Integer(4), None, None),
        Input::new("filter type".to_string(), Value::FilterType(image::imageops::FilterType::Gaussian), None, None),
    ];
    let result = OpImageTransformResizeFill::run(&mut inputs).await.unwrap();
    match &result.responses[0].value {
        Value::Image { data, .. } => {
            for (x, y, px) in data.enumerate_pixels() {
                let (r, g, b, a) = (px[0], px[1], px[2], px[3]);
                if a > 0.01 {
                    assert!(
                        r < 0.05 && g < 0.05 && b < 0.05,
                        "hidden colour bled into visible pixel at ({}, {}): {:?}",
                        x, y, px
                    );
                }
            }
        }
        other => panic!("Expected Image, got {:?}", other),
    }
}
