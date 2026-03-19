use super::*;
use image::{DynamicImage, ImageBuffer, Rgba, Rgb, Luma, LumaA};
use std::sync::Arc;

/// Helper: a wrapper so we can use serde_json with our custom serialize/deserialize
#[derive(serde::Serialize, serde::Deserialize)]
struct Wrapper {
    #[serde(serialize_with = "super::serialize", deserialize_with = "super::deserialize")]
    image: Arc<DynamicImage>,
}

fn roundtrip(img: DynamicImage) {
    let original_width = img.width();
    let original_height = img.height();
    let original_color = img.color();
    let original_bytes: Vec<u8> = img.as_bytes().to_vec();

    let wrapper = Wrapper { image: Arc::new(img) };
    let json = serde_json::to_string(&wrapper).expect("serialize failed");
    let restored: Wrapper = serde_json::from_str(&json).expect("deserialize failed");

    assert_eq!(restored.image.width(), original_width);
    assert_eq!(restored.image.height(), original_height);
    assert_eq!(restored.image.color(), original_color);
    assert_eq!(restored.image.as_bytes(), &original_bytes);
}

// --- Roundtrip tests for each color type ---

#[test]
fn roundtrip_rgba8() {
    let buf = ImageBuffer::from_fn(4, 3, |x, y| {
        Rgba([x as u8 * 10, y as u8 * 20, 128, 255])
    });
    roundtrip(DynamicImage::ImageRgba8(buf));
}

#[test]
fn roundtrip_rgb8() {
    let buf = ImageBuffer::from_fn(3, 2, |x, y| {
        Rgb([x as u8 * 50, y as u8 * 100, 200])
    });
    roundtrip(DynamicImage::ImageRgb8(buf));
}

#[test]
fn roundtrip_luma8() {
    let buf = ImageBuffer::from_fn(5, 5, |x, y| {
        Luma([(x * y) as u8])
    });
    roundtrip(DynamicImage::ImageLuma8(buf));
}

#[test]
fn roundtrip_luma_a8() {
    let buf = ImageBuffer::from_fn(2, 2, |x, y| {
        LumaA([(x + y) as u8 * 30, 200])
    });
    roundtrip(DynamicImage::ImageLumaA8(buf));
}

#[test]
fn roundtrip_rgba16() {
    let buf = ImageBuffer::from_fn(2, 2, |x, y| {
        image::Rgba([x as u16 * 1000, y as u16 * 2000, 30000, 65535])
    });
    roundtrip(DynamicImage::ImageRgba16(buf));
}

#[test]
fn roundtrip_rgb16() {
    let buf = ImageBuffer::from_fn(2, 3, |x, y| {
        image::Rgb([x as u16 * 5000, y as u16 * 10000, 40000])
    });
    roundtrip(DynamicImage::ImageRgb16(buf));
}

#[test]
fn roundtrip_luma16() {
    let buf = ImageBuffer::from_fn(3, 3, |x, y| {
        image::Luma([(x * y * 1000) as u16])
    });
    roundtrip(DynamicImage::ImageLuma16(buf));
}

#[test]
fn roundtrip_luma_a16() {
    let buf = ImageBuffer::from_fn(2, 2, |x, y| {
        image::LumaA([(x + y) as u16 * 500, 60000])
    });
    roundtrip(DynamicImage::ImageLumaA16(buf));
}

#[test]
fn roundtrip_rgb32f() {
    let buf = ImageBuffer::from_fn(2, 2, |x, y| {
        image::Rgb([x as f32 * 0.25, y as f32 * 0.5, 1.0])
    });
    roundtrip(DynamicImage::ImageRgb32F(buf));
}

#[test]
fn roundtrip_rgba32f() {
    let buf = ImageBuffer::from_fn(2, 2, |x, y| {
        image::Rgba([x as f32 * 0.1, y as f32 * 0.2, 0.5, 1.0])
    });
    roundtrip(DynamicImage::ImageRgba32F(buf));
}

// --- Edge cases ---

#[test]
fn roundtrip_1x1_image() {
    let buf = ImageBuffer::from_pixel(1, 1, Rgba([42, 99, 200, 255]));
    roundtrip(DynamicImage::ImageRgba8(buf));
}

#[test]
fn roundtrip_large_dimension() {
    // Tall narrow image
    let buf = ImageBuffer::from_fn(1, 256, |_, y| {
        Luma([y as u8])
    });
    roundtrip(DynamicImage::ImageLuma8(buf));
}

#[test]
fn roundtrip_all_zeros() {
    let buf = ImageBuffer::from_pixel(4, 4, Rgba([0u8, 0, 0, 0]));
    roundtrip(DynamicImage::ImageRgba8(buf));
}

#[test]
fn roundtrip_all_max_values() {
    let buf = ImageBuffer::from_pixel(2, 2, Rgba([255u8, 255, 255, 255]));
    roundtrip(DynamicImage::ImageRgba8(buf));
}

#[test]
fn roundtrip_f32_special_values() {
    // Test with edge-case float values (zero, very small, very large)
    let buf = ImageBuffer::from_fn(2, 2, |x, y| {
        match (x, y) {
            (0, 0) => image::Rgba([0.0f32, 0.0, 0.0, 0.0]),
            (1, 0) => image::Rgba([f32::MIN_POSITIVE, f32::EPSILON, 1.0, 1.0]),
            (0, 1) => image::Rgba([f32::MAX, f32::MIN, 0.5, 0.5]),
            _ => image::Rgba([1.0, 1.0, 1.0, 1.0]),
        }
    });
    roundtrip(DynamicImage::ImageRgba32F(buf));
}

// --- view_as_u16 tests ---

#[test]
fn view_as_u16_valid() {
    let val: u16 = 0x1234;
    let bytes = val.to_ne_bytes();
    let result = view_as_u16(&bytes).unwrap();
    assert_eq!(result, vec![0x1234]);
}

#[test]
fn view_as_u16_multiple_values() {
    let mut bytes = Vec::new();
    for v in [100u16, 200, 65535] {
        bytes.extend_from_slice(&v.to_ne_bytes());
    }
    let result = view_as_u16(&bytes).unwrap();
    assert_eq!(result, vec![100, 200, 65535]);
}

#[test]
fn view_as_u16_odd_length_returns_none() {
    assert!(view_as_u16(&[1, 2, 3]).is_none());
}

#[test]
fn view_as_u16_single_byte_returns_none() {
    assert!(view_as_u16(&[0xFF]).is_none());
}

#[test]
fn view_as_u16_empty_returns_empty() {
    let result = view_as_u16(&[]).unwrap();
    assert!(result.is_empty());
}

// --- view_as_f32 tests ---

#[test]
fn view_as_f32_valid() {
    let val: f32 = 3.14;
    let bytes = val.to_ne_bytes();
    let result = view_as_f32(&bytes).unwrap();
    assert_eq!(result.len(), 1);
    assert!((result[0] - 3.14).abs() < f32::EPSILON);
}

#[test]
fn view_as_f32_multiple_values() {
    let mut bytes = Vec::new();
    for v in [0.0f32, 1.0, -1.0] {
        bytes.extend_from_slice(&v.to_ne_bytes());
    }
    let result = view_as_f32(&bytes).unwrap();
    assert_eq!(result, vec![0.0, 1.0, -1.0]);
}

#[test]
fn view_as_f32_non_multiple_of_4_returns_none() {
    assert!(view_as_f32(&[1, 2, 3]).is_none());
    assert!(view_as_f32(&[1, 2, 3, 4, 5]).is_none());
    assert!(view_as_f32(&[1]).is_none());
}

#[test]
fn view_as_f32_empty_returns_empty() {
    let result = view_as_f32(&[]).unwrap();
    assert!(result.is_empty());
}

// --- Deserialization error cases ---

#[test]
fn deserialize_unknown_color_type_errors() {
    let json = r#"{"image":{"type":"ImageRgba64","width":1,"height":1,"data":[0,0,0,0]}}"#;
    let result: Result<Wrapper, _> = serde_json::from_str(json);
    assert!(result.is_err());
}

#[test]
fn deserialize_missing_type_field_errors() {
    let json = r#"{"image":{"width":1,"height":1,"data":[0,0,0,0]}}"#;
    let result: Result<Wrapper, _> = serde_json::from_str(json);
    assert!(result.is_err());
}

#[test]
fn deserialize_missing_width_field_errors() {
    let json = r#"{"image":{"type":"ImageRgba8","height":1,"data":[0,0,0,0]}}"#;
    let result: Result<Wrapper, _> = serde_json::from_str(json);
    assert!(result.is_err());
}

#[test]
fn deserialize_missing_data_field_errors() {
    let json = r#"{"image":{"type":"ImageRgba8","width":1,"height":1}}"#;
    let result: Result<Wrapper, _> = serde_json::from_str(json);
    assert!(result.is_err());
}

#[test]
fn deserialize_mismatched_data_length_errors() {
    // RGBA8 for 2x2 needs 16 bytes, only providing 4
    let json = r#"{"image":{"type":"ImageRgba8","width":2,"height":2,"data":[0,0,0,0]}}"#;
    let result: Result<Wrapper, _> = serde_json::from_str(json);
    assert!(result.is_err());
}

#[test]
fn deserialize_u16_odd_byte_count_errors() {
    // Luma16 for 1x1 needs 2 bytes, providing 3
    let json = r#"{"image":{"type":"ImageLuma16","width":1,"height":1,"data":[1,2,3]}}"#;
    let result: Result<Wrapper, _> = serde_json::from_str(json);
    assert!(result.is_err());
}

#[test]
fn deserialize_f32_bad_byte_count_errors() {
    // Rgb32F for 1x1 needs 12 bytes (3 channels * 4 bytes), providing 5
    let json = r#"{"image":{"type":"ImageRgb32F","width":1,"height":1,"data":[1,2,3,4,5]}}"#;
    let result: Result<Wrapper, _> = serde_json::from_str(json);
    assert!(result.is_err());
}

#[test]
fn deserialize_ignores_unknown_fields() {
    // Should still work with extra fields present
    let buf = ImageBuffer::from_pixel(1, 1, Rgba([10, 20, 30, 40]));
    let wrapper = Wrapper { image: Arc::new(DynamicImage::ImageRgba8(buf)) };
    let mut json: serde_json::Value = serde_json::to_value(&wrapper).unwrap();
    json["image"]["extra_field"] = serde_json::Value::String("ignored".into());
    let restored: Wrapper = serde_json::from_value(json).expect("should ignore unknown fields");
    assert_eq!(restored.image.width(), 1);
    assert_eq!(restored.image.height(), 1);
}
