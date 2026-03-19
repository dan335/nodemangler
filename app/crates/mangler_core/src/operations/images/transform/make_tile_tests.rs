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
async fn test_make_tile_settings() {
    let s = OpImageTransformMakeTile::settings();
    assert_eq!(s.name, "make tile");
    assert_eq!(OpImageTransformMakeTile::create_inputs().len(), 2);
    assert_eq!(OpImageTransformMakeTile::create_outputs().len(), 1);
}

#[tokio::test]
async fn test_make_tile_basic() {
    let mut inputs = vec![
        Input::new("image".to_string(), image_input(16, 16), None, None),
        Input::new("blend size".to_string(), Value::Decimal(0.25), None, None),
    ];
    let result = OpImageTransformMakeTile::run(&mut inputs).await.unwrap();
    assert_eq!(result.responses.len(), 1);
    match &result.responses[0].value {
        Value::DynamicImage { data, .. } => {
            assert_eq!(data.width(), 16);
            assert_eq!(data.height(), 16);
        }
        other => panic!("Expected DynamicImage, got {:?}", other),
    }
}

#[tokio::test]
async fn test_make_tile_1x1() {
    let mut inputs = vec![
        Input::new("image".to_string(), image_input(1, 1), None, None),
        Input::new("blend size".to_string(), Value::Decimal(0.25), None, None),
    ];
    let result = OpImageTransformMakeTile::run(&mut inputs).await;
    assert!(result.is_ok(), "make_tile 1x1 failed: {:?}", result.err());
}

#[tokio::test]
async fn test_make_tile_preserves_dimensions() {
    let mut inputs = vec![
        Input::new("image".to_string(), image_input(32, 16), None, None),
        Input::new("blend size".to_string(), Value::Decimal(0.1), None, None),
    ];
    let result = OpImageTransformMakeTile::run(&mut inputs).await.unwrap();
    match &result.responses[0].value {
        Value::DynamicImage { data, .. } => {
            assert_eq!(data.width(), 32);
            assert_eq!(data.height(), 16);
        }
        other => panic!("Expected DynamicImage, got {:?}", other),
    }
}

#[tokio::test]
async fn test_make_tile_uniform_image_unchanged() {
    // A uniform image tiled should remain the same uniform color
    let uniform = image::RgbaImage::from_pixel(8, 8, image::Rgba([100u8, 150, 200, 255]));
    let img = Arc::new(DynamicImage::ImageRgba8(uniform));
    let mut inputs = vec![
        Input::new("image".to_string(), Value::DynamicImage { data: img, change_id: get_id() }, None, None),
        Input::new("blend size".to_string(), Value::Decimal(0.25), None, None),
    ];
    let result = OpImageTransformMakeTile::run(&mut inputs).await.unwrap();
    match &result.responses[0].value {
        Value::DynamicImage { data, .. } => {
            let rgba = data.to_rgba8();
            // Blending uniform pixels together still gives the same color
            let p = rgba.get_pixel(4, 4).0;
            assert_eq!(p, [100u8, 150, 200, 255], "uniform image should stay uniform after tiling");
        }
        other => panic!("Expected DynamicImage, got {:?}", other),
    }
}
