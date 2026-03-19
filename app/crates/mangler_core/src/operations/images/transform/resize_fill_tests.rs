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
        Value::DynamicImage { .. } => {}
        other => panic!("Expected DynamicImage, got {:?}", other),
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
        Value::DynamicImage { data, .. } => {
            assert_eq!(data.width(), 12, "resize_fill must give exact width");
            assert_eq!(data.height(), 6, "resize_fill must give exact height");
        }
        other => panic!("Expected DynamicImage, got {:?}", other),
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
