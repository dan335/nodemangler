use std::panic;

use image::{
    imageops::FilterType, GrayAlphaImage, GrayImage, ImageBuffer, Luma, LumaA, Rgb, Rgb32FImage,
    RgbImage, Rgba, Rgba32FImage, RgbaImage, DynamicImage,
};

#[derive(Debug, Clone)]
pub enum Value {
    Bool(bool),
    Integer(i32),
    Decimal(f32),
    String(String),
    ImageRgba32F(Rgba32FImage),
    ImageRgb32F(Rgb32FImage),
    ImageRgba16(ImageBuffer<Rgba<u16>, Vec<u16>>),
    ImageRgb16(ImageBuffer<Rgb<u16>, Vec<u16>>),
    ImageGrayA16(ImageBuffer<LumaA<u16>, Vec<u16>>),
    ImageGray16(ImageBuffer<Luma<u16>, Vec<u16>>),
    ImageRgba8(RgbaImage),
    ImageRgb8(RgbImage),
    ImageGrayA8(GrayAlphaImage),
    ImageGray8(GrayImage),
    FilterType(FilterType),
    ImageFormat(ImageFormat),
}

impl Value {
    pub fn value_type(self) -> ValueType {
        match self {
            Value::Bool(_) => ValueType::Bool,
            Value::Integer(_) => ValueType::Integer,
            Value::Decimal(_) => ValueType::Decimal,
            Value::String(_) => ValueType::String,
            Value::ImageRgba32F(_) => ValueType::ImageRgba32F,
            Value::ImageRgba8(_) => ValueType::ImageRgba8,
            Value::ImageGray8(_) => ValueType::ImageGray8,
            Value::FilterType(_) => ValueType::FilterType,
            Value::ImageRgb32F(_) => ValueType::ImageRgb32F,
            Value::ImageRgba16(_) => ValueType::ImageRgba16,
            Value::ImageRgb16(_) => ValueType::ImageRgb16,
            Value::ImageGrayA16(_) => ValueType::ImageGrayA16,
            Value::ImageGray16(_) => ValueType::ImageGray16,
            Value::ImageRgb8(_) => ValueType::ImageRgb8,
            Value::ImageGrayA8(_) => ValueType::ImageGrayA8,
            Value::ImageFormat(_) => ValueType::ImageFormat,
        }
    }

    pub fn value_name(&self) -> String {
        match self {
            Value::Bool(_) => "bool".to_string(),
            Value::Integer(_) => "integer".to_string(),
            Value::Decimal(_) => "decimal".to_string(),
            Value::String(_) => "string".to_string(),
            Value::ImageRgba32F(_) => "rgba 32f".to_string(),
            Value::ImageRgb32F(_) => "rgb 32f".to_string(),
            Value::ImageRgba16(_) => "rgba 16".to_string(),
            Value::ImageRgb16(_) => "rgb 16".to_string(),
            Value::ImageGrayA16(_) => "gray a 16".to_string(),
            Value::ImageGray16(_) => "gray 16".to_string(),
            Value::ImageRgba8(_) => "rgba 8".to_string(),
            Value::ImageRgb8(_) => "rgb 8".to_string(),
            Value::ImageGrayA8(_) => "gray a 8".to_string(),
            Value::ImageGray8(_) => "gray 8".to_string(),
            Value::FilterType(_) => "filter type".to_string(),
            Value::ImageFormat(_) => "image format".to_string(),
        }
    }

    pub fn convert_to(&self, other: ValueType) -> Result<Value, ConversionError> {
        match self {
            Value::Bool(a) => match other {
                ValueType::Bool => Ok(Value::Bool(*a)),
                ValueType::Integer => {
                    if *a {
                        Ok(Value::Integer(1))
                    } else {
                        Ok(Value::Integer(0))
                    }
                },
                ValueType::Decimal => {
                    if *a {
                        Ok(Value::Decimal(1.0))
                    } else {
                        Ok(Value::Decimal(0.0))
                    }
                },
                ValueType::String => Ok(Value::String(a.to_string())),
                ValueType::ImageRgba32F |
                ValueType::ImageRgb32F |
                ValueType::ImageRgba16 |
                ValueType::ImageRgb16 |
                ValueType::ImageGrayA16 |
                ValueType::ImageGray16 |
                ValueType::ImageRgba8 |
                ValueType::ImageRgb8 |
                ValueType::ImageGrayA8 |
                ValueType::ImageGray8
                 => Err(ConversionError {
                    message: "Unable to convert bool to image.".to_string(),
                }),
                ValueType::FilterType => Err(ConversionError {
                    message: "Unable to convert bool to filter type.".to_string(),
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
                ValueType::ImageRgba32F |
                ValueType::ImageRgb32F |
                ValueType::ImageRgba16 |
                ValueType::ImageRgb16 |
                ValueType::ImageGrayA16 |
                ValueType::ImageGray16 |
                ValueType::ImageRgba8 |
                ValueType::ImageRgb8 |
                ValueType::ImageGrayA8 |
                ValueType::ImageGray8
                 => Err(ConversionError {
                    message: "Unable to convert integer to image.".to_string(),
                }),
                ValueType::FilterType => Err(ConversionError {
                    message: "Unable to convert bool to filter type.".to_string(),
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
                ValueType::ImageRgba32F |
                ValueType::ImageRgb32F |
                ValueType::ImageRgba16 |
                ValueType::ImageRgb16 |
                ValueType::ImageGrayA16 |
                ValueType::ImageGray16 |
                ValueType::ImageRgba8 |
                ValueType::ImageRgb8 |
                ValueType::ImageGrayA8 |
                ValueType::ImageGray8
                 => Err(ConversionError {
                    message: "Unable to convert decimal to image.".to_string(),
                }),
                ValueType::FilterType => Err(ConversionError {
                    message: "Unable to convert bool to filter type.".to_string(),
                }),
                ValueType::ImageFormat => Err(ConversionError {
                    message: "Unable to convert decimal to image format.".to_string(),
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
                },
                ValueType::Decimal => {
                    let result: Result<f32, _> = a.parse();
                    match result {
                        Ok(r) => Ok(Value::Decimal(r)),
                        Err(_) => Err(ConversionError {
                            message: "Error converting string to decimal.".to_string(),
                        }),
                    }
                },
                ValueType::String => Ok(Value::String(a.clone())),
                ValueType::ImageRgba32F |
                ValueType::ImageRgb32F |
                ValueType::ImageRgba16 |
                ValueType::ImageRgb16 |
                ValueType::ImageGrayA16 |
                ValueType::ImageGray16 |
                ValueType::ImageRgba8 |
                ValueType::ImageRgb8 |
                ValueType::ImageGrayA8 |
                ValueType::ImageGray8
                 => Err(ConversionError {
                    message: "Unable to convert string to image.".to_string(),
                }),
                ValueType::FilterType => Err(ConversionError {
                    message: "Unable to convert bool to filter type.".to_string(),
                }),
                ValueType::ImageFormat => Err(ConversionError {
                    message: "Unable to convert string to image format.".to_string(),
                }),
            },
            Value::ImageRgba32F(a) => match other {
                ValueType::Bool => Err(ConversionError {
                    message: "Unable to convert image to bool.".to_string(),
                }),
                ValueType::Integer => Err(ConversionError {
                    message: "Unable to convert image to integer.".to_string(),
                }),
                ValueType::Decimal => Err(ConversionError {
                    message: "Unable to convert image to decimal.".to_string(),
                }),
                ValueType::String => Err(ConversionError {
                    message: "Unable to convert image to string.".to_string(),
                }),
                ValueType::ImageRgba32F => {
                    Ok(Value::ImageRgba32F(a.clone()))
                },
                ValueType::ImageRgb32F => {
                    Ok(Value::ImageRgb32F(DynamicImage::ImageRgba32F(a.clone()).into_rgb32f()))
                },
                ValueType::ImageRgba16 => {
                    Ok(Value::ImageRgba16(DynamicImage::ImageRgba32F(a.clone()).into_rgba16()))
                },
                ValueType::ImageRgb16 => {
                    Ok(Value::ImageRgb16(DynamicImage::ImageRgba32F(a.clone()).into_rgb16()))
                },
                ValueType::ImageGrayA16 => {
                    Ok(Value::ImageGrayA16(DynamicImage::ImageRgba32F(a.clone()).into_luma_alpha16()))
                },
                ValueType::ImageGray16 => {
                    Ok(Value::ImageGray16(DynamicImage::ImageRgba32F(a.clone()).into_luma16()))
                },
                ValueType::ImageRgba8 => {
                    Ok(Value::ImageRgba8(DynamicImage::ImageRgba32F(a.clone()).into_rgba8()))
                },
                ValueType::ImageRgb8 => {
                    Ok(Value::ImageRgb8(DynamicImage::ImageRgba32F(a.clone()).into_rgb8()))
                },
                ValueType::ImageGrayA8 => {
                    Ok(Value::ImageGrayA8(DynamicImage::ImageRgba32F(a.clone()).into_luma_alpha8()))
                },
                ValueType::ImageGray8 => {
                    Ok(Value::ImageGray8(DynamicImage::ImageRgba32F(a.clone()).into_luma8()))
                },
                ValueType::FilterType => Err(ConversionError {
                    message: "Unable to convert image to filter type.".to_string(),
                }),
                ValueType::ImageFormat => {
                    Ok(Value::ImageFormat(ImageFormat::ImageRgba32F))
                }
            },
            Value::ImageRgb32F(a) => match other {
                ValueType::Bool => Err(ConversionError {
                    message: "Unable to convert image to bool.".to_string(),
                }),
                ValueType::Integer => Err(ConversionError {
                    message: "Unable to convert image to integer.".to_string(),
                }),
                ValueType::Decimal => Err(ConversionError {
                    message: "Unable to convert image to decimal.".to_string(),
                }),
                ValueType::String => Err(ConversionError {
                    message: "Unable to convert image to string.".to_string(),
                }),
                ValueType::ImageRgba32F => {
                    Ok(Value::ImageRgba32F(DynamicImage::ImageRgb32F(a.clone()).into_rgba32f()))
                },
                ValueType::ImageRgb32F => {
                    Ok(Value::ImageRgb32F(a.clone()))
                },
                ValueType::ImageRgba16 => {
                    Ok(Value::ImageRgba16(DynamicImage::ImageRgb32F(a.clone()).into_rgba16()))
                },
                ValueType::ImageRgb16 => {
                    Ok(Value::ImageRgb16(DynamicImage::ImageRgb32F(a.clone()).into_rgb16()))
                },
                ValueType::ImageGrayA16 => {
                    Ok(Value::ImageGrayA16(DynamicImage::ImageRgb32F(a.clone()).into_luma_alpha16()))
                },
                ValueType::ImageGray16 => {
                    Ok(Value::ImageGray16(DynamicImage::ImageRgb32F(a.clone()).into_luma16()))
                },
                ValueType::ImageRgba8 => {
                    Ok(Value::ImageRgba8(DynamicImage::ImageRgb32F(a.clone()).into_rgba8()))
                },
                ValueType::ImageRgb8 => {
                    Ok(Value::ImageRgb8(DynamicImage::ImageRgb32F(a.clone()).into_rgb8()))
                },
                ValueType::ImageGrayA8 => {
                    Ok(Value::ImageGrayA8(DynamicImage::ImageRgb32F(a.clone()).into_luma_alpha8()))
                },
                ValueType::ImageGray8 => {
                    Ok(Value::ImageGray8(DynamicImage::ImageRgb32F(a.clone()).into_luma8()))
                },
                ValueType::FilterType => Err(ConversionError {
                    message: "Unable to convert image to filter type.".to_string(),
                }),
                ValueType::ImageFormat => {
                    Ok(Value::ImageFormat(ImageFormat::ImageRgb32F))
                }
            },
            Value::ImageRgba16(a) => match other {
                ValueType::Bool => Err(ConversionError {
                    message: "Unable to convert image to bool.".to_string(),
                }),
                ValueType::Integer => Err(ConversionError {
                    message: "Unable to convert image to integer.".to_string(),
                }),
                ValueType::Decimal => Err(ConversionError {
                    message: "Unable to convert image to decimal.".to_string(),
                }),
                ValueType::String => Err(ConversionError {
                    message: "Unable to convert image to string.".to_string(),
                }),
                ValueType::ImageRgba32F => {
                    Ok(Value::ImageRgba32F(DynamicImage::ImageRgba16(a.clone()).into_rgba32f()))
                },
                ValueType::ImageRgb32F => {
                    Ok(Value::ImageRgb32F(DynamicImage::ImageRgba16(a.clone()).into_rgb32f()))
                },
                ValueType::ImageRgba16 => {
                    Ok(Value::ImageRgba16(DynamicImage::ImageRgba16(a.clone()).into_rgba16()))
                },
                ValueType::ImageRgb16 => {
                    Ok(Value::ImageRgb16(DynamicImage::ImageRgba16(a.clone()).into_rgb16()))
                },
                ValueType::ImageGrayA16 => {
                    Ok(Value::ImageGrayA16(DynamicImage::ImageRgba16(a.clone()).into_luma_alpha16()))
                },
                ValueType::ImageGray16 => {
                    Ok(Value::ImageGray16(DynamicImage::ImageRgba16(a.clone()).into_luma16()))
                },
                ValueType::ImageRgba8 => {
                    Ok(Value::ImageRgba8(DynamicImage::ImageRgba16(a.clone()).into_rgba8()))
                },
                ValueType::ImageRgb8 => {
                    Ok(Value::ImageRgb8(DynamicImage::ImageRgba16(a.clone()).into_rgb8()))
                },
                ValueType::ImageGrayA8 => {
                    Ok(Value::ImageGrayA8(DynamicImage::ImageRgba16(a.clone()).into_luma_alpha8()))
                },
                ValueType::ImageGray8 => {
                    Ok(Value::ImageGray8(DynamicImage::ImageRgba16(a.clone()).into_luma8()))
                },
                ValueType::FilterType => Err(ConversionError {
                    message: "Unable to convert image to filter type.".to_string(),
                }),
                ValueType::ImageFormat => {
                    Ok(Value::ImageFormat(ImageFormat::ImageRgba16))
                }
            },
            Value::ImageRgb16(a) => match other {
                ValueType::Bool => Err(ConversionError {
                    message: "Unable to convert image to bool.".to_string(),
                }),
                ValueType::Integer => Err(ConversionError {
                    message: "Unable to convert image to integer.".to_string(),
                }),
                ValueType::Decimal => Err(ConversionError {
                    message: "Unable to convert image to decimal.".to_string(),
                }),
                ValueType::String => Err(ConversionError {
                    message: "Unable to convert image to string.".to_string(),
                }),
                ValueType::ImageRgba32F => {
                    Ok(Value::ImageRgba32F(DynamicImage::ImageRgb16(a.clone()).into_rgba32f()))
                },
                ValueType::ImageRgb32F => {
                    Ok(Value::ImageRgb32F(DynamicImage::ImageRgb16(a.clone()).into_rgb32f()))
                },
                ValueType::ImageRgba16 => {
                    Ok(Value::ImageRgba16(DynamicImage::ImageRgb16(a.clone()).into_rgba16()))
                },
                ValueType::ImageRgb16 => {
                    Ok(Value::ImageRgb16(DynamicImage::ImageRgb16(a.clone()).into_rgb16()))
                },
                ValueType::ImageGrayA16 => {
                    Ok(Value::ImageGrayA16(DynamicImage::ImageRgb16(a.clone()).into_luma_alpha16()))
                },
                ValueType::ImageGray16 => {
                    Ok(Value::ImageGray16(DynamicImage::ImageRgb16(a.clone()).into_luma16()))
                },
                ValueType::ImageRgba8 => {
                    Ok(Value::ImageRgba8(DynamicImage::ImageRgb16(a.clone()).into_rgba8()))
                },
                ValueType::ImageRgb8 => {
                    Ok(Value::ImageRgb8(DynamicImage::ImageRgb16(a.clone()).into_rgb8()))
                },
                ValueType::ImageGrayA8 => {
                    Ok(Value::ImageGrayA8(DynamicImage::ImageRgb16(a.clone()).into_luma_alpha8()))
                },
                ValueType::ImageGray8 => {
                    Ok(Value::ImageGray8(DynamicImage::ImageRgb16(a.clone()).into_luma8()))
                },
                ValueType::FilterType => Err(ConversionError {
                    message: "Unable to convert image to filter type.".to_string(),
                }),
                ValueType::ImageFormat => {
                    Ok(Value::ImageFormat(ImageFormat::ImageRgb16))
                }
            },
            Value::ImageGrayA16(a) => match other {
                ValueType::Bool => Err(ConversionError {
                    message: "Unable to convert image to bool.".to_string(),
                }),
                ValueType::Integer => Err(ConversionError {
                    message: "Unable to convert image to integer.".to_string(),
                }),
                ValueType::Decimal => Err(ConversionError {
                    message: "Unable to convert image to decimal.".to_string(),
                }),
                ValueType::String => Err(ConversionError {
                    message: "Unable to convert image to string.".to_string(),
                }),
                ValueType::ImageRgba32F => {
                    Ok(Value::ImageRgba32F(DynamicImage::ImageLumaA16(a.clone()).into_rgba32f()))
                },
                ValueType::ImageRgb32F => {
                    Ok(Value::ImageRgb32F(DynamicImage::ImageLumaA16(a.clone()).into_rgb32f()))
                },
                ValueType::ImageRgba16 => {
                    Ok(Value::ImageRgba16(DynamicImage::ImageLumaA16(a.clone()).into_rgba16()))
                },
                ValueType::ImageRgb16 => {
                    Ok(Value::ImageRgb16(DynamicImage::ImageLumaA16(a.clone()).into_rgb16()))
                },
                ValueType::ImageGrayA16 => {
                    Ok(Value::ImageGrayA16(DynamicImage::ImageLumaA16(a.clone()).into_luma_alpha16()))
                },
                ValueType::ImageGray16 => {
                    Ok(Value::ImageGray16(DynamicImage::ImageLumaA16(a.clone()).into_luma16()))
                },
                ValueType::ImageRgba8 => {
                    Ok(Value::ImageRgba8(DynamicImage::ImageLumaA16(a.clone()).into_rgba8()))
                },
                ValueType::ImageRgb8 => {
                    Ok(Value::ImageRgb8(DynamicImage::ImageLumaA16(a.clone()).into_rgb8()))
                },
                ValueType::ImageGrayA8 => {
                    Ok(Value::ImageGrayA8(DynamicImage::ImageLumaA16(a.clone()).into_luma_alpha8()))
                },
                ValueType::ImageGray8 => {
                    Ok(Value::ImageGray8(DynamicImage::ImageLumaA16(a.clone()).into_luma8()))
                },
                ValueType::FilterType => Err(ConversionError {
                    message: "Unable to convert image to filter type.".to_string(),
                }),
                ValueType::ImageFormat => {
                    Ok(Value::ImageFormat(ImageFormat::ImageGrayA16))
                }
            },
            Value::ImageGray16(a) => match other {
                ValueType::Bool => Err(ConversionError {
                    message: "Unable to convert image to bool.".to_string(),
                }),
                ValueType::Integer => Err(ConversionError {
                    message: "Unable to convert image to integer.".to_string(),
                }),
                ValueType::Decimal => Err(ConversionError {
                    message: "Unable to convert image to decimal.".to_string(),
                }),
                ValueType::String => Err(ConversionError {
                    message: "Unable to convert image to string.".to_string(),
                }),
                ValueType::ImageRgba32F => {
                    Ok(Value::ImageRgba32F(DynamicImage::ImageLuma16(a.clone()).into_rgba32f()))
                },
                ValueType::ImageRgb32F => {
                    Ok(Value::ImageRgb32F(DynamicImage::ImageLuma16(a.clone()).into_rgb32f()))
                },
                ValueType::ImageRgba16 => {
                    Ok(Value::ImageRgba16(DynamicImage::ImageLuma16(a.clone()).into_rgba16()))
                },
                ValueType::ImageRgb16 => {
                    Ok(Value::ImageRgb16(DynamicImage::ImageLuma16(a.clone()).into_rgb16()))
                },
                ValueType::ImageGrayA16 => {
                    Ok(Value::ImageGrayA16(DynamicImage::ImageLuma16(a.clone()).into_luma_alpha16()))
                },
                ValueType::ImageGray16 => {
                    Ok(Value::ImageGray16(DynamicImage::ImageLuma16(a.clone()).into_luma16()))
                },
                ValueType::ImageRgba8 => {
                    Ok(Value::ImageRgba8(DynamicImage::ImageLuma16(a.clone()).into_rgba8()))
                },
                ValueType::ImageRgb8 => {
                    Ok(Value::ImageRgb8(DynamicImage::ImageLuma16(a.clone()).into_rgb8()))
                },
                ValueType::ImageGrayA8 => {
                    Ok(Value::ImageGrayA8(DynamicImage::ImageLuma16(a.clone()).into_luma_alpha8()))
                },
                ValueType::ImageGray8 => {
                    Ok(Value::ImageGray8(DynamicImage::ImageLuma16(a.clone()).into_luma8()))
                },
                ValueType::FilterType => Err(ConversionError {
                    message: "Unable to convert image to filter type.".to_string(),
                }),
                ValueType::ImageFormat => {
                    Ok(Value::ImageFormat(ImageFormat::ImageGray16))
                }
            },
            Value::ImageRgba8(a) => match other {
                ValueType::Bool => Err(ConversionError {
                    message: "Unable to convert image to bool.".to_string(),
                }),
                ValueType::Integer => Err(ConversionError {
                    message: "Unable to convert image to integer.".to_string(),
                }),
                ValueType::Decimal => Err(ConversionError {
                    message: "Unable to convert image to decimal.".to_string(),
                }),
                ValueType::String => Err(ConversionError {
                    message: "Unable to convert image to string.".to_string(),
                }),
                ValueType::ImageRgba32F => {
                    Ok(Value::ImageRgba32F(DynamicImage::ImageRgba8(a.clone()).into_rgba32f()))
                },
                ValueType::ImageRgb32F => {
                    Ok(Value::ImageRgb32F(DynamicImage::ImageRgba8(a.clone()).into_rgb32f()))
                },
                ValueType::ImageRgba16 => {
                    Ok(Value::ImageRgba16(DynamicImage::ImageRgba8(a.clone()).into_rgba16()))
                },
                ValueType::ImageRgb16 => {
                    Ok(Value::ImageRgb16(DynamicImage::ImageRgba8(a.clone()).into_rgb16()))
                },
                ValueType::ImageGrayA16 => {
                    Ok(Value::ImageGrayA16(DynamicImage::ImageRgba8(a.clone()).into_luma_alpha16()))
                },
                ValueType::ImageGray16 => {
                    Ok(Value::ImageGray16(DynamicImage::ImageRgba8(a.clone()).into_luma16()))
                },
                ValueType::ImageRgba8 => {
                    Ok(Value::ImageRgba8(DynamicImage::ImageRgba8(a.clone()).into_rgba8()))
                },
                ValueType::ImageRgb8 => {
                    Ok(Value::ImageRgb8(DynamicImage::ImageRgba8(a.clone()).into_rgb8()))
                },
                ValueType::ImageGrayA8 => {
                    Ok(Value::ImageGrayA8(DynamicImage::ImageRgba8(a.clone()).into_luma_alpha8()))
                },
                ValueType::ImageGray8 => {
                    Ok(Value::ImageGray8(DynamicImage::ImageRgba8(a.clone()).into_luma8()))
                },
                ValueType::FilterType => Err(ConversionError {
                    message: "Unable to convert image to filter type.".to_string(),
                }),
                ValueType::ImageFormat => {
                    Ok(Value::ImageFormat(ImageFormat::ImageRgba8))
                }
            },
            Value::ImageRgb8(a) => match other {
                ValueType::Bool => Err(ConversionError {
                    message: "Unable to convert image to bool.".to_string(),
                }),
                ValueType::Integer => Err(ConversionError {
                    message: "Unable to convert image to integer.".to_string(),
                }),
                ValueType::Decimal => Err(ConversionError {
                    message: "Unable to convert image to decimal.".to_string(),
                }),
                ValueType::String => Err(ConversionError {
                    message: "Unable to convert image to string.".to_string(),
                }),
                ValueType::ImageRgba32F => {
                    Ok(Value::ImageRgba32F(DynamicImage::ImageRgb8(a.clone()).into_rgba32f()))
                },
                ValueType::ImageRgb32F => {
                    Ok(Value::ImageRgb32F(DynamicImage::ImageRgb8(a.clone()).into_rgb32f()))
                },
                ValueType::ImageRgba16 => {
                    Ok(Value::ImageRgba16(DynamicImage::ImageRgb8(a.clone()).into_rgba16()))
                },
                ValueType::ImageRgb16 => {
                    Ok(Value::ImageRgb16(DynamicImage::ImageRgb8(a.clone()).into_rgb16()))
                },
                ValueType::ImageGrayA16 => {
                    Ok(Value::ImageGrayA16(DynamicImage::ImageRgb8(a.clone()).into_luma_alpha16()))
                },
                ValueType::ImageGray16 => {
                    Ok(Value::ImageGray16(DynamicImage::ImageRgb8(a.clone()).into_luma16()))
                },
                ValueType::ImageRgba8 => {
                    Ok(Value::ImageRgba8(DynamicImage::ImageRgb8(a.clone()).into_rgba8()))
                },
                ValueType::ImageRgb8 => {
                    Ok(Value::ImageRgb8(DynamicImage::ImageRgb8(a.clone()).into_rgb8()))
                },
                ValueType::ImageGrayA8 => {
                    Ok(Value::ImageGrayA8(DynamicImage::ImageRgb8(a.clone()).into_luma_alpha8()))
                },
                ValueType::ImageGray8 => {
                    Ok(Value::ImageGray8(DynamicImage::ImageRgb8(a.clone()).into_luma8()))
                },
                ValueType::FilterType => Err(ConversionError {
                    message: "Unable to convert image to filter type.".to_string(),
                }),
                ValueType::ImageFormat => {
                    Ok(Value::ImageFormat(ImageFormat::ImageRgb8))
                }
            },
            Value::ImageGrayA8(a) => match other {
                ValueType::Bool => Err(ConversionError {
                    message: "Unable to convert image to bool.".to_string(),
                }),
                ValueType::Integer => Err(ConversionError {
                    message: "Unable to convert image to integer.".to_string(),
                }),
                ValueType::Decimal => Err(ConversionError {
                    message: "Unable to convert image to decimal.".to_string(),
                }),
                ValueType::String => Err(ConversionError {
                    message: "Unable to convert image to string.".to_string(),
                }),
                ValueType::ImageRgba32F => {
                    Ok(Value::ImageRgba32F(DynamicImage::ImageLumaA8(a.clone()).into_rgba32f()))
                },
                ValueType::ImageRgb32F => {
                    Ok(Value::ImageRgb32F(DynamicImage::ImageLumaA8(a.clone()).into_rgb32f()))
                },
                ValueType::ImageRgba16 => {
                    Ok(Value::ImageRgba16(DynamicImage::ImageLumaA8(a.clone()).into_rgba16()))
                },
                ValueType::ImageRgb16 => {
                    Ok(Value::ImageRgb16(DynamicImage::ImageLumaA8(a.clone()).into_rgb16()))
                },
                ValueType::ImageGrayA16 => {
                    Ok(Value::ImageGrayA16(DynamicImage::ImageLumaA8(a.clone()).into_luma_alpha16()))
                },
                ValueType::ImageGray16 => {
                    Ok(Value::ImageGray16(DynamicImage::ImageLumaA8(a.clone()).into_luma16()))
                },
                ValueType::ImageRgba8 => {
                    Ok(Value::ImageRgba8(DynamicImage::ImageLumaA8(a.clone()).into_rgba8()))
                },
                ValueType::ImageRgb8 => {
                    Ok(Value::ImageRgb8(DynamicImage::ImageLumaA8(a.clone()).into_rgb8()))
                },
                ValueType::ImageGrayA8 => {
                    Ok(Value::ImageGrayA8(DynamicImage::ImageLumaA8(a.clone()).into_luma_alpha8()))
                },
                ValueType::ImageGray8 => {
                    Ok(Value::ImageGray8(DynamicImage::ImageLumaA8(a.clone()).into_luma8()))
                },
                ValueType::FilterType => Err(ConversionError {
                    message: "Unable to convert image to filter type.".to_string(),
                }),
                ValueType::ImageFormat => {
                    Ok(Value::ImageFormat(ImageFormat::ImageGrayA8))
                }
            },
            Value::ImageGray8(a) => match other {
                ValueType::Bool => Err(ConversionError {
                    message: "Unable to convert image to bool.".to_string(),
                }),
                ValueType::Integer => Err(ConversionError {
                    message: "Unable to convert image to integer.".to_string(),
                }),
                ValueType::Decimal => Err(ConversionError {
                    message: "Unable to convert image to decimal.".to_string(),
                }),
                ValueType::String => Err(ConversionError {
                    message: "Unable to convert image to string.".to_string(),
                }),
                ValueType::ImageRgba32F => {
                    Ok(Value::ImageRgba32F(DynamicImage::ImageLuma8(a.clone()).into_rgba32f()))
                },
                ValueType::ImageRgb32F => {
                    Ok(Value::ImageRgb32F(DynamicImage::ImageLuma8(a.clone()).into_rgb32f()))
                },
                ValueType::ImageRgba16 => {
                    Ok(Value::ImageRgba16(DynamicImage::ImageLuma8(a.clone()).into_rgba16()))
                },
                ValueType::ImageRgb16 => {
                    Ok(Value::ImageRgb16(DynamicImage::ImageLuma8(a.clone()).into_rgb16()))
                },
                ValueType::ImageGrayA16 => {
                    Ok(Value::ImageGrayA16(DynamicImage::ImageLuma8(a.clone()).into_luma_alpha16()))
                },
                ValueType::ImageGray16 => {
                    Ok(Value::ImageGray16(DynamicImage::ImageLuma8(a.clone()).into_luma16()))
                },
                ValueType::ImageRgba8 => {
                    Ok(Value::ImageRgba8(DynamicImage::ImageLuma8(a.clone()).into_rgba8()))
                },
                ValueType::ImageRgb8 => {
                    Ok(Value::ImageRgb8(DynamicImage::ImageLuma8(a.clone()).into_rgb8()))
                },
                ValueType::ImageGrayA8 => {
                    Ok(Value::ImageGrayA8(DynamicImage::ImageLuma8(a.clone()).into_luma_alpha8()))
                },
                ValueType::ImageGray8 => {
                    Ok(Value::ImageGray8(DynamicImage::ImageLuma8(a.clone()).into_luma8()))
                },
                ValueType::FilterType => Err(ConversionError {
                    message: "Unable to convert image to filter type.".to_string(),
                }),
                ValueType::ImageFormat => {
                    Ok(Value::ImageFormat(ImageFormat::ImageGray8))
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
                ValueType::ImageRgba32F => todo!(),
                ValueType::ImageRgb32F => todo!(),
                ValueType::ImageRgba16 => todo!(),
                ValueType::ImageRgb16 => todo!(),
                ValueType::ImageGrayA16 => todo!(),
                ValueType::ImageGray16 => todo!(),
                ValueType::ImageRgba8 => todo!(),
                ValueType::ImageRgb8 => todo!(),
                ValueType::ImageGrayA8 => todo!(),
                ValueType::ImageGray8 => todo!(),
                ValueType::FilterType => todo!(),
                ValueType::ImageFormat => Err(ConversionError {
                    message: "Unable to convert filter type to image format.".to_string(),
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
                ValueType::ImageRgba32F => Err(ConversionError {
                    message: "Unable to convert image type to image.".to_string(),
                }),
                ValueType::ImageRgb32F => Err(ConversionError {
                    message: "Unable to convert image type to image.".to_string(),
                }),
                ValueType::ImageRgba16 => Err(ConversionError {
                    message: "Unable to convert image type to image.".to_string(),
                }),
                ValueType::ImageRgb16 => Err(ConversionError {
                    message: "Unable to convert image type to image.".to_string(),
                }),
                ValueType::ImageGrayA16 => Err(ConversionError {
                    message: "Unable to convert image type to image.".to_string(),
                }),
                ValueType::ImageGray16 => Err(ConversionError {
                    message: "Unable to convert image type to image.".to_string(),
                }),
                ValueType::ImageRgba8 => Err(ConversionError {
                    message: "Unable to convert image type to image.".to_string(),
                }),
                ValueType::ImageRgb8 => Err(ConversionError {
                    message: "Unable to convert image type to image.".to_string(),
                }),
                ValueType::ImageGrayA8 => Err(ConversionError {
                    message: "Unable to convert image type to image.".to_string(),
                }),
                ValueType::ImageGray8 => Err(ConversionError {
                    message: "Unable to convert image type to image.".to_string(),
                }),
                ValueType::FilterType => Err(ConversionError {
                    message: "Unable to convert image type to image.".to_string(),
                }),
                ValueType::ImageFormat => {
                    Ok(Value::ImageFormat(a.clone()))
                },
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
    FilterType,
    ImageFormat,
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