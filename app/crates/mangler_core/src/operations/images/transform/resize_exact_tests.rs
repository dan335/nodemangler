use super::*;

use crate::get_id;
use crate::input::Input;
use crate::value::Value;
use image::DynamicImage;
use std::sync::Arc;

fn test_image(w: u32, h: u32) -> Arc<DynamicImage> {
    let mut imgbuf = image::RgbaImage::new(w, h);
    for (x, y, pixel) in imgbuf.enumerate_pixels_mut() {
        let r = (x * 255 / w.max(1)) as u8;
        let g = (y * 255 / h.max(1)) as u8;
        *pixel = image::Rgba([r, g, 128, 255]);
    }
    Arc::new(DynamicImage::ImageRgba8(imgbuf))
}

fn image_input(w: u32, h: u32) -> Value {
    Value::DynamicImage { data: test_image(w, h), change_id: get_id() }
}

#[tokio::test]
async fn test_resize_exact_settings() {
    let s = OpImageTransformResizeExact::settings();
    assert_eq!(s.name, "resize exact");
    assert_eq!(OpImageTransformResizeExact::create_inputs().len(), 4);
    assert_eq!(OpImageTransformResizeExact::create_outputs().len(), 3);
}

#[tokio::test]
async fn test_resize_exact() {
    let mut inputs = vec![
        Input::new("image".to_string(), image_input(8, 8), None, None),
        Input::new("width".to_string(), Value::Integer(16), None, None),
        Input::new("height".to_string(), Value::Integer(4), None, None),
        Input::new("filter type".to_string(), Value::FilterType(image::imageops::FilterType::Gaussian), None, None),
    ];
    let result = OpImageTransformResizeExact::run(&mut inputs).await.unwrap();
    assert_eq!(result.responses.len(), 3);
    match &result.responses[0].value {
        Value::DynamicImage { data, .. } => {
            assert_eq!(data.width(), 16);
            assert_eq!(data.height(), 4);
        }
        other => panic!("Expected DynamicImage, got {:?}", other),
    }
}

#[tokio::test]
async fn test_resize_exact_1x1() {
    let mut inputs = vec![
        Input::new("image".to_string(), image_input(8, 8), None, None),
        Input::new("width".to_string(), Value::Integer(1), None, None),
        Input::new("height".to_string(), Value::Integer(1), None, None),
        Input::new("filter type".to_string(), Value::FilterType(image::imageops::FilterType::Nearest), None, None),
    ];
    let result = OpImageTransformResizeExact::run(&mut inputs).await;
    assert!(result.is_ok(), "resize_exact to 1x1 failed: {:?}", result.err());
}

#[tokio::test]
async fn test_resize_exact_always_gives_requested_dimensions() {
    // Unlike resize (aspect-ratio preserving), resize_exact must give exactly requested dims
    let mut inputs = vec![
        Input::new("image".to_string(), image_input(4, 4), None, None),
        Input::new("width".to_string(), Value::Integer(20), None, None),
        Input::new("height".to_string(), Value::Integer(3), None, None),
        Input::new("filter type".to_string(), Value::FilterType(image::imageops::FilterType::Nearest), None, None),
    ];
    let result = OpImageTransformResizeExact::run(&mut inputs).await.unwrap();
    match &result.responses[0].value {
        Value::DynamicImage { data, .. } => {
            assert_eq!(data.width(), 20, "resize_exact must give exact width");
            assert_eq!(data.height(), 3, "resize_exact must give exact height");
        }
        other => panic!("Expected DynamicImage, got {:?}", other),
    }
    // Also verify the width/height outputs
    match &result.responses[1].value {
        Value::Integer(w) => assert_eq!(*w, 20),
        other => panic!("Expected Integer width, got {:?}", other),
    }
    match &result.responses[2].value {
        Value::Integer(h) => assert_eq!(*h, 3),
        other => panic!("Expected Integer height, got {:?}", other),
    }
}
