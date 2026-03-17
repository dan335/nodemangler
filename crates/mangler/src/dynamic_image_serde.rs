use image::{ColorType, DynamicImage, ImageBuffer};
use serde::de::{MapAccess, Visitor};
use serde::ser::SerializeStruct;
use serde::{Deserializer, Serializer};
use std::fmt::Formatter;
use std::sync::Arc;

pub fn serialize<S>(data: &Arc<DynamicImage>, serializer: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
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

        while let Some(key) = map.next_key::<String>()? {
            match key.as_str() {
                "type" => color_type = Some(map.next_value()?),
                "width" => width = Some(map.next_value()?),
                "height" => height = Some(map.next_value()?),
                "data" => data = Some(map.next_value()?),
                _ => { let _ = map.next_value::<serde::de::IgnoredAny>()?; }
            }
        }

        let color_type = color_type.ok_or_else(|| serde::de::Error::missing_field("type"))?;
        let width = width.ok_or_else(|| serde::de::Error::missing_field("width"))?;
        let height = height.ok_or_else(|| serde::de::Error::missing_field("height"))?;
        let data = data.ok_or_else(|| serde::de::Error::missing_field("data"))?;

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
