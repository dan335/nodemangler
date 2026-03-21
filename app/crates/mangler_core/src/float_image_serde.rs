//! Custom serde serialization for `Arc<FloatImage>`.
//!
//! Serialized format (as a struct with 4 fields):
//!   - `channels`: number of channels per pixel (1–4)
//!   - `width`:    image width in pixels
//!   - `height`:   image height in pixels
//!   - `data`:     raw pixel bytes (f32 values as native-endian bytes)

use crate::float_image::FloatImage;
use serde::de::{MapAccess, Visitor};
use serde::ser::SerializeStruct;
use serde::{Deserializer, Serializer};
use std::fmt::Formatter;
use std::sync::Arc;

/// Serializes an `Arc<FloatImage>` as a struct with channels, width, height, and raw byte data.
pub fn serialize<S>(data: &Arc<FloatImage>, serializer: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    let raw = data.as_raw();
    // Convert f32 slice to byte vec for serialization
    let bytes: Vec<u8> = raw.iter()
        .flat_map(|f| f.to_ne_bytes())
        .collect();

    let mut ser = serializer.serialize_struct("FloatImage", 4)?;
    ser.serialize_field("channels", &data.channels())?;
    ser.serialize_field("width", &data.width())?;
    ser.serialize_field("height", &data.height())?;
    ser.serialize_field("data", &bytes)?;
    ser.end()
}

/// Deserializes an `Arc<FloatImage>` from the struct format produced by `serialize`.
pub fn deserialize<'de, D>(deserializer: D) -> Result<Arc<FloatImage>, D::Error>
where
    D: Deserializer<'de>,
{
    deserializer
        .deserialize_struct(
            "FloatImage",
            &["channels", "width", "height", "data"],
            FloatImageVisitor,
        )
        .map(Arc::new)
}

/// Serde visitor that reads the FloatImage struct fields from any map-like format.
struct FloatImageVisitor;

impl<'de> Visitor<'de> for FloatImageVisitor {
    type Value = FloatImage;

    fn expecting(&self, formatter: &mut Formatter) -> std::fmt::Result {
        formatter.write_str("struct FloatImage with channels, width, height, and data")
    }

    fn visit_map<A>(self, mut map: A) -> Result<Self::Value, A::Error>
    where
        A: MapAccess<'de>,
    {
        let mut channels: Option<u32> = None;
        let mut width: Option<u32> = None;
        let mut height: Option<u32> = None;
        let mut data: Option<Vec<u8>> = None;

        // Extract known fields, skip unknown ones
        while let Some(key) = map.next_key::<String>()? {
            match key.as_str() {
                "channels" => channels = Some(map.next_value()?),
                "width" => width = Some(map.next_value()?),
                "height" => height = Some(map.next_value()?),
                "data" => data = Some(map.next_value()?),
                _ => { let _ = map.next_value::<serde::de::IgnoredAny>()?; }
            }
        }

        // All four fields are required
        let channels = channels.ok_or_else(|| serde::de::Error::missing_field("channels"))?;
        let width = width.ok_or_else(|| serde::de::Error::missing_field("width"))?;
        let height = height.ok_or_else(|| serde::de::Error::missing_field("height"))?;
        let data = data.ok_or_else(|| serde::de::Error::missing_field("data"))?;

        // Validate channel count
        if !(1..=4).contains(&channels) {
            return Err(serde::de::Error::custom(format!(
                "channels must be 1–4, got {}", channels
            )));
        }

        // Reinterpret bytes as f32 values
        let float_data = view_as_f32(&data).ok_or_else(|| {
            serde::de::Error::custom("data length is not a multiple of 4 bytes")
        })?;

        FloatImage::from_raw(width, height, channels, float_data).ok_or_else(|| {
            serde::de::Error::custom(format!(
                "data length {} does not match {}x{}x{} = {}",
                data.len() / 4,
                width, height, channels,
                width * height * channels
            ))
        })
    }
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
#[path = "float_image_serde_tests.rs"]
mod tests;
