//! Custom serde serialization for `Arc<DynamicImage>`.
//!
//! The `image` crate's `DynamicImage` doesn't implement Serialize/Deserialize,
//! so we provide a custom format that stores the color type tag, dimensions,
//! and raw pixel bytes. Used via `#[serde(serialize_with, deserialize_with)]`
//! on fields that hold an `Arc<DynamicImage>` (e.g. in `Value::DynamicImage`).
//!
//! Serialized format (as a struct with 4 fields):
//!   - `type`:   color type string (e.g. "ImageRgba8", "ImageRgb32F")
//!   - `width`:  image width in pixels
//!   - `height`: image height in pixels
//!   - `data`:   raw pixel bytes (u8 slice, regardless of underlying channel type)

use image::{ColorType, DynamicImage, ImageBuffer};
use serde::de::{MapAccess, Visitor};
use serde::ser::SerializeStruct;
use serde::{Deserializer, Serializer};
use std::fmt::Formatter;
use std::sync::Arc;

/// Serializes an `Arc<DynamicImage>` as a struct with type, width, height, and raw byte data.
/// Maps the image's `ColorType` to a string tag so deserialization can reconstruct
/// the correct `DynamicImage` variant.
pub fn serialize<S>(data: &Arc<DynamicImage>, serializer: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    // Map ColorType enum to a stable string tag for the serialized format
    let color_type_str = match data.color() {
        ColorType::L8 => "ImageLuma8",
        ColorType::La8 => "ImageLumaA8",
        ColorType::Rgb8 => "ImageRgb8",
        ColorType::Rgba8 => "ImageRgba8",
        ColorType::L16 => "ImageLuma16",
        ColorType::La16 => "ImageLumaA16",
        ColorType::Rgb16 => "ImageRgb16",
        ColorType::Rgba16 => "ImageRgba16",
        ColorType::Rgb32F => "ImageRgb32F",
        ColorType::Rgba32F => "ImageRgba32F",
        _ => return Err(serde::ser::Error::custom("Unsupported color type")),
    };
    let mut ser = serializer.serialize_struct("DynamicImage", 4)?;
    ser.serialize_field("type", color_type_str)?;
    ser.serialize_field("width", &data.width())?;
    ser.serialize_field("height", &data.height())?;
    ser.serialize_field("data", data.as_bytes())?;
    ser.end()
}

/// Deserializes an `Arc<DynamicImage>` from the struct format produced by `serialize`.
/// Delegates to `DynamicImageVisitor` which reads the fields in any order and
/// reconstructs the correct `DynamicImage` variant from the type tag.
pub fn deserialize<'de, D>(deserializer: D) -> Result<Arc<DynamicImage>, D::Error>
where
    D: Deserializer<'de>,
{
    deserializer
        .deserialize_struct(
            "DynamicImage",
            &["type", "width", "height", "data"],
            DynamicImageVisitor,
        )
        .map(Arc::new)
}

/// Serde visitor that reads the DynamicImage struct fields from any map-like
/// format. Fields can appear in any order; unknown fields are silently skipped.
struct DynamicImageVisitor;

impl<'de> Visitor<'de> for DynamicImageVisitor {
    type Value = DynamicImage;

    fn expecting(&self, formatter: &mut Formatter) -> std::fmt::Result {
        formatter.write_str("struct DynamicImage")
    }

    fn visit_map<A>(self, mut map: A) -> Result<Self::Value, A::Error>
    where
        A: MapAccess<'de>,
    {
        let mut color_type: Option<String> = None;
        let mut width: Option<u32> = None;
        let mut height: Option<u32> = None;
        let mut data: Option<Vec<u8>> = None;

        // Extract known fields, skip unknown ones
        while let Some(key) = map.next_key::<String>()? {
            match key.as_str() {
                "type" => color_type = Some(map.next_value()?),
                "width" => width = Some(map.next_value()?),
                "height" => height = Some(map.next_value()?),
                "data" => data = Some(map.next_value()?),
                _ => { let _ = map.next_value::<serde::de::IgnoredAny>()?; }
            }
        }

        // All four fields are required
        let color_type = color_type.ok_or_else(|| serde::de::Error::missing_field("type"))?;
        let width = width.ok_or_else(|| serde::de::Error::missing_field("width"))?;
        let height = height.ok_or_else(|| serde::de::Error::missing_field("height"))?;
        let data = data.ok_or_else(|| serde::de::Error::missing_field("data"))?;

        // Reconstruct the DynamicImage variant from the type tag.
        // 8-bit types use the raw bytes directly.
        // 16-bit types reinterpret bytes as u16 via view_as_u16.
        // 32-bit float types reinterpret bytes as f32 via view_as_f32.
        // ImageBuffer::from_raw returns None if data length doesn't match width*height*channels.
        match color_type.as_str() {
            "ImageLuma8" => ImageBuffer::from_raw(width, height, data)
                .map(DynamicImage::ImageLuma8)
                .ok_or_else(|| serde::de::Error::custom("Invalid luma8 image data")),
            "ImageLumaA8" => ImageBuffer::from_raw(width, height, data)
                .map(DynamicImage::ImageLumaA8)
                .ok_or_else(|| serde::de::Error::custom("Invalid lumaA8 image data")),
            "ImageRgb8" => ImageBuffer::from_raw(width, height, data)
                .map(DynamicImage::ImageRgb8)
                .ok_or_else(|| serde::de::Error::custom("Invalid rgb8 image data")),
            "ImageRgba8" => ImageBuffer::from_raw(width, height, data)
                .map(DynamicImage::ImageRgba8)
                .ok_or_else(|| serde::de::Error::custom("Invalid rgba8 image data")),
            "ImageLuma16" => view_as_u16(&data)
                .and_then(|v| ImageBuffer::from_raw(width, height, v))
                .map(DynamicImage::ImageLuma16)
                .ok_or_else(|| serde::de::Error::custom("Invalid luma16 image data")),
            "ImageLumaA16" => view_as_u16(&data)
                .and_then(|v| ImageBuffer::from_raw(width, height, v))
                .map(DynamicImage::ImageLumaA16)
                .ok_or_else(|| serde::de::Error::custom("Invalid lumaA16 image data")),
            "ImageRgb16" => view_as_u16(&data)
                .and_then(|v| ImageBuffer::from_raw(width, height, v))
                .map(DynamicImage::ImageRgb16)
                .ok_or_else(|| serde::de::Error::custom("Invalid rgb16 image data")),
            "ImageRgba16" => view_as_u16(&data)
                .and_then(|v| ImageBuffer::from_raw(width, height, v))
                .map(DynamicImage::ImageRgba16)
                .ok_or_else(|| serde::de::Error::custom("Invalid rgba16 image data")),
            "ImageRgb32F" => view_as_f32(&data)
                .and_then(|v| ImageBuffer::from_raw(width, height, v))
                .map(DynamicImage::ImageRgb32F)
                .ok_or_else(|| serde::de::Error::custom("Invalid rgb32f image data")),
            "ImageRgba32F" => view_as_f32(&data)
                .and_then(|v| ImageBuffer::from_raw(width, height, v))
                .map(DynamicImage::ImageRgba32F)
                .ok_or_else(|| serde::de::Error::custom("Invalid rgba32f image data")),
            _ => Err(serde::de::Error::custom(format!(
                "Unsupported color type: {}",
                color_type
            ))),
        }
    }
}

/// Reinterprets a byte slice as a vec of `u16` values using native endianness.
/// Returns `None` if the byte count is odd (not a valid u16 sequence).
fn view_as_u16(data: &[u8]) -> Option<Vec<u16>> {
    if data.len() % 2 != 0 {
        return None;
    }
    Some(
        data.chunks_exact(2)
            .map(|a| u16::from_ne_bytes([a[0], a[1]]))
            .collect(),
    )
}

/// Reinterprets a byte slice as a vec of `f32` values using native endianness.
/// Returns `None` if the byte count is not a multiple of 4.
fn view_as_f32(data: &[u8]) -> Option<Vec<f32>> {
    if data.len() % 4 != 0 {
        return None;
    }
    Some(
        data.chunks_exact(4)
            .map(|a| f32::from_ne_bytes([a[0], a[1], a[2], a[3]]))
            .collect(),
    )
}

#[cfg(test)]
mod tests {
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
}
