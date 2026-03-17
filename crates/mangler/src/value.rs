use std::path::PathBuf;
use std::sync::Arc;

use image::{imageops::FilterType, DynamicImage, RgbaImage};
use serde::{Deserialize, Serialize};

use crate::{
    color::Color, get_id, operations::images::noise::worley_distance::NoiseWorleyDistanceFunction,
    thumbnail::Thumbnail,
};

pub const THUMBNAIL_SIZE: [u32; 2] = [150, 150];

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Value {
    Bool(bool),
    Integer(i32),
    Decimal(f32),
    String(String),
    Color(Color),
    DynamicImage {
        #[serde(with = "crate::dynamic_image_serde")]
        data: Arc<DynamicImage>,
        change_id: String, // new id each time image changes
    },
    Path(PathBuf),
    #[serde(
        serialize_with = "serialize_filter_type",
        deserialize_with = "deserialize_filter_type"
    )]
    FilterType(FilterType),
    ColorFormat(ColorFormat),

    #[serde(
        serialize_with = "serialize_image_format",
        deserialize_with = "deserialize_image_format"
    )]
    ImageType(image::ImageFormat),
    Trigger,
    NoiseWorleyDistanceFunction(NoiseWorleyDistanceFunction),
    ColorSpace(crate::color::color_spaces::ColorSpace),
    BlendMode(crate::color::blend::BlendMode),
}

pub enum PathType {
    PickFile,
    PickFiles,
    PickFolder,
    PickFolders,
    SaveFile,
}

impl Value {
    pub fn create_thumbnail(&self) -> Option<Thumbnail> {
        match &self {
            Value::Color(color) => {
                let rgb = color.to_srgb_u8();
                let color = image::Rgba([rgb.0, rgb.1, rgb.2, rgb.3]);
                let mut img = RgbaImage::new(THUMBNAIL_SIZE[0], THUMBNAIL_SIZE[1]);
                for x in 0..THUMBNAIL_SIZE[0] {
                    for y in 0..THUMBNAIL_SIZE[1] {
                        img.put_pixel(x, y, color);
                    }
                }

                Some(Thumbnail::Image(img))
            }
            //Value::DynamicImage { data, change_id:_ } => Some(Thumbnail::Image(data.thumbnail(THUMBNAIL_SIZE[0], THUMBNAIL_SIZE[1]).into_rgba8())),
            Value::DynamicImage { data, change_id: _ } => Some(Thumbnail::Image(
                data.thumbnail(THUMBNAIL_SIZE[0], THUMBNAIL_SIZE[1])
                    .to_rgba8(),
            )),
            Value::Bool(value) => Some(Thumbnail::Text(value.to_string())),
            Value::Integer(value) => Some(Thumbnail::Text(value.to_string())),
            Value::Decimal(value) => Some(Thumbnail::Text(format!("{:?}", value))),
            Value::String(value) => Some(Thumbnail::Text(value.clone())),
            Value::Path(path) => Some(Thumbnail::Text(format!(
                "{}",
                path.to_str().unwrap_or("none").to_string()
            ))),
            Value::FilterType(value) => Some(Thumbnail::Text(format!("{:?}", value))),
            Value::ColorFormat(value) => Some(Thumbnail::Text(format!("{:?}", value))),
            Value::Trigger => Some(Thumbnail::Text("trigger".to_string())),
            Value::ImageType(value) => Some(Thumbnail::Text(format!("{:?}", value))),
            Value::NoiseWorleyDistanceFunction(value) => {
                Some(Thumbnail::Text(format!("{:?}", value)))
            }
            Value::ColorSpace(value) => Some(Thumbnail::Text(format!("{:?}", value))),
            Value::BlendMode(value) => Some(Thumbnail::Text(format!("{:?}", value))),
        }
    }

    /// Zero-allocation fingerprint for cache comparison.
    /// Returns a u64 hash that changes when the value changes.
    pub fn fingerprint(&self) -> u64 {
        use std::hash::{Hash, Hasher};
        use std::collections::hash_map::DefaultHasher;
        let mut h = DefaultHasher::new();
        std::mem::discriminant(self).hash(&mut h);
        match self {
            Value::Bool(v) => v.hash(&mut h),
            Value::Integer(v) => v.hash(&mut h),
            Value::Decimal(v) => v.to_bits().hash(&mut h),
            Value::String(v) => v.hash(&mut h),
            Value::Color(c) => { c.r.to_bits().hash(&mut h); c.g.to_bits().hash(&mut h); c.b.to_bits().hash(&mut h); c.a.to_bits().hash(&mut h); },
            Value::DynamicImage { data: _, change_id } => change_id.hash(&mut h),
            Value::Path(p) => p.hash(&mut h),
            Value::FilterType(f) => (*f as u8).hash(&mut h),
            Value::ColorFormat(cf) => (*cf as u8).hash(&mut h),
            Value::ImageType(it) => format!("{:?}", it).hash(&mut h),
            Value::Trigger => 0u8.hash(&mut h), // always same — triggers re-run via is_dirty
            Value::NoiseWorleyDistanceFunction(w) => format!("{:?}", w).hash(&mut h),
            Value::ColorSpace(cs) => format!("{:?}", cs).hash(&mut h),
            Value::BlendMode(bm) => format!("{:?}", bm).hash(&mut h),
        }
        h.finish()
    }

    pub fn value_type(&self) -> ValueType {
        match self {
            Value::Bool(_) => ValueType::Bool,
            Value::Integer(_) => ValueType::Integer,
            Value::Decimal(_) => ValueType::Decimal,
            Value::String(_) => ValueType::String,
            Value::Color(_) => ValueType::Color,
            Value::ColorFormat(_) => ValueType::ColorFormat,
            Value::Trigger => ValueType::Trigger,
            Value::FilterType(_) => ValueType::FilterType,
            Value::Path(_) => ValueType::Path,
            Value::DynamicImage {
                data: _,
                change_id: _,
            } => ValueType::DynamicImage,
            Value::ImageType(_) => ValueType::ImageType,
            Value::NoiseWorleyDistanceFunction(_) => ValueType::NoiseWorleyDistanceFunction,
            Value::ColorSpace(_) => ValueType::ColorSpace,
            Value::BlendMode(_) => ValueType::BlendMode,
        }
    }

    pub fn try_convert_to(&self, other: ValueType) -> Result<Value, ConversionError> {
        match self {
            Value::Bool(a) => match other {
                ValueType::Bool => Ok(Value::Bool(*a)),
                ValueType::Integer => {
                    if *a {
                        Ok(Value::Integer(1))
                    } else {
                        Ok(Value::Integer(0))
                    }
                }
                ValueType::Decimal => {
                    if *a {
                        Ok(Value::Decimal(1.0))
                    } else {
                        Ok(Value::Decimal(0.0))
                    }
                }
                ValueType::String => Ok(Value::String(a.to_string())),
                ValueType::Color => {
                    if *a {
                        Ok(Value::Color(Color::from_srgb_float(1.0, 1.0, 1.0, 1.0)))
                    } else {
                        Ok(Value::Color(Color::from_srgb_float(0.0, 0.0, 0.0, 1.0)))
                    }
                }
                ValueType::DynamicImage => {
                    let mut imgbuf = image::RgbaImage::new(1, 1);
                    let color_value: u8 = if *a { 255 } else { 0 };

                    for (_x, _y, pixel) in imgbuf.enumerate_pixels_mut() {
                        *pixel = image::Rgba([color_value, color_value, color_value, color_value]);
                    }

                    Ok(Value::DynamicImage { data: Arc::new(DynamicImage::ImageRgba8(imgbuf)), change_id: get_id() })
                }
                _ => Err(ConversionError {
                    message: "Unable to convert bool to filter type.".to_string(),
                }),
            },
            Value::Integer(a) => match other {
                ValueType::Bool => Ok(Value::Bool(*a != 0)),
                ValueType::Integer => Ok(Value::Integer(*a)),
                ValueType::Decimal => Ok(Value::Decimal(*a as f32)),
                ValueType::String => Ok(Value::String(a.to_string())),
                _ => Err(ConversionError {
                    message: "Unable to convert bool to filter type.".to_string(),
                }),
            },
            Value::Decimal(a) => match other {
                ValueType::Bool => Ok(Value::Bool(*a != 0.0)),
                ValueType::Integer => Ok(Value::Integer(*a as i32)),
                ValueType::Decimal => Ok(Value::Decimal(*a)),
                ValueType::String => Ok(Value::String(a.to_string())),
                _ => Err(ConversionError {
                    message: "Unable to convert bool to filter type.".to_string(),
                }),
            },
            Value::String(a) => match other {
                ValueType::Bool => {
                    let result: Result<bool, _> = a.parse();
                    match result {
                        Ok(r) => Ok(Value::Bool(r)),
                        Err(_) => Err(ConversionError {
                            message: "Error converting string to bool.".to_string(),
                        }),
                    }
                }
                ValueType::Integer => {
                    let result: Result<i32, _> = a.parse();
                    match result {
                        Ok(r) => Ok(Value::Integer(r)),
                        Err(_) => Err(ConversionError {
                            message: "Error converting string to integer.".to_string(),
                        }),
                    }
                }
                ValueType::Decimal => {
                    let result: Result<f32, _> = a.parse();
                    match result {
                        Ok(r) => Ok(Value::Decimal(r)),
                        Err(_) => Err(ConversionError {
                            message: "Error converting string to decimal.".to_string(),
                        }),
                    }
                }
                ValueType::String => Ok(Value::String(a.clone())),
                _ => Err(ConversionError {
                    message: "Unable to convert bool to filter type.".to_string(),
                }),
            },
            Value::Color(a) => match other {
                ValueType::Color => Ok(Value::Color(*a)),
                _ => Err(ConversionError {
                    message: "Unable to convert integer to image format.".to_string(),
                }),
            },
            Value::FilterType(a) => match other {
                ValueType::FilterType => Ok(Value::FilterType(*a)),
                _ => Err(ConversionError {
                    message: "Unable to convert filter type to bool.".to_string(),
                }),
            },
            Value::ColorFormat(a) => match other {
                ValueType::ColorFormat => Ok(Value::ColorFormat(*a)),
                _ => Err(ConversionError {
                    message: "Unable to convert image type to bool.".to_string(),
                }),
            },
            Value::Trigger => todo!(),
            Value::DynamicImage { data, change_id } => match other {
                ValueType::DynamicImage => Ok(Value::DynamicImage {
                    data: data.clone(),
                    change_id: change_id.clone(),
                }),
                _ => Err(ConversionError {
                    message: "Unable to convert integer to image format.".to_string(),
                }),
            },
            Value::Path(path) => match other {
                ValueType::String => {
                    if let Ok(path_string) = path.clone().into_os_string().into_string() {
                        Ok(Value::String(path_string))
                    } else {
                        Err(ConversionError {
                            message: "Unable to convert integer to image format.".to_string(),
                        })
                    }
                }
                ValueType::Path => Ok(Value::Path(path.clone())),
                _ => Err(ConversionError {
                    message: "Unable to convert integer to image format.".to_string(),
                }),
            },
            Value::ImageType(image_format) => match other {
                ValueType::ImageType => Ok(Value::ImageType(image_format.clone())),
                _ => Err(ConversionError {
                    message: "Unable to convert.".to_string(),
                }),
            },
            Value::NoiseWorleyDistanceFunction(a) => match other {
                ValueType::NoiseWorleyDistanceFunction => {
                    Ok(Value::NoiseWorleyDistanceFunction(a.clone()))
                }
                _ => Err(ConversionError {
                    message: "Unable to convert.".to_string(),
                }),
            },
            Value::ColorSpace(a) => match other {
                ValueType::ColorSpace => Ok(Value::ColorSpace(a.clone())),
                _ => Err(ConversionError {
                    message: "Unable to convert.".to_string(),
                }),
            },
            Value::BlendMode(a) => match other {
                ValueType::BlendMode => Ok(Value::BlendMode(a.clone())),
                _ => Err(ConversionError {
                    message: "Unable to convert.".to_string(),
                }),
            },
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ValueType {
    Bool,
    Integer,
    Decimal,
    String,
    Color,
    FilterType,
    ColorFormat,
    ImageType,
    Trigger,
    DynamicImage,
    Path,
    NoiseWorleyDistanceFunction,
    ColorSpace,
    BlendMode,
}

impl ValueType {
    pub fn types() -> [ValueType; 10] {
        let types: [ValueType; 10] = [
            ValueType::Bool,
            ValueType::Integer,
            ValueType::Decimal,
            ValueType::String,
            ValueType::Color,
            ValueType::FilterType,
            ValueType::ColorFormat,
            ValueType::Trigger,
            ValueType::DynamicImage,
            ValueType::Path,
        ];

        types
    }

    pub fn value_name(&self) -> String {
        match self {
            ValueType::Bool => "bool".to_string(),
            ValueType::Integer => "integer".to_string(),
            ValueType::Decimal => "decimal".to_string(),
            ValueType::String => "string".to_string(),
            ValueType::Color => "color".to_string(),
            ValueType::FilterType => "filter type".to_string(),
            ValueType::ColorFormat => "color format".to_string(),
            ValueType::Trigger => "trigger".to_string(),
            ValueType::DynamicImage => "image".to_string(),
            ValueType::Path => "path".to_string(),
            ValueType::ImageType => "image format".to_string(),
            ValueType::NoiseWorleyDistanceFunction => "worley noise distance function".to_string(),
            ValueType::ColorSpace => "color space".to_string(),
            ValueType::BlendMode => "blend mode".to_string(),
        }
    }

    // file extensions that can be opened for each type
    pub fn file_extensions(value_type: &ValueType) -> Vec<String> {
        match value_type {
            ValueType::DynamicImage => {
                let mut list = vec![];

                for image_format in ImageType::types().iter() {
                    let ext = image_format.format().extensions_str()[0];
                    list.push(ext.to_string());
                }

                list
            }
            _ => vec![],
        }
    }

    pub fn valid_conversions(&self) -> Vec<ValueType> {
        match self {
            ValueType::Bool => vec![
                ValueType::Bool,
                ValueType::Integer,
                ValueType::Decimal,
                ValueType::String,
                ValueType::Trigger,
            ],
            ValueType::Integer => vec![
                ValueType::Bool,
                ValueType::Integer,
                ValueType::Decimal,
                ValueType::String,
                ValueType::Trigger,
            ],
            ValueType::Decimal => vec![
                ValueType::Bool,
                ValueType::Integer,
                ValueType::Decimal,
                ValueType::String,
                ValueType::Trigger,
            ],
            ValueType::String => vec![ValueType::String, ValueType::Trigger],
            ValueType::Color => vec![ValueType::Color, ValueType::Trigger],
            ValueType::DynamicImage => vec![ValueType::DynamicImage, ValueType::Trigger],
            ValueType::Path => vec![ValueType::String, ValueType::Path, ValueType::Trigger],
            ValueType::FilterType => {
                vec![ValueType::FilterType, ValueType::String, ValueType::Trigger]
            }
            ValueType::ColorFormat => vec![
                ValueType::ColorFormat,
                ValueType::String,
                ValueType::Trigger,
            ],
            ValueType::Trigger => vec![ValueType::Trigger],
            ValueType::ImageType => vec![ValueType::ImageType, ValueType::Trigger],
            ValueType::NoiseWorleyDistanceFunction => {
                vec![ValueType::NoiseWorleyDistanceFunction, ValueType::Trigger]
            }
            ValueType::ColorSpace => vec![ValueType::ColorSpace, ValueType::Trigger],
            ValueType::BlendMode => vec![ValueType::BlendMode, ValueType::Trigger],
        }
    }

    pub fn valid_coversions_from(&self) -> Vec<ValueType> {
        let mut types: Vec<ValueType> = Vec::new();

        for value_type in ValueType::types().iter() {
            //if value_type != self {
            if value_type.valid_conversions().contains(&self) {
                types.push(value_type.clone());
            }
            //}
        }

        types
    }
}

#[derive(Debug)]
pub struct ConversionError {
    pub message: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum ColorFormat {
    Rgba32F,
    Rgb32F,
    Rgba16,
    Rgb16,
    GrayA16,
    Gray16,
    Rgba8,
    Rgb8,
    GrayA8,
    Gray8,
}

impl ColorFormat {
    pub fn format(&self) -> image::ColorType {
        match self {
            ColorFormat::Rgba32F => image::ColorType::Rgba32F,
            ColorFormat::Rgb32F => image::ColorType::Rgb32F,
            ColorFormat::Rgba16 => image::ColorType::Rgba16,
            ColorFormat::Rgb16 => image::ColorType::Rgb16,
            ColorFormat::GrayA16 => image::ColorType::La16,
            ColorFormat::Gray16 => image::ColorType::L16,
            ColorFormat::Rgba8 => image::ColorType::Rgba8,
            ColorFormat::Rgb8 => image::ColorType::Rgb8,
            ColorFormat::GrayA8 => image::ColorType::La8,
            ColorFormat::Gray8 => image::ColorType::L8,
        }
    }

    pub fn types() -> [ColorFormat; 10] {
        let types: [ColorFormat; 10] = [
            ColorFormat::Rgba32F,
            ColorFormat::Rgb32F,
            ColorFormat::Rgba16,
            ColorFormat::Rgb16,
            ColorFormat::GrayA16,
            ColorFormat::Gray16,
            ColorFormat::Rgba8,
            ColorFormat::Rgb8,
            ColorFormat::GrayA8,
            ColorFormat::Gray8,
        ];

        types
    }
}

// https://docs.rs/image/latest/src/image/image.rs.html#28-73
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum ImageType {
    Png,
    Jpeg,
    Gif,
    WebP,
    Pnm,
    Tiff,
    Tga,
    //Dds,  // can't read or write
    Bmp,
    Ico,
    Hdr, // can't write
    OpenExr,
    Farbfeld,
    Avif,
    Qoi,
}

impl ImageType {
    pub fn format(&self) -> image::ImageFormat {
        match self {
            ImageType::Png => image::ImageFormat::Png,
            ImageType::Jpeg => image::ImageFormat::Jpeg,
            ImageType::Gif => image::ImageFormat::Gif,
            ImageType::WebP => image::ImageFormat::WebP,
            ImageType::Pnm => image::ImageFormat::Pnm,
            ImageType::Tiff => image::ImageFormat::Tiff,
            ImageType::Tga => image::ImageFormat::Tga,
            ImageType::Bmp => image::ImageFormat::Bmp,
            ImageType::Ico => image::ImageFormat::Ico,
            ImageType::Hdr => image::ImageFormat::Hdr,
            ImageType::OpenExr => image::ImageFormat::OpenExr,
            ImageType::Farbfeld => image::ImageFormat::Farbfeld,
            ImageType::Avif => image::ImageFormat::Avif,
            ImageType::Qoi => image::ImageFormat::Qoi,
        }
    }

    pub fn types() -> [ImageType; 14] {
        let types: [ImageType; 14] = [
            ImageType::Png,
            ImageType::Jpeg,
            ImageType::Gif,
            ImageType::WebP,
            ImageType::Pnm,
            ImageType::Tiff,
            ImageType::Tga,
            ImageType::Bmp,
            ImageType::Ico,
            ImageType::Hdr,
            ImageType::OpenExr,
            ImageType::Farbfeld,
            ImageType::Avif,
            ImageType::Qoi,
        ];

        types
    }
}

#[derive(Debug, Clone)]
pub struct UiButton(bool);

fn serialize_filter_type<S>(value: &FilterType, serializer: S) -> Result<S::Ok, S::Error>
where
    S: serde::Serializer,
{
    let serialized_value = match value {
        FilterType::CatmullRom => "catmullrom",
        FilterType::Gaussian => "guassian",
        FilterType::Lanczos3 => "lanczos3",
        FilterType::Nearest => "nearest",
        FilterType::Triangle => "triangle",
    };
    serializer.serialize_str(serialized_value)
}

fn deserialize_filter_type<'de, D>(deserializer: D) -> Result<FilterType, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let deserialized_value = String::deserialize(deserializer)?;
    match deserialized_value.as_str() {
        "catmullrom" => Ok(FilterType::CatmullRom),
        "guassian" => Ok(FilterType::Gaussian),
        "lanczos3" => Ok(FilterType::Lanczos3),
        "nearest" => Ok(FilterType::Nearest),
        "triangle" => Ok(FilterType::Triangle),
        _ => Err(serde::de::Error::custom("Unknown enum value")),
    }
}

fn serialize_image_format<S>(value: &image::ImageFormat, serializer: S) -> Result<S::Ok, S::Error>
where
    S: serde::Serializer,
{
    let serialized_value = value.extensions_str()[0];
    serializer.serialize_str(serialized_value)
}

fn deserialize_image_format<'de, D>(deserializer: D) -> Result<image::ImageFormat, D::Error>
where
    D: serde::Deserializer<'de>,
{
    if let Ok(s) = String::deserialize(deserializer) {
        if let Some(format) = image::ImageFormat::from_extension(s) {
            Ok(format)
        } else {
            Err(serde::de::Error::custom("Unknown enum value"))
        }
    } else {
        Err(serde::de::Error::custom("Unknown enum value"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    // Helper to match Value variants since Value doesn't impl PartialEq
    macro_rules! assert_value {
        ($val:expr, Bool($expected:expr)) => {
            match &$val { Value::Bool(v) => assert_eq!(*v, $expected), other => panic!("Expected Bool({}), got {:?}", $expected, other) }
        };
        ($val:expr, Integer($expected:expr)) => {
            match &$val { Value::Integer(v) => assert_eq!(*v, $expected), other => panic!("Expected Integer({}), got {:?}", $expected, other) }
        };
        ($val:expr, Decimal($expected:expr)) => {
            match &$val { Value::Decimal(v) => assert!((*v - $expected).abs() < 1e-6, "Expected Decimal({}), got Decimal({})", $expected, v), other => panic!("Expected Decimal({}), got {:?}", $expected, other) }
        };
        ($val:expr, String($expected:expr)) => {
            match &$val { Value::String(v) => assert_eq!(v, $expected), other => panic!("Expected String({}), got {:?}", $expected, other) }
        };
    }

    // value_type tests
    #[test]
    fn test_value_type_bool() {
        assert_eq!(Value::Bool(true).value_type(), ValueType::Bool);
    }

    #[test]
    fn test_value_type_integer() {
        assert_eq!(Value::Integer(42).value_type(), ValueType::Integer);
    }

    #[test]
    fn test_value_type_decimal() {
        assert_eq!(Value::Decimal(3.14).value_type(), ValueType::Decimal);
    }

    #[test]
    fn test_value_type_string() {
        assert_eq!(Value::String("hi".to_string()).value_type(), ValueType::String);
    }

    #[test]
    fn test_value_type_color() {
        assert_eq!(Value::Color(Color::default()).value_type(), ValueType::Color);
    }

    #[test]
    fn test_value_type_path() {
        assert_eq!(Value::Path(PathBuf::new()).value_type(), ValueType::Path);
    }

    #[test]
    fn test_value_type_trigger() {
        assert_eq!(Value::Trigger.value_type(), ValueType::Trigger);
    }

    // try_convert_to: Bool conversions
    #[test]
    fn test_bool_true_to_integer() {
        let result = Value::Bool(true).try_convert_to(ValueType::Integer).unwrap();
        assert_value!(result, Integer(1));
    }

    #[test]
    fn test_bool_false_to_integer() {
        let result = Value::Bool(false).try_convert_to(ValueType::Integer).unwrap();
        assert_value!(result, Integer(0));
    }

    #[test]
    fn test_bool_true_to_decimal() {
        let result = Value::Bool(true).try_convert_to(ValueType::Decimal).unwrap();
        assert_value!(result, Decimal(1.0));
    }

    #[test]
    fn test_bool_false_to_decimal() {
        let result = Value::Bool(false).try_convert_to(ValueType::Decimal).unwrap();
        assert_value!(result, Decimal(0.0));
    }

    #[test]
    fn test_bool_to_string() {
        let result = Value::Bool(true).try_convert_to(ValueType::String).unwrap();
        assert_value!(result, String("true"));
    }

    #[test]
    fn test_bool_to_bool_identity() {
        let result = Value::Bool(true).try_convert_to(ValueType::Bool).unwrap();
        assert_value!(result, Bool(true));
    }

    #[test]
    fn test_bool_to_color_true() {
        let result = Value::Bool(true).try_convert_to(ValueType::Color).unwrap();
        match result {
            Value::Color(c) => {
                assert_eq!(c.r, 1.0);
                assert_eq!(c.g, 1.0);
                assert_eq!(c.b, 1.0);
            },
            other => panic!("Expected Color, got {:?}", other),
        }
    }

    #[test]
    fn test_bool_to_color_false() {
        let result = Value::Bool(false).try_convert_to(ValueType::Color).unwrap();
        match result {
            Value::Color(c) => {
                assert_eq!(c.r, 0.0);
                assert_eq!(c.g, 0.0);
                assert_eq!(c.b, 0.0);
            },
            other => panic!("Expected Color, got {:?}", other),
        }
    }

    #[test]
    fn test_bool_to_dynamic_image() {
        let result = Value::Bool(true).try_convert_to(ValueType::DynamicImage);
        assert!(result.is_ok());
        match result.unwrap() {
            Value::DynamicImage { data: _, change_id: _ } => {},
            other => panic!("Expected DynamicImage, got {:?}", other),
        }
    }

    #[test]
    fn test_bool_to_filter_type_fails() {
        let result = Value::Bool(true).try_convert_to(ValueType::FilterType);
        assert!(result.is_err());
    }

    // try_convert_to: Integer conversions
    #[test]
    fn test_integer_to_bool_nonzero() {
        let result = Value::Integer(42).try_convert_to(ValueType::Bool).unwrap();
        assert_value!(result, Bool(true));
    }

    #[test]
    fn test_integer_to_bool_zero() {
        let result = Value::Integer(0).try_convert_to(ValueType::Bool).unwrap();
        assert_value!(result, Bool(false));
    }

    #[test]
    fn test_integer_to_decimal() {
        let result = Value::Integer(42).try_convert_to(ValueType::Decimal).unwrap();
        assert_value!(result, Decimal(42.0));
    }

    #[test]
    fn test_integer_to_string() {
        let result = Value::Integer(42).try_convert_to(ValueType::String).unwrap();
        assert_value!(result, String("42"));
    }

    #[test]
    fn test_integer_to_integer_identity() {
        let result = Value::Integer(42).try_convert_to(ValueType::Integer).unwrap();
        assert_value!(result, Integer(42));
    }

    #[test]
    fn test_integer_to_color_fails() {
        let result = Value::Integer(42).try_convert_to(ValueType::Color);
        assert!(result.is_err());
    }

    // try_convert_to: Decimal conversions
    #[test]
    fn test_decimal_to_bool_nonzero() {
        let result = Value::Decimal(3.14).try_convert_to(ValueType::Bool).unwrap();
        assert_value!(result, Bool(true));
    }

    #[test]
    fn test_decimal_to_bool_zero() {
        let result = Value::Decimal(0.0).try_convert_to(ValueType::Bool).unwrap();
        assert_value!(result, Bool(false));
    }

    #[test]
    fn test_decimal_to_integer() {
        let result = Value::Decimal(3.14).try_convert_to(ValueType::Integer).unwrap();
        assert_value!(result, Integer(3));
    }

    #[test]
    fn test_decimal_to_string() {
        let result = Value::Decimal(3.14).try_convert_to(ValueType::String).unwrap();
        match result {
            Value::String(_) => {},
            other => panic!("Expected String, got {:?}", other),
        }
    }

    #[test]
    fn test_decimal_to_decimal_identity() {
        let result = Value::Decimal(3.14).try_convert_to(ValueType::Decimal).unwrap();
        assert_value!(result, Decimal(3.14));
    }

    // try_convert_to: String conversions
    #[test]
    fn test_string_to_bool_true() {
        let result = Value::String("true".to_string()).try_convert_to(ValueType::Bool).unwrap();
        assert_value!(result, Bool(true));
    }

    #[test]
    fn test_string_to_bool_false() {
        let result = Value::String("false".to_string()).try_convert_to(ValueType::Bool).unwrap();
        assert_value!(result, Bool(false));
    }

    #[test]
    fn test_string_to_bool_invalid() {
        let result = Value::String("not a bool".to_string()).try_convert_to(ValueType::Bool);
        assert!(result.is_err());
    }

    #[test]
    fn test_string_to_integer() {
        let result = Value::String("42".to_string()).try_convert_to(ValueType::Integer).unwrap();
        assert_value!(result, Integer(42));
    }

    #[test]
    fn test_string_to_integer_invalid() {
        let result = Value::String("abc".to_string()).try_convert_to(ValueType::Integer);
        assert!(result.is_err());
    }

    #[test]
    fn test_string_to_decimal() {
        let result = Value::String("3.14".to_string()).try_convert_to(ValueType::Decimal).unwrap();
        assert_value!(result, Decimal(3.14));
    }

    #[test]
    fn test_string_to_decimal_invalid() {
        let result = Value::String("abc".to_string()).try_convert_to(ValueType::Decimal);
        assert!(result.is_err());
    }

    #[test]
    fn test_string_to_string_identity() {
        let result = Value::String("hello".to_string()).try_convert_to(ValueType::String).unwrap();
        assert_value!(result, String("hello"));
    }

    // try_convert_to: Other types
    #[test]
    fn test_color_to_color_identity() {
        let color = Color::from_srgb_float(0.5, 0.3, 0.7, 1.0);
        let result = Value::Color(color).try_convert_to(ValueType::Color).unwrap();
        match result {
            Value::Color(c) => assert_eq!(c, color),
            other => panic!("Expected Color, got {:?}", other),
        }
    }

    #[test]
    fn test_color_to_integer_fails() {
        let result = Value::Color(Color::default()).try_convert_to(ValueType::Integer);
        assert!(result.is_err());
    }

    #[test]
    fn test_path_to_string() {
        let result = Value::Path(PathBuf::from("/test/path")).try_convert_to(ValueType::String).unwrap();
        match result {
            Value::String(s) => assert!(s.contains("test")),
            other => panic!("Expected String, got {:?}", other),
        }
    }

    #[test]
    fn test_path_to_path_identity() {
        let result = Value::Path(PathBuf::from("/test")).try_convert_to(ValueType::Path).unwrap();
        match result {
            Value::Path(p) => assert_eq!(p, PathBuf::from("/test")),
            other => panic!("Expected Path, got {:?}", other),
        }
    }

    // valid_conversions tests
    #[test]
    fn test_bool_valid_conversions() {
        let conversions = ValueType::Bool.valid_conversions();
        assert!(conversions.contains(&ValueType::Bool));
        assert!(conversions.contains(&ValueType::Integer));
        assert!(conversions.contains(&ValueType::Decimal));
        assert!(conversions.contains(&ValueType::String));
        assert!(conversions.contains(&ValueType::Trigger));
    }

    #[test]
    fn test_dynamic_image_valid_conversions() {
        let conversions = ValueType::DynamicImage.valid_conversions();
        assert!(conversions.contains(&ValueType::DynamicImage));
        assert!(conversions.contains(&ValueType::Trigger));
        assert!(!conversions.contains(&ValueType::Integer));
    }

    #[test]
    fn test_integer_valid_conversions() {
        let conversions = ValueType::Integer.valid_conversions();
        assert!(conversions.contains(&ValueType::Bool));
        assert!(conversions.contains(&ValueType::Integer));
        assert!(conversions.contains(&ValueType::Decimal));
        assert!(conversions.contains(&ValueType::String));
    }

    #[test]
    fn test_value_type_name() {
        assert_eq!(ValueType::Bool.value_name(), "bool");
        assert_eq!(ValueType::Integer.value_name(), "integer");
        assert_eq!(ValueType::Decimal.value_name(), "decimal");
        assert_eq!(ValueType::String.value_name(), "string");
        assert_eq!(ValueType::Color.value_name(), "color");
        assert_eq!(ValueType::DynamicImage.value_name(), "image");
        assert_eq!(ValueType::Path.value_name(), "path");
    }
}
