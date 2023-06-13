use std::path::PathBuf;

use image::{imageops::FilterType, DynamicImage};
use serde::{Deserialize, Serialize};

use crate::thumbnail::Thumbnail;

pub const THUMBNAIL_SIZE: [u32; 2] = [128, 128];

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

    ImageFormat(ImageFormat),
    UiButton(bool),
}

impl Value {
    pub fn create_thumbnail(&self) -> Option<Thumbnail> {
        match &self {
            Value::DynamicImage { data, change_id:_ } => Some(Thumbnail::Image(data.thumbnail(THUMBNAIL_SIZE[0], THUMBNAIL_SIZE[1]).to_rgba8())),
            Value::Bool(value) => Some(Thumbnail::Text(value.to_string())),
            Value::Integer(value) => Some(Thumbnail::Text(value.to_string())),
            Value::Decimal(value) => Some(Thumbnail::Text(value.to_string())),
            Value::String(value) => Some(Thumbnail::Text(value.clone())),
            Value::Path(value) => Some(Thumbnail::Text(value.to_str().unwrap_or("None").to_string())),
            Value::FilterType(value) => Some(Thumbnail::Text(format!("{:?}", value))),
            Value::ImageFormat(value) => Some(Thumbnail::Text(format!("{:?}", value))),
            Value::UiButton(value) => Some(Thumbnail::Text(format!("{:?}", value))),
        }
    }

    pub fn value_type(&self) -> ValueType {
        match self {
            Value::Bool(_) => ValueType::Bool,
            Value::Integer(_) => ValueType::Integer,
            Value::Decimal(_) => ValueType::Decimal,
            Value::String(_) => ValueType::String,
            Value::ImageFormat(_) => ValueType::ImageFormat,
            Value::UiButton(_) => ValueType::UiButton,
            Value::FilterType(_) => ValueType::FilterType,
            Value::Path(_) => ValueType::Path,
            Value::DynamicImage { data:_, change_id:_ } => ValueType::DynamicImage,
        }
    }

    pub fn value_name(&self) -> String {
        match self {
            Value::Bool(_) => "bool".to_string(),
            Value::Integer(_) => "integer".to_string(),
            Value::Decimal(_) => "decimal".to_string(),
            Value::String(_) => "string".to_string(),
            Value::FilterType(_) => "filter type".to_string(),
            Value::ImageFormat(_) => "image format".to_string(),
            Value::UiButton(_) => "button".to_string(),
            Value::DynamicImage { data:_, change_id:_ } => "image".to_string(),
            Value::Path(_) => "path".to_string(),
        }
    }

    pub fn valid_conversions(&self) -> Vec<ValueType> {
        match self {
            Value::Bool(_) => vec![ValueType::Bool, ValueType::Integer, ValueType::Decimal, ValueType::String],
            Value::Integer(_) => vec![ValueType::Bool, ValueType::Integer, ValueType::Decimal, ValueType::String],
            Value::Decimal(_) => vec![ValueType::Bool, ValueType::Integer, ValueType::Decimal, ValueType::String],
            Value::String(_) => vec![ValueType::String, ValueType::Path],
            Value::DynamicImage { data:_, change_id:_ } => vec![ValueType::DynamicImage],
            Value::Path(_) => vec![ValueType::String, ValueType::Path],
            Value::FilterType(_) => vec![ValueType::FilterType, ValueType::String],
            Value::ImageFormat(_) => vec![ValueType::FilterType, ValueType::String],
            Value::UiButton(_) => vec![],
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
                ValueType::ImageFormat => Err(ConversionError {
                    message: "Unable to convert bool to image format.".to_string(),
                }),
                ValueType::UiButton => Ok(Value::Bool(*a)),
                ValueType::DynamicImage => Err(ConversionError {
                    message: "Unable to convert bool to image format.".to_string(),
                }),
                ValueType::Path => Err(ConversionError {
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
                ValueType::ImageFormat => Err(ConversionError {
                    message: "Unable to convert integer to image format.".to_string(),
                }),
                ValueType::UiButton => Err(ConversionError {
                    message: "Unable to convert integer to image format.".to_string(),
                }),
                ValueType::DynamicImage => Err(ConversionError {
                    message: "Unable to convert integer to image format.".to_string(),
                }),
                ValueType::Path => Err(ConversionError {
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
                ValueType::ImageFormat => Err(ConversionError {
                    message: "Unable to convert decimal to image format.".to_string(),
                }),
                ValueType::UiButton => Err(ConversionError {
                    message: "Unable to convert integer to image format.".to_string(),
                }),
                ValueType::DynamicImage => Err(ConversionError {
                    message: "Unable to convert integer to image format.".to_string(),
                }),
                ValueType::Path => Err(ConversionError {
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
                ValueType::ImageFormat => Err(ConversionError {
                    message: "Unable to convert string to image format.".to_string(),
                }),
                ValueType::UiButton => Err(ConversionError {
                    message: "Unable to convert integer to image format.".to_string(),
                }),
                ValueType::DynamicImage => Err(ConversionError {
                    message: "Unable to convert integer to image format.".to_string(),
                }),
                ValueType::Path => {
                    let path = PathBuf::from(a.clone());
                    Ok(Value::Path(path))
                }
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
                ValueType::ImageFormat => Err(ConversionError {
                    message: "Unable to convert filter type to image format.".to_string(),
                }),
                ValueType::UiButton => Err(ConversionError {
                    message: "Unable to convert integer to image format.".to_string(),
                }),
                ValueType::DynamicImage => Err(ConversionError {
                    message: "Unable to convert integer to image format.".to_string(),
                }),
                ValueType::Path => todo!(),
            },
            Value::ImageFormat(a) => match other {
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
                ValueType::ImageFormat => Ok(Value::ImageFormat(*a)),
                ValueType::UiButton => todo!(),
                ValueType::DynamicImage => todo!(),
                ValueType::Path => todo!(),
            },
            Value::UiButton(_) => todo!(),
            Value::DynamicImage { data:_, change_id:_ } => match other {
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
                ValueType::ImageFormat => Err(ConversionError {
                    message: "Unable to convert integer to image format.".to_string(),
                }),
                ValueType::UiButton => Err(ConversionError {
                    message: "Unable to convert integer to image format.".to_string(),
                }),
                ValueType::DynamicImage => todo!(),
                ValueType::Path => todo!(),
            },
            Value::Path(a) => match other {
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
                    if let Ok(path_string) = a.clone().into_os_string().into_string() {
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
                ValueType::ImageFormat => Err(ConversionError {
                    message: "Unable to convert integer to image format.".to_string(),
                }),
                ValueType::UiButton => Err(ConversionError {
                    message: "Unable to convert integer to image format.".to_string(),
                }),
                ValueType::DynamicImage => Err(ConversionError {
                    message: "Unable to convert integer to image format.".to_string(),
                }),
                ValueType::Path => {
                    Ok(Value::Path(a.clone()))
                },
            }
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
    ImageFormat,
    UiButton,
    DynamicImage,
    Path,
}

#[derive(Debug)]
pub struct ConversionError {
    pub message: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum ImageFormat {
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
