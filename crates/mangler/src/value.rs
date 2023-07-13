use std::path::PathBuf;

use image::{imageops::FilterType, DynamicImage, RgbaImage};
use serde::{Deserialize, Serialize};

use crate::{thumbnail::Thumbnail, get_id, color::Color, operations::images::noise::worley_distance::NoiseWorleyDistanceFunction};

pub const THUMBNAIL_SIZE: [u32; 2] = [150, 150];

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Value {
    Bool(bool),
    Integer(i32),
    Decimal(f32),
    String(String),
    Color(Color),
    DynamicImage {
        data: DynamicImage,
        change_id: String,  // new id each time image changes
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

                //Some(Thumbnail::Image(DynamicImage::ImageRgba8(img).thumbnail(THUMBNAIL_SIZE[0], THUMBNAIL_SIZE[1]).to_rgba8()))
            },
            Value::DynamicImage { data, change_id:_ } => Some(Thumbnail::Image(data.thumbnail(THUMBNAIL_SIZE[0], THUMBNAIL_SIZE[1]).to_rgba8())),
            Value::Bool(value) => Some(Thumbnail::Text(value.to_string())),
            Value::Integer(value) => Some(Thumbnail::Text(value.to_string())),
            Value::Decimal(value) => Some(Thumbnail::Text(format!("{:?}", value))),
            Value::String(value) => Some(Thumbnail::Text(value.clone())),
            Value::Path(path) => Some(Thumbnail::Text(format!("{}", path.to_str().unwrap_or("none").to_string()))),
            Value::FilterType(value) => Some(Thumbnail::Text(format!("{:?}", value))),
            Value::ColorFormat(value) => Some(Thumbnail::Text(format!("{:?}", value))),
            Value::Trigger => Some(Thumbnail::Text("trigger".to_string())),
            Value::ImageType(value) => Some(Thumbnail::Text(format!("{:?}", value))),
            Value::NoiseWorleyDistanceFunction(value) => Some(Thumbnail::Text(format!("{:?}", value))),
            Value::ColorSpace(value) => Some(Thumbnail::Text(format!("{:?}", value))),
        }
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
            Value::DynamicImage { data:_, change_id:_ } => ValueType::DynamicImage,
            Value::ImageType(_) => ValueType::ImageType,
            Value::NoiseWorleyDistanceFunction(_) => ValueType::NoiseWorleyDistanceFunction,
            Value::ColorSpace(_) => ValueType::ColorSpace,
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
            }
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
            Value::DynamicImage { data, change_id:_ } => match other {
                ValueType::DynamicImage => Ok(Value::DynamicImage{ data: data.clone(), change_id: get_id() }),
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
                },
                ValueType::Path => {
                    Ok(Value::Path(path.clone()))
                },
                _ => Err(ConversionError {
                    message: "Unable to convert integer to image format.".to_string(),
                }),
            }
            Value::ImageType(image_format) => match other {
                ValueType::ImageType => {
                    Ok(Value::ImageType(image_format.clone()))
                },
                _ => Err(ConversionError {
                    message: "Unable to convert.".to_string(),
                }),
            },
            Value::NoiseWorleyDistanceFunction(a) => match other {
                ValueType::NoiseWorleyDistanceFunction => Ok(Value::NoiseWorleyDistanceFunction(a.clone())),
                _ => Err(ConversionError { message: "Unable to convert.".to_string() })
            },
            Value::ColorSpace(a) => match other {
                ValueType::ColorSpace => Ok(Value::ColorSpace(a.clone())),
                _ => Err(ConversionError { message: "Unable to convert.".to_string() })
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
            },
            _ => vec![],
        }
    }

    pub fn valid_conversions(&self) -> Vec<ValueType> {
        match self {
            ValueType::Bool => vec![ValueType::Bool, ValueType::Integer, ValueType::Decimal, ValueType::String, ValueType::Trigger],
            ValueType::Integer => vec![ValueType::Bool, ValueType::Integer, ValueType::Decimal, ValueType::String, ValueType::Trigger],
            ValueType::Decimal => vec![ValueType::Bool, ValueType::Integer, ValueType::Decimal, ValueType::String, ValueType::Trigger],
            ValueType::String => vec![ValueType::String, ValueType::Trigger],
            ValueType::Color => vec![ValueType::Color, ValueType::Trigger],
            ValueType::DynamicImage => vec![ValueType::DynamicImage, ValueType::Trigger],
            ValueType::Path => vec![ValueType::String, ValueType::Path, ValueType::Trigger],
            ValueType::FilterType => vec![ValueType::FilterType, ValueType::String, ValueType::Trigger],
            ValueType::ColorFormat => vec![ValueType::ColorFormat, ValueType::String, ValueType::Trigger],
            ValueType::Trigger => vec![ValueType::Trigger],
            ValueType::ImageType => vec![ValueType::ImageType, ValueType::Trigger],
            ValueType::NoiseWorleyDistanceFunction => vec![ValueType::NoiseWorleyDistanceFunction, ValueType::Trigger],
            ValueType::ColorSpace => vec![ValueType::ColorSpace, ValueType::Trigger],
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
    Hdr,    // can't write
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
where S: serde::Serializer {
    let serialized_value = image::ImageFormat::Jpeg.extensions_str()[0];
    serializer.serialize_str(serialized_value)
}

fn deserialize_image_format<'de, D>(deserializer: D) -> Result<image::ImageFormat, D::Error>
where D: serde::Deserializer<'de> {
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
