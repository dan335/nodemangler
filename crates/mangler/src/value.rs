use std::path::PathBuf;

use image::{imageops::FilterType, DynamicImage};
use serde::{Deserialize, Serialize};

use crate::thumbnail::Thumbnail;

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
    Path {
        name: String,
        path: PathBuf,
        file_extensions: Vec<String>,
    },

    #[serde(
        serialize_with = "serialize_filter_type",
        deserialize_with = "deserialize_filter_type"
    )]
    FilterType(FilterType),

    ImageFormat(ImageFormat),
    Trigger {
        name: String,
    },
}

impl Value {
    pub fn create_thumbnail(&self) -> Option<Thumbnail> {
        match &self {
            Value::DynamicImage { data, change_id:_ } => Some(Thumbnail::Image(data.thumbnail(THUMBNAIL_SIZE[0], THUMBNAIL_SIZE[1]).to_rgba8())),
            Value::Bool(value) => Some(Thumbnail::Text(value.to_string())),
            Value::Integer(value) => Some(Thumbnail::Text(value.to_string())),
            Value::Decimal(value) => Some(Thumbnail::Text(value.to_string())),
            Value::String(value) => Some(Thumbnail::Text(value.clone())),
            Value::Path{ name, path, file_extensions } => Some(Thumbnail::Text(format!("{}", path.to_str().unwrap_or("none").to_string()))),
            Value::FilterType(value) => Some(Thumbnail::Text(format!("{:?}", value))),
            Value::ImageFormat(value) => Some(Thumbnail::Text(format!("{:?}", value))),
            Value::Trigger { name } => Some(Thumbnail::Text(name.to_string())),
        }
    }

    pub fn value_type(&self) -> ValueType {
        match self {
            Value::Bool(_) => ValueType::Bool,
            Value::Integer(_) => ValueType::Integer,
            Value::Decimal(_) => ValueType::Decimal,
            Value::String(_) => ValueType::String,
            Value::ImageFormat(_) => ValueType::ImageFormat,
            Value::Trigger { name } => ValueType::Trigger,
            Value::FilterType(_) => ValueType::FilterType,
            Value::Path{ name:_, path:_, file_extensions:_ } => ValueType::Path,
            Value::DynamicImage { data:_, change_id:_ } => ValueType::DynamicImage,
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
                ValueType::Trigger => Ok(Value::Bool(*a)),
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
                ValueType::Trigger => Err(ConversionError {
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
                ValueType::Trigger => Err(ConversionError {
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
                ValueType::Trigger => Err(ConversionError {
                    message: "Unable to convert integer to image format.".to_string(),
                }),
                ValueType::DynamicImage => Err(ConversionError {
                    message: "Unable to convert integer to image format.".to_string(),
                }),
                ValueType::Path => Err(ConversionError {
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
                ValueType::ImageFormat => Err(ConversionError {
                    message: "Unable to convert filter type to image format.".to_string(),
                }),
                ValueType::Trigger => Err(ConversionError {
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
                ValueType::Trigger => todo!(),
                ValueType::DynamicImage => todo!(),
                ValueType::Path => todo!(),
            },
            Value::Trigger { name } => todo!(),
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
                ValueType::Trigger => Err(ConversionError {
                    message: "Unable to convert integer to image format.".to_string(),
                }),
                ValueType::DynamicImage => todo!(),
                ValueType::Path => todo!(),
            },
            Value::Path { name, path, file_extensions } => match other {
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
                ValueType::ImageFormat => Err(ConversionError {
                    message: "Unable to convert integer to image format.".to_string(),
                }),
                ValueType::Trigger => Err(ConversionError {
                    message: "Unable to convert integer to image format.".to_string(),
                }),
                ValueType::DynamicImage => Err(ConversionError {
                    message: "Unable to convert integer to image format.".to_string(),
                }),
                ValueType::Path => {
                    Ok(Value::Path { name:name.clone(), path:path.clone(), file_extensions:file_extensions.clone() })
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
            ValueType::ImageFormat,
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
            ValueType::ImageFormat => "image format".to_string(),
            ValueType::Trigger => "trigger".to_string(),
            ValueType::DynamicImage => "image".to_string(),
            ValueType::Path => "path".to_string(),
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
            ValueType::ImageFormat => vec![ValueType::ImageFormat, ValueType::String, ValueType::Trigger],
            ValueType::Trigger => vec![ValueType::Trigger],
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
