use std::path::PathBuf;

use image::{imageops::FilterType, DynamicImage};
use serde::{Deserialize, Serialize};

use crate::{thumbnail::Thumbnail, get_id};

pub const THUMBNAIL_SIZE: [u32; 2] = [150, 150];

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum Value {
    Bool(bool),
    Integer(i32),
    Decimal(f32),
    String(String),
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
    ImageFormat(image::ImageFormat),
    Trigger,
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
            Value::DynamicImage { data, change_id:_ } => Some(Thumbnail::Image(data.thumbnail(THUMBNAIL_SIZE[0], THUMBNAIL_SIZE[1]).to_rgba8())),
            Value::Bool(value) => Some(Thumbnail::Text(value.to_string())),
            Value::Integer(value) => Some(Thumbnail::Text(value.to_string())),
            Value::Decimal(value) => Some(Thumbnail::Text(value.to_string())),
            Value::String(value) => Some(Thumbnail::Text(value.clone())),
            Value::Path(path) => Some(Thumbnail::Text(format!("{}", path.to_str().unwrap_or("none").to_string()))),
            Value::FilterType(value) => Some(Thumbnail::Text(format!("{:?}", value))),
            Value::ColorFormat(value) => Some(Thumbnail::Text(format!("{:?}", value))),
            Value::Trigger => Some(Thumbnail::Text("trigger".to_string())),
            Value::ImageFormat(value) => Some(Thumbnail::Text(format!("{:?}", value))),
        }
    }

    pub fn value_type(&self) -> ValueType {
        match self {
            Value::Bool(_) => ValueType::Bool,
            Value::Integer(_) => ValueType::Integer,
            Value::Decimal(_) => ValueType::Decimal,
            Value::String(_) => ValueType::String,
            Value::ColorFormat(_) => ValueType::ColorFormat,
            Value::Trigger => ValueType::Trigger,
            Value::FilterType(_) => ValueType::FilterType,
            Value::Path(_) => ValueType::Path,
            Value::DynamicImage { data:_, change_id:_ } => ValueType::DynamicImage,
            Value::ImageFormat(_) => ValueType::ImageFormat,
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
                ValueType::FilterType => Err(ConversionError {
                    message: "Unable to convert bool to filter type.".to_string(),
                }),
                ValueType::ColorFormat => Err(ConversionError {
                    message: "Unable to convert bool to image format.".to_string(),
                }),
                ValueType::Trigger => Ok(Value::Bool(*a)),
                ValueType::DynamicImage => Err(ConversionError {
                    message: "Unable to convert bool to image format.".to_string(),
                }),
                ValueType::Path => Err(ConversionError {
                    message: "Unable to convert bool to image format.".to_string(),
                }),
                ValueType::ImageFormat => Err(ConversionError {
                    message: "Unable to convert bool to image format.".to_string(),
                }),
            },
            Value::Integer(a) => match other {
                ValueType::Bool => Ok(Value::Bool(*a != 0)),
                ValueType::Integer => Ok(Value::Integer(*a)),
                ValueType::Decimal => Ok(Value::Decimal(*a as f32)),
                ValueType::String => Ok(Value::String(a.to_string())),
                ValueType::FilterType => Err(ConversionError {
                    message: "Unable to convert bool to filter type.".to_string(),
                }),
                ValueType::ColorFormat => Err(ConversionError {
                    message: "Unable to convert integer to image format.".to_string(),
                }),
                ValueType::Trigger => Err(ConversionError {
                    message: "Unable to convert integer to image format.".to_string(),
                }),
                ValueType::DynamicImage => Err(ConversionError {
                    message: "Unable to convert integer to image format.".to_string(),
                }),
                ValueType::Path => Err(ConversionError {
                    message: "Unable to convert integer to image format.".to_string(),
                }),
                ValueType::ImageFormat => Err(ConversionError {
                    message: "Unable to convert integer to image format.".to_string(),
                }),
            },
            Value::Decimal(a) => match other {
                ValueType::Bool => Ok(Value::Bool(*a != 0.0)),
                ValueType::Integer => Ok(Value::Integer(*a as i32)),
                ValueType::Decimal => Ok(Value::Decimal(*a)),
                ValueType::String => Ok(Value::String(a.to_string())),
                ValueType::FilterType => Err(ConversionError {
                    message: "Unable to convert bool to filter type.".to_string(),
                }),
                ValueType::ColorFormat => Err(ConversionError {
                    message: "Unable to convert decimal to image format.".to_string(),
                }),
                ValueType::Trigger => Err(ConversionError {
                    message: "Unable to convert integer to image format.".to_string(),
                }),
                ValueType::DynamicImage => Err(ConversionError {
                    message: "Unable to convert integer to image format.".to_string(),
                }),
                ValueType::Path => Err(ConversionError {
                    message: "Unable to convert integer to image format.".to_string(),
                }),
                ValueType::ImageFormat => Err(ConversionError {
                    message: "Unable to convert integer to image format.".to_string(),
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
                ValueType::FilterType => Err(ConversionError {
                    message: "Unable to convert bool to filter type.".to_string(),
                }),
                ValueType::ColorFormat => Err(ConversionError {
                    message: "Unable to convert string to image format.".to_string(),
                }),
                ValueType::Trigger => Err(ConversionError {
                    message: "Unable to convert integer to image format.".to_string(),
                }),
                ValueType::DynamicImage => Err(ConversionError {
                    message: "Unable to convert integer to image format.".to_string(),
                }),
                ValueType::Path => Err(ConversionError {
                    message: "Unable to convert integer to image format.".to_string(),
                }),
                ValueType::ImageFormat => Err(ConversionError {
                    message: "Unable to convert integer to image format.".to_string(),
                }),
            },
            Value::FilterType(a) => match other {
                ValueType::Bool => Err(ConversionError {
                    message: "Unable to convert filter type to bool.".to_string(),
                }),
                ValueType::Integer => Err(ConversionError {
                    message: "Unable to convert filter type to integer.".to_string(),
                }),
                ValueType::Decimal => Err(ConversionError {
                    message: "Unable to convert filter type to decimal.".to_string(),
                }),
                ValueType::String => Err(ConversionError {
                    message: "Unable to convert filter type to string.".to_string(),
                }),
                ValueType::FilterType => Ok(Value::FilterType(*a)),
                ValueType::ColorFormat => Err(ConversionError {
                    message: "Unable to convert filter type to image format.".to_string(),
                }),
                ValueType::Trigger => Err(ConversionError {
                    message: "Unable to convert integer to image format.".to_string(),
                }),
                ValueType::DynamicImage => Err(ConversionError {
                    message: "Unable to convert integer to image format.".to_string(),
                }),
                ValueType::Path => Err(ConversionError {
                    message: "Unable to convert integer to image format.".to_string(),
                }),
                ValueType::ImageFormat => Err(ConversionError {
                    message: "Unable to convert integer to image format.".to_string(),
                }),
            },
            Value::ColorFormat(a) => match other {
                ValueType::Bool => Err(ConversionError {
                    message: "Unable to convert image type to bool.".to_string(),
                }),
                ValueType::Integer => Err(ConversionError {
                    message: "Unable to convert image type to integer.".to_string(),
                }),
                ValueType::Decimal => Err(ConversionError {
                    message: "Unable to convert image type to decimal.".to_string(),
                }),
                ValueType::String => Err(ConversionError {
                    message: "Unable to convert image type to string.".to_string(),
                }),
                ValueType::FilterType => Err(ConversionError {
                    message: "Unable to convert image type to image.".to_string(),
                }),
                ValueType::ColorFormat => Ok(Value::ColorFormat(*a)),
                ValueType::Trigger => todo!(),
                ValueType::DynamicImage => Err(ConversionError {
                    message: "Unable to convert image type to image.".to_string(),
                }),
                ValueType::Path => Err(ConversionError {
                    message: "Unable to convert image type to image.".to_string(),
                }),
                ValueType::ImageFormat => Err(ConversionError {
                    message: "Unable to convert image type to image.".to_string(),
                }),
            },
            Value::Trigger => todo!(),
            Value::DynamicImage { data, change_id } => match other {
                ValueType::Bool => Err(ConversionError {
                    message: "Unable to convert integer to image format.".to_string(),
                }),
                ValueType::Integer => Err(ConversionError {
                    message: "Unable to convert integer to image format.".to_string(),
                }),
                ValueType::Decimal => Err(ConversionError {
                    message: "Unable to convert integer to image format.".to_string(),
                }),
                ValueType::String => Err(ConversionError {
                    message: "Unable to convert integer to image format.".to_string(),
                }),
                ValueType::FilterType => Err(ConversionError {
                    message: "Unable to convert integer to image format.".to_string(),
                }),
                ValueType::ColorFormat => Err(ConversionError {
                    message: "Unable to convert integer to image format.".to_string(),
                }),
                ValueType::Trigger => Err(ConversionError {
                    message: "Unable to convert integer to image format.".to_string(),
                }),
                ValueType::DynamicImage => Ok(Value::DynamicImage{ data: data.clone(), change_id: get_id() }),
                ValueType::Path => Err(ConversionError {
                    message: "Unable to convert integer to image format.".to_string(),
                }),
                ValueType::ImageFormat => Err(ConversionError {
                    message: "Unable to convert integer to image format.".to_string(),
                }),
            },
            Value::Path(path) => match other {
                ValueType::Bool => Err(ConversionError {
                    message: "Unable to convert integer to image format.".to_string(),
                }),
                ValueType::Integer => Err(ConversionError {
                    message: "Unable to convert integer to image format.".to_string(),
                }),
                ValueType::Decimal => Err(ConversionError {
                    message: "Unable to convert integer to image format.".to_string(),
                }),
                ValueType::String => {
                    if let Ok(path_string) = path.clone().into_os_string().into_string() {
                        Ok(Value::String(path_string))
                    } else {
                        Err(ConversionError {
                            message: "Unable to convert integer to image format.".to_string(),
                        })
                    }
                },
                ValueType::FilterType => Err(ConversionError {
                    message: "Unable to convert integer to image format.".to_string(),
                }),
                ValueType::ColorFormat => Err(ConversionError {
                    message: "Unable to convert integer to image format.".to_string(),
                }),
                ValueType::Trigger => Err(ConversionError {
                    message: "Unable to convert integer to image format.".to_string(),
                }),
                ValueType::DynamicImage => Err(ConversionError {
                    message: "Unable to convert integer to image format.".to_string(),
                }),
                ValueType::Path => {
                    Ok(Value::Path(path.clone()))
                },
                ValueType::ImageFormat => Err(ConversionError {
                    message: "Unable to convert integer to image format.".to_string(),
                }),
            }
            Value::ImageFormat(image_format) => match other {
                ValueType::Bool => Err(ConversionError {
                    message: "Unable to convert.".to_string(),
                }),
                ValueType::Integer => Err(ConversionError {
                    message: "Unable to convert.".to_string(),
                }),
                ValueType::Decimal => Err(ConversionError {
                    message: "Unable to convert.".to_string(),
                }),
                ValueType::String => Err(ConversionError {
                    message: "Unable to convert.".to_string(),
                }),
                ValueType::FilterType => Err(ConversionError {
                    message: "Unable to convert.".to_string(),
                }),
                ValueType::ColorFormat => Err(ConversionError {
                    message: "Unable to convert.".to_string(),
                }),
                ValueType::ImageFormat => {
                    Ok(Value::ImageFormat(image_format.clone()))
                },
                ValueType::Trigger => Err(ConversionError {
                    message: "Unable to convert.".to_string(),
                }),
                ValueType::DynamicImage => Err(ConversionError {
                    message: "Unable to convert.".to_string(),
                }),
                ValueType::Path => Err(ConversionError {
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
    FilterType,
    ColorFormat,
    ImageFormat,
    Trigger,
    DynamicImage,
    Path,
}

impl ValueType {

    pub fn types() -> [ValueType; 9] {
        let types: [ValueType; 9] = [
            ValueType::Bool,
            ValueType::Integer,
            ValueType::Decimal,
            ValueType::String,
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
            ValueType::FilterType => "filter type".to_string(),
            ValueType::ColorFormat => "color format".to_string(),
            ValueType::Trigger => "trigger".to_string(),
            ValueType::DynamicImage => "image".to_string(),
            ValueType::Path => "path".to_string(),
            ValueType::ImageFormat => "image format".to_string(),
        }
    }

    // file extensions that can be opened for each type
    pub fn file_extensions(value_type: &ValueType) -> Vec<String> {
        match value_type {
            ValueType::Bool => vec![],
            ValueType::Integer => vec![],
            ValueType::Decimal => vec![],
            ValueType::String => vec![],
            ValueType::FilterType => vec![],
            ValueType::ColorFormat => vec![],
            ValueType::Trigger => vec![],
            ValueType::DynamicImage => vec![
                "avif".to_string(),
                "jpg".to_string(),
                "jpeg".to_string(),
                "png".to_string(),
                "gif".to_string(),
                "webp".to_string(),
                "tif".to_string(),
                "tiff".to_string(),
                "tga".to_string(),
                "dds".to_string(),
                "bmp".to_string(),
                "ico".to_string(),
                "hdr".to_string(),
                "exr".to_string(),
                "pbm".to_string(),
                "pam".to_string(),
                "ppm".to_string(),
                "pgm".to_string(),
                "ff".to_string(),
                "farbfeld".to_string(),
                "qoi".to_string(),
            ],
            ValueType::Path => vec![],
            ValueType::ImageFormat => vec![],
        }
    }

    pub fn valid_conversions(&self) -> Vec<ValueType> {
        match self {
            ValueType::Bool => vec![ValueType::Bool, ValueType::Integer, ValueType::Decimal, ValueType::String, ValueType::Trigger],
            ValueType::Integer => vec![ValueType::Bool, ValueType::Integer, ValueType::Decimal, ValueType::String, ValueType::Trigger],
            ValueType::Decimal => vec![ValueType::Bool, ValueType::Integer, ValueType::Decimal, ValueType::String, ValueType::Trigger],
            ValueType::String => vec![ValueType::String, ValueType::Trigger],
            ValueType::DynamicImage => vec![ValueType::DynamicImage, ValueType::Trigger],
            ValueType::Path => vec![ValueType::String, ValueType::Path, ValueType::Trigger],
            ValueType::FilterType => vec![ValueType::FilterType, ValueType::String, ValueType::Trigger],
            ValueType::ColorFormat => vec![ValueType::ColorFormat, ValueType::String, ValueType::Trigger],
            ValueType::Trigger => vec![ValueType::Trigger],
            ValueType::ImageFormat => vec![ValueType::ImageFormat, ValueType::Trigger],
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
    ImageRgba32F,
    ImageRgb32F,
    ImageRgba16,
    ImageRgb16,
    ImageGrayA16,
    ImageGray16,
    ImageRgba8,
    ImageRgb8,
    ImageGrayA8,
    ImageGray8,
}

pub enum ImageFormat {

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
