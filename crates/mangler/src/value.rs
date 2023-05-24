use image::{imageops::FilterType, DynamicImage, ImageBuffer, Rgba};

pub const THUMBNAIL_SIZE: [u32; 2] = [128, 128];

#[derive(Debug, Clone)]
pub enum Value {
    Bool(bool),
    Integer(i32),
    Decimal(f32),
    String(String),
    DynamicImage(DynamicImage), // switch to using dynamicimage
    FilterType(FilterType),
    ImageFormat(ImageFormat),
    UiButton(bool),
}


impl Value {

    pub fn create_thumbnail(&self) -> Option<ImageBuffer<Rgba<u8>, Vec<u8>>> {
        match &self {
            Value::Bool(_) |
            Value::Integer(_) |
            Value::Decimal(_) |
            Value::String(_) |
            Value::FilterType(_) |
            Value::ImageFormat(_) => {
                None
            },
            Value::UiButton(_) => todo!(),
            Value::DynamicImage(value) =>  Some(value.thumbnail(THUMBNAIL_SIZE[0], THUMBNAIL_SIZE[1]).to_rgba8()),
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
            Value::DynamicImage(_) => ValueType::DynamicImage,
            Value::FilterType(_) => todo!(),
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
            Value::DynamicImage(_) => "image".to_string(),
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
                ValueType::FilterType => todo!(),
                ValueType::ImageFormat => Err(ConversionError {
                    message: "Unable to convert filter type to image format.".to_string(),
                }),
                ValueType::UiButton => Err(ConversionError {
                    message: "Unable to convert integer to image format.".to_string(),
                }),
                ValueType::DynamicImage => Err(ConversionError {
                    message: "Unable to convert integer to image format.".to_string(),
                }),
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
                ValueType::ImageFormat => Ok(Value::ImageFormat(a.clone())),
                ValueType::UiButton => todo!(),
                ValueType::DynamicImage => todo!(),
            },
            Value::UiButton(_) => todo!(),
            Value::DynamicImage(a) => match other {
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
            },
        }
    }
}

#[derive(Debug, Clone)]
pub enum ValueType {
    Bool,
    Integer,
    Decimal,
    String,
    FilterType,
    ImageFormat,
    UiButton,
    DynamicImage,
}


#[derive(Debug)]
pub struct ConversionError {
    pub message: String,
}

#[derive(Debug, Clone, Copy, PartialEq)]
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