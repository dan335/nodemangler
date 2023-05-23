use image::{
    imageops::FilterType, DynamicImage, GrayAlphaImage, GrayImage, ImageBuffer, Luma, LumaA, Rgb,
    Rgb32FImage, RgbImage, Rgba, Rgba32FImage, RgbaImage,
};

pub const THUMBNAIL_SIZE: [u32; 2] = [128, 128];

#[derive(Debug, Clone)]
pub enum Value {
    Bool(bool),
    Integer(i32),
    Decimal(f32),
    String(String),
    Rgba32FImage(Rgba32FImage),
    Rgb32FImage(Rgb32FImage),
    Rgba16Image(ImageBuffer<Rgba<u16>, Vec<u16>>),
    Rgb16Image(ImageBuffer<Rgb<u16>, Vec<u16>>),
    GrayAlpha16Image(ImageBuffer<LumaA<u16>, Vec<u16>>),
    Gray16Image(ImageBuffer<Luma<u16>, Vec<u16>>),
    RgbaImage(RgbaImage),
    RgbImage(RgbImage),
    GrayAlphaImage(GrayAlphaImage),
    GrayImage(GrayImage),
    FilterType(FilterType),
    ImageFormat(ImageFormat),
    UiButton(bool),
}


impl Value {

    pub fn create_thumbnail(&self) -> Option<ImageBuffer<Rgba<u8>, Vec<u8>>> {
        match &self {
            Value::Rgba32FImage(value) => {
                Some(DynamicImage::ImageRgba32F(value.clone()).resize(THUMBNAIL_SIZE[0], THUMBNAIL_SIZE[1], image::imageops::FilterType::Triangle).to_rgba8())
            },
            Value::RgbaImage(value) => {
                Some(DynamicImage::ImageRgba8(value.clone()).resize(THUMBNAIL_SIZE[0], THUMBNAIL_SIZE[1], image::imageops::FilterType::Triangle).to_rgba8())
            },
            Value::GrayImage(value) => {
                Some(DynamicImage::ImageLuma8(value.clone()).resize(THUMBNAIL_SIZE[0], THUMBNAIL_SIZE[1], image::imageops::FilterType::Triangle).to_rgba8())
            },
            Value::Rgb32FImage(value) => {
                Some(DynamicImage::ImageRgb32F(value.clone()).resize(THUMBNAIL_SIZE[0], THUMBNAIL_SIZE[1], image::imageops::FilterType::Triangle).to_rgba8())
            },
            Value::Rgba16Image(value) => {
                Some(DynamicImage::ImageRgba16(value.clone()).resize(THUMBNAIL_SIZE[0], THUMBNAIL_SIZE[1], image::imageops::FilterType::Triangle).to_rgba8())
            },
            Value::Rgb16Image(value) => {
                Some(DynamicImage::ImageRgb16(value.clone()).resize(THUMBNAIL_SIZE[0], THUMBNAIL_SIZE[1], image::imageops::FilterType::Triangle).to_rgba8())
            },
            Value::GrayAlpha16Image(value) => {
                Some(DynamicImage::ImageLumaA16(value.clone()).resize(THUMBNAIL_SIZE[0], THUMBNAIL_SIZE[1], image::imageops::FilterType::Triangle).to_rgba8())
            },
            Value::Gray16Image(value) => {
                Some(DynamicImage::ImageLuma16(value.clone()).resize(THUMBNAIL_SIZE[0], THUMBNAIL_SIZE[1], image::imageops::FilterType::Triangle).to_rgba8())
            },
            Value::RgbImage(value) => {
                Some(DynamicImage::ImageRgb8(value.clone()).resize(THUMBNAIL_SIZE[0], THUMBNAIL_SIZE[1], image::imageops::FilterType::Triangle).to_rgba8())
            },
            Value::GrayAlphaImage(value) => {
                Some(DynamicImage::ImageLumaA8(value.clone()).resize(THUMBNAIL_SIZE[0], THUMBNAIL_SIZE[1], image::imageops::FilterType::Triangle).to_rgba8())
            },
            Value::Bool(_) |
            Value::Integer(_) |
            Value::Decimal(_) |
            Value::String(_) |
            Value::FilterType(_) |
            Value::ImageFormat(_) => {
                None
            },
            Value::UiButton(_) => todo!(),
        }
    }

    pub fn value_type(&self) -> ValueType {
        match self {
            Value::Bool(_) => ValueType::Bool,
            Value::Integer(_) => ValueType::Integer,
            Value::Decimal(_) => ValueType::Decimal,
            Value::String(_) => ValueType::String,
            Value::Rgba32FImage(_) => ValueType::Rgba32FImage,
            Value::RgbaImage(_) => ValueType::RgbaImage,
            Value::GrayImage(_) => ValueType::GrayImage,
            Value::FilterType(_) => ValueType::FilterType,
            Value::Rgb32FImage(_) => ValueType::Rgb32FImage,
            Value::Rgba16Image(_) => ValueType::Rgba16Image,
            Value::Rgb16Image(_) => ValueType::Rgb16Image,
            Value::GrayAlpha16Image(_) => ValueType::GrayAlpha16Image,
            Value::Gray16Image(_) => ValueType::Gray16Image,
            Value::RgbImage(_) => ValueType::RgbImage,
            Value::GrayAlphaImage(_) => ValueType::GrayAlphaImage,
            Value::ImageFormat(_) => ValueType::ImageFormat,
            Value::UiButton(_) => ValueType::UiButton,
        }
    }

    pub fn value_name(&self) -> String {        
        match self {
            Value::Bool(_) => "bool".to_string(),
            Value::Integer(_) => "integer".to_string(),
            Value::Decimal(_) => "decimal".to_string(),
            Value::String(_) => "string".to_string(),
            Value::Rgba32FImage(_) => "rgba 32f image".to_string(),
            Value::Rgb32FImage(_) => "rgb 32f image".to_string(),
            Value::Rgba16Image(_) => "rgba 16 image".to_string(),
            Value::Rgb16Image(_) => "rgb 16 image".to_string(),
            Value::GrayAlpha16Image(_) => "gray alpha 16 image".to_string(),
            Value::Gray16Image(_) => "gray 16 image".to_string(),
            Value::RgbaImage(_) => "rgba 8 image".to_string(),
            Value::RgbImage(_) => "rgb 8 image".to_string(),
            Value::GrayAlphaImage(_) => "gray alpha 8 image".to_string(),
            Value::GrayImage(_) => "gray 8 image".to_string(),
            Value::FilterType(_) => "filter type".to_string(),
            Value::ImageFormat(_) => "image format".to_string(),
            Value::UiButton(_) => "button".to_string(),
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
                ValueType::Rgba32FImage
                | ValueType::Rgb32FImage
                | ValueType::Rgba16Image
                | ValueType::Rgb16Image
                | ValueType::GrayAlpha16Image
                | ValueType::Gray16Image
                | ValueType::RgbaImage
                | ValueType::RgbImage
                | ValueType::GrayAlphaImage
                | ValueType::GrayImage => Err(ConversionError {
                    message: "Unable to convert bool to image.".to_string(),
                }),
                ValueType::FilterType => Err(ConversionError {
                    message: "Unable to convert bool to filter type.".to_string(),
                }),
                ValueType::ImageFormat => Err(ConversionError {
                    message: "Unable to convert bool to image format.".to_string(),
                }),
                ValueType::UiButton => todo!(),
            },
            Value::Integer(a) => match other {
                ValueType::Bool => Ok(Value::Bool(*a != 0)),
                ValueType::Integer => Ok(Value::Integer(*a)),
                ValueType::Decimal => Ok(Value::Decimal(*a as f32)),
                ValueType::String => Ok(Value::String(a.to_string())),
                ValueType::Rgba32FImage
                | ValueType::Rgb32FImage
                | ValueType::Rgba16Image
                | ValueType::Rgb16Image
                | ValueType::GrayAlpha16Image
                | ValueType::Gray16Image
                | ValueType::RgbaImage
                | ValueType::RgbImage
                | ValueType::GrayAlphaImage
                | ValueType::GrayImage => Err(ConversionError {
                    message: "Unable to convert integer to image.".to_string(),
                }),
                ValueType::FilterType => Err(ConversionError {
                    message: "Unable to convert bool to filter type.".to_string(),
                }),
                ValueType::ImageFormat => Err(ConversionError {
                    message: "Unable to convert integer to image format.".to_string(),
                }),
                ValueType::UiButton => todo!(),
            },
            Value::Decimal(a) => match other {
                ValueType::Bool => Ok(Value::Bool(*a != 0.0)),
                ValueType::Integer => Ok(Value::Integer(*a as i32)),
                ValueType::Decimal => Ok(Value::Decimal(*a)),
                ValueType::String => Ok(Value::String(a.to_string())),
                ValueType::Rgba32FImage
                | ValueType::Rgb32FImage
                | ValueType::Rgba16Image
                | ValueType::Rgb16Image
                | ValueType::GrayAlpha16Image
                | ValueType::Gray16Image
                | ValueType::RgbaImage
                | ValueType::RgbImage
                | ValueType::GrayAlphaImage
                | ValueType::GrayImage => Err(ConversionError {
                    message: "Unable to convert decimal to image.".to_string(),
                }),
                ValueType::FilterType => Err(ConversionError {
                    message: "Unable to convert bool to filter type.".to_string(),
                }),
                ValueType::ImageFormat => Err(ConversionError {
                    message: "Unable to convert decimal to image format.".to_string(),
                }),
                ValueType::UiButton => todo!(),
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
                ValueType::Rgba32FImage
                | ValueType::Rgb32FImage
                | ValueType::Rgba16Image
                | ValueType::Rgb16Image
                | ValueType::GrayAlpha16Image
                | ValueType::Gray16Image
                | ValueType::RgbaImage
                | ValueType::RgbImage
                | ValueType::GrayAlphaImage
                | ValueType::GrayImage => Err(ConversionError {
                    message: "Unable to convert string to image.".to_string(),
                }),
                ValueType::FilterType => Err(ConversionError {
                    message: "Unable to convert bool to filter type.".to_string(),
                }),
                ValueType::ImageFormat => Err(ConversionError {
                    message: "Unable to convert string to image format.".to_string(),
                }),
                ValueType::UiButton => todo!(),
            },
            Value::Rgba32FImage(a) => match other {
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
                ValueType::Rgba32FImage => Ok(Value::Rgba32FImage(a.clone())),
                ValueType::Rgb32FImage => Ok(Value::Rgb32FImage(
                    DynamicImage::ImageRgba32F(a.clone()).into_rgb32f(),
                )),
                ValueType::Rgba16Image => Ok(Value::Rgba16Image(
                    DynamicImage::ImageRgba32F(a.clone()).into_rgba16(),
                )),
                ValueType::Rgb16Image => Ok(Value::Rgb16Image(
                    DynamicImage::ImageRgba32F(a.clone()).into_rgb16(),
                )),
                ValueType::GrayAlpha16Image => Ok(Value::GrayAlpha16Image(
                    DynamicImage::ImageRgba32F(a.clone()).into_luma_alpha16(),
                )),
                ValueType::Gray16Image => Ok(Value::Gray16Image(
                    DynamicImage::ImageRgba32F(a.clone()).into_luma16(),
                )),
                ValueType::RgbaImage => Ok(Value::RgbaImage(
                    DynamicImage::ImageRgba32F(a.clone()).into_rgba8(),
                )),
                ValueType::RgbImage => Ok(Value::RgbImage(
                    DynamicImage::ImageRgba32F(a.clone()).into_rgb8(),
                )),
                ValueType::GrayAlphaImage => Ok(Value::GrayAlphaImage(
                    DynamicImage::ImageRgba32F(a.clone()).into_luma_alpha8(),
                )),
                ValueType::GrayImage => Ok(Value::GrayImage(
                    DynamicImage::ImageRgba32F(a.clone()).into_luma8(),
                )),
                ValueType::FilterType => Err(ConversionError {
                    message: "Unable to convert image to filter type.".to_string(),
                }),
                ValueType::ImageFormat => Ok(Value::ImageFormat(ImageFormat::ImageRgba32F)),
                ValueType::UiButton => todo!(),
            },
            Value::Rgb32FImage(a) => match other {
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
                ValueType::Rgba32FImage => Ok(Value::Rgba32FImage(
                    DynamicImage::ImageRgb32F(a.clone()).into_rgba32f(),
                )),
                ValueType::Rgb32FImage => Ok(Value::Rgb32FImage(a.clone())),
                ValueType::Rgba16Image => Ok(Value::Rgba16Image(
                    DynamicImage::ImageRgb32F(a.clone()).into_rgba16(),
                )),
                ValueType::Rgb16Image => Ok(Value::Rgb16Image(
                    DynamicImage::ImageRgb32F(a.clone()).into_rgb16(),
                )),
                ValueType::GrayAlpha16Image => Ok(Value::GrayAlpha16Image(
                    DynamicImage::ImageRgb32F(a.clone()).into_luma_alpha16(),
                )),
                ValueType::Gray16Image => Ok(Value::Gray16Image(
                    DynamicImage::ImageRgb32F(a.clone()).into_luma16(),
                )),
                ValueType::RgbaImage => Ok(Value::RgbaImage(
                    DynamicImage::ImageRgb32F(a.clone()).into_rgba8(),
                )),
                ValueType::RgbImage => Ok(Value::RgbImage(
                    DynamicImage::ImageRgb32F(a.clone()).into_rgb8(),
                )),
                ValueType::GrayAlphaImage => Ok(Value::GrayAlphaImage(
                    DynamicImage::ImageRgb32F(a.clone()).into_luma_alpha8(),
                )),
                ValueType::GrayImage => Ok(Value::GrayImage(
                    DynamicImage::ImageRgb32F(a.clone()).into_luma8(),
                )),
                ValueType::FilterType => Err(ConversionError {
                    message: "Unable to convert image to filter type.".to_string(),
                }),
                ValueType::ImageFormat => Ok(Value::ImageFormat(ImageFormat::ImageRgb32F)),
                ValueType::UiButton => todo!(),
            },
            Value::Rgba16Image(a) => match other {
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
                ValueType::Rgba32FImage => Ok(Value::Rgba32FImage(
                    DynamicImage::ImageRgba16(a.clone()).into_rgba32f(),
                )),
                ValueType::Rgb32FImage => Ok(Value::Rgb32FImage(
                    DynamicImage::ImageRgba16(a.clone()).into_rgb32f(),
                )),
                ValueType::Rgba16Image => Ok(Value::Rgba16Image(
                    DynamicImage::ImageRgba16(a.clone()).into_rgba16(),
                )),
                ValueType::Rgb16Image => Ok(Value::Rgb16Image(
                    DynamicImage::ImageRgba16(a.clone()).into_rgb16(),
                )),
                ValueType::GrayAlpha16Image => Ok(Value::GrayAlpha16Image(
                    DynamicImage::ImageRgba16(a.clone()).into_luma_alpha16(),
                )),
                ValueType::Gray16Image => Ok(Value::Gray16Image(
                    DynamicImage::ImageRgba16(a.clone()).into_luma16(),
                )),
                ValueType::RgbaImage => Ok(Value::RgbaImage(
                    DynamicImage::ImageRgba16(a.clone()).into_rgba8(),
                )),
                ValueType::RgbImage => Ok(Value::RgbImage(
                    DynamicImage::ImageRgba16(a.clone()).into_rgb8(),
                )),
                ValueType::GrayAlphaImage => Ok(Value::GrayAlphaImage(
                    DynamicImage::ImageRgba16(a.clone()).into_luma_alpha8(),
                )),
                ValueType::GrayImage => Ok(Value::GrayImage(
                    DynamicImage::ImageRgba16(a.clone()).into_luma8(),
                )),
                ValueType::FilterType => Err(ConversionError {
                    message: "Unable to convert image to filter type.".to_string(),
                }),
                ValueType::ImageFormat => Ok(Value::ImageFormat(ImageFormat::ImageRgba16)),
                ValueType::UiButton => todo!(),
            },
            Value::Rgb16Image(a) => match other {
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
                ValueType::Rgba32FImage => Ok(Value::Rgba32FImage(
                    DynamicImage::ImageRgb16(a.clone()).into_rgba32f(),
                )),
                ValueType::Rgb32FImage => Ok(Value::Rgb32FImage(
                    DynamicImage::ImageRgb16(a.clone()).into_rgb32f(),
                )),
                ValueType::Rgba16Image => Ok(Value::Rgba16Image(
                    DynamicImage::ImageRgb16(a.clone()).into_rgba16(),
                )),
                ValueType::Rgb16Image => Ok(Value::Rgb16Image(
                    DynamicImage::ImageRgb16(a.clone()).into_rgb16(),
                )),
                ValueType::GrayAlpha16Image => Ok(Value::GrayAlpha16Image(
                    DynamicImage::ImageRgb16(a.clone()).into_luma_alpha16(),
                )),
                ValueType::Gray16Image => Ok(Value::Gray16Image(
                    DynamicImage::ImageRgb16(a.clone()).into_luma16(),
                )),
                ValueType::RgbaImage => Ok(Value::RgbaImage(
                    DynamicImage::ImageRgb16(a.clone()).into_rgba8(),
                )),
                ValueType::RgbImage => Ok(Value::RgbImage(
                    DynamicImage::ImageRgb16(a.clone()).into_rgb8(),
                )),
                ValueType::GrayAlphaImage => Ok(Value::GrayAlphaImage(
                    DynamicImage::ImageRgb16(a.clone()).into_luma_alpha8(),
                )),
                ValueType::GrayImage => Ok(Value::GrayImage(
                    DynamicImage::ImageRgb16(a.clone()).into_luma8(),
                )),
                ValueType::FilterType => Err(ConversionError {
                    message: "Unable to convert image to filter type.".to_string(),
                }),
                ValueType::ImageFormat => Ok(Value::ImageFormat(ImageFormat::ImageRgb16)),
                ValueType::UiButton => todo!(),
            },
            Value::GrayAlpha16Image(a) => match other {
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
                ValueType::Rgba32FImage => Ok(Value::Rgba32FImage(
                    DynamicImage::ImageLumaA16(a.clone()).into_rgba32f(),
                )),
                ValueType::Rgb32FImage => Ok(Value::Rgb32FImage(
                    DynamicImage::ImageLumaA16(a.clone()).into_rgb32f(),
                )),
                ValueType::Rgba16Image => Ok(Value::Rgba16Image(
                    DynamicImage::ImageLumaA16(a.clone()).into_rgba16(),
                )),
                ValueType::Rgb16Image => Ok(Value::Rgb16Image(
                    DynamicImage::ImageLumaA16(a.clone()).into_rgb16(),
                )),
                ValueType::GrayAlpha16Image => Ok(Value::GrayAlpha16Image(
                    DynamicImage::ImageLumaA16(a.clone()).into_luma_alpha16(),
                )),
                ValueType::Gray16Image => Ok(Value::Gray16Image(
                    DynamicImage::ImageLumaA16(a.clone()).into_luma16(),
                )),
                ValueType::RgbaImage => Ok(Value::RgbaImage(
                    DynamicImage::ImageLumaA16(a.clone()).into_rgba8(),
                )),
                ValueType::RgbImage => Ok(Value::RgbImage(
                    DynamicImage::ImageLumaA16(a.clone()).into_rgb8(),
                )),
                ValueType::GrayAlphaImage => Ok(Value::GrayAlphaImage(
                    DynamicImage::ImageLumaA16(a.clone()).into_luma_alpha8(),
                )),
                ValueType::GrayImage => Ok(Value::GrayImage(
                    DynamicImage::ImageLumaA16(a.clone()).into_luma8(),
                )),
                ValueType::FilterType => Err(ConversionError {
                    message: "Unable to convert image to filter type.".to_string(),
                }),
                ValueType::ImageFormat => Ok(Value::ImageFormat(ImageFormat::ImageGrayA16)),
                ValueType::UiButton => todo!(),
            },
            Value::Gray16Image(a) => match other {
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
                ValueType::Rgba32FImage => Ok(Value::Rgba32FImage(
                    DynamicImage::ImageLuma16(a.clone()).into_rgba32f(),
                )),
                ValueType::Rgb32FImage => Ok(Value::Rgb32FImage(
                    DynamicImage::ImageLuma16(a.clone()).into_rgb32f(),
                )),
                ValueType::Rgba16Image => Ok(Value::Rgba16Image(
                    DynamicImage::ImageLuma16(a.clone()).into_rgba16(),
                )),
                ValueType::Rgb16Image => Ok(Value::Rgb16Image(
                    DynamicImage::ImageLuma16(a.clone()).into_rgb16(),
                )),
                ValueType::GrayAlpha16Image => Ok(Value::GrayAlpha16Image(
                    DynamicImage::ImageLuma16(a.clone()).into_luma_alpha16(),
                )),
                ValueType::Gray16Image => Ok(Value::Gray16Image(
                    DynamicImage::ImageLuma16(a.clone()).into_luma16(),
                )),
                ValueType::RgbaImage => Ok(Value::RgbaImage(
                    DynamicImage::ImageLuma16(a.clone()).into_rgba8(),
                )),
                ValueType::RgbImage => Ok(Value::RgbImage(
                    DynamicImage::ImageLuma16(a.clone()).into_rgb8(),
                )),
                ValueType::GrayAlphaImage => Ok(Value::GrayAlphaImage(
                    DynamicImage::ImageLuma16(a.clone()).into_luma_alpha8(),
                )),
                ValueType::GrayImage => Ok(Value::GrayImage(
                    DynamicImage::ImageLuma16(a.clone()).into_luma8(),
                )),
                ValueType::FilterType => Err(ConversionError {
                    message: "Unable to convert image to filter type.".to_string(),
                }),
                ValueType::ImageFormat => Ok(Value::ImageFormat(ImageFormat::ImageGray16)),
                ValueType::UiButton => todo!(),
            },
            Value::RgbaImage(a) => match other {
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
                ValueType::Rgba32FImage => Ok(Value::Rgba32FImage(
                    DynamicImage::ImageRgba8(a.clone()).into_rgba32f(),
                )),
                ValueType::Rgb32FImage => Ok(Value::Rgb32FImage(
                    DynamicImage::ImageRgba8(a.clone()).into_rgb32f(),
                )),
                ValueType::Rgba16Image => Ok(Value::Rgba16Image(
                    DynamicImage::ImageRgba8(a.clone()).into_rgba16(),
                )),
                ValueType::Rgb16Image => Ok(Value::Rgb16Image(
                    DynamicImage::ImageRgba8(a.clone()).into_rgb16(),
                )),
                ValueType::GrayAlpha16Image => Ok(Value::GrayAlpha16Image(
                    DynamicImage::ImageRgba8(a.clone()).into_luma_alpha16(),
                )),
                ValueType::Gray16Image => Ok(Value::Gray16Image(
                    DynamicImage::ImageRgba8(a.clone()).into_luma16(),
                )),
                ValueType::RgbaImage => Ok(Value::RgbaImage(
                    DynamicImage::ImageRgba8(a.clone()).into_rgba8(),
                )),
                ValueType::RgbImage => Ok(Value::RgbImage(
                    DynamicImage::ImageRgba8(a.clone()).into_rgb8(),
                )),
                ValueType::GrayAlphaImage => Ok(Value::GrayAlphaImage(
                    DynamicImage::ImageRgba8(a.clone()).into_luma_alpha8(),
                )),
                ValueType::GrayImage => Ok(Value::GrayImage(
                    DynamicImage::ImageRgba8(a.clone()).into_luma8(),
                )),
                ValueType::FilterType => Err(ConversionError {
                    message: "Unable to convert image to filter type.".to_string(),
                }),
                ValueType::ImageFormat => Ok(Value::ImageFormat(ImageFormat::ImageRgba8)),
                ValueType::UiButton => todo!(),
            },
            Value::RgbImage(a) => match other {
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
                ValueType::Rgba32FImage => Ok(Value::Rgba32FImage(
                    DynamicImage::ImageRgb8(a.clone()).into_rgba32f(),
                )),
                ValueType::Rgb32FImage => Ok(Value::Rgb32FImage(
                    DynamicImage::ImageRgb8(a.clone()).into_rgb32f(),
                )),
                ValueType::Rgba16Image => Ok(Value::Rgba16Image(
                    DynamicImage::ImageRgb8(a.clone()).into_rgba16(),
                )),
                ValueType::Rgb16Image => Ok(Value::Rgb16Image(
                    DynamicImage::ImageRgb8(a.clone()).into_rgb16(),
                )),
                ValueType::GrayAlpha16Image => Ok(Value::GrayAlpha16Image(
                    DynamicImage::ImageRgb8(a.clone()).into_luma_alpha16(),
                )),
                ValueType::Gray16Image => Ok(Value::Gray16Image(
                    DynamicImage::ImageRgb8(a.clone()).into_luma16(),
                )),
                ValueType::RgbaImage => Ok(Value::RgbaImage(
                    DynamicImage::ImageRgb8(a.clone()).into_rgba8(),
                )),
                ValueType::RgbImage => Ok(Value::RgbImage(
                    DynamicImage::ImageRgb8(a.clone()).into_rgb8(),
                )),
                ValueType::GrayAlphaImage => Ok(Value::GrayAlphaImage(
                    DynamicImage::ImageRgb8(a.clone()).into_luma_alpha8(),
                )),
                ValueType::GrayImage => Ok(Value::GrayImage(
                    DynamicImage::ImageRgb8(a.clone()).into_luma8(),
                )),
                ValueType::FilterType => Err(ConversionError {
                    message: "Unable to convert image to filter type.".to_string(),
                }),
                ValueType::ImageFormat => Ok(Value::ImageFormat(ImageFormat::ImageRgb8)),
                ValueType::UiButton => todo!(),
            },
            Value::GrayAlphaImage(a) => match other {
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
                ValueType::Rgba32FImage => Ok(Value::Rgba32FImage(
                    DynamicImage::ImageLumaA8(a.clone()).into_rgba32f(),
                )),
                ValueType::Rgb32FImage => Ok(Value::Rgb32FImage(
                    DynamicImage::ImageLumaA8(a.clone()).into_rgb32f(),
                )),
                ValueType::Rgba16Image => Ok(Value::Rgba16Image(
                    DynamicImage::ImageLumaA8(a.clone()).into_rgba16(),
                )),
                ValueType::Rgb16Image => Ok(Value::Rgb16Image(
                    DynamicImage::ImageLumaA8(a.clone()).into_rgb16(),
                )),
                ValueType::GrayAlpha16Image => Ok(Value::GrayAlpha16Image(
                    DynamicImage::ImageLumaA8(a.clone()).into_luma_alpha16(),
                )),
                ValueType::Gray16Image => Ok(Value::Gray16Image(
                    DynamicImage::ImageLumaA8(a.clone()).into_luma16(),
                )),
                ValueType::RgbaImage => Ok(Value::RgbaImage(
                    DynamicImage::ImageLumaA8(a.clone()).into_rgba8(),
                )),
                ValueType::RgbImage => Ok(Value::RgbImage(
                    DynamicImage::ImageLumaA8(a.clone()).into_rgb8(),
                )),
                ValueType::GrayAlphaImage => Ok(Value::GrayAlphaImage(
                    DynamicImage::ImageLumaA8(a.clone()).into_luma_alpha8(),
                )),
                ValueType::GrayImage => Ok(Value::GrayImage(
                    DynamicImage::ImageLumaA8(a.clone()).into_luma8(),
                )),
                ValueType::FilterType => Err(ConversionError {
                    message: "Unable to convert image to filter type.".to_string(),
                }),
                ValueType::ImageFormat => Ok(Value::ImageFormat(ImageFormat::ImageGrayA8)),
                ValueType::UiButton => todo!(),
            },
            Value::GrayImage(a) => match other {
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
                ValueType::Rgba32FImage => Ok(Value::Rgba32FImage(
                    DynamicImage::ImageLuma8(a.clone()).into_rgba32f(),
                )),
                ValueType::Rgb32FImage => Ok(Value::Rgb32FImage(
                    DynamicImage::ImageLuma8(a.clone()).into_rgb32f(),
                )),
                ValueType::Rgba16Image => Ok(Value::Rgba16Image(
                    DynamicImage::ImageLuma8(a.clone()).into_rgba16(),
                )),
                ValueType::Rgb16Image => Ok(Value::Rgb16Image(
                    DynamicImage::ImageLuma8(a.clone()).into_rgb16(),
                )),
                ValueType::GrayAlpha16Image => Ok(Value::GrayAlpha16Image(
                    DynamicImage::ImageLuma8(a.clone()).into_luma_alpha16(),
                )),
                ValueType::Gray16Image => Ok(Value::Gray16Image(
                    DynamicImage::ImageLuma8(a.clone()).into_luma16(),
                )),
                ValueType::RgbaImage => Ok(Value::RgbaImage(
                    DynamicImage::ImageLuma8(a.clone()).into_rgba8(),
                )),
                ValueType::RgbImage => Ok(Value::RgbImage(
                    DynamicImage::ImageLuma8(a.clone()).into_rgb8(),
                )),
                ValueType::GrayAlphaImage => Ok(Value::GrayAlphaImage(
                    DynamicImage::ImageLuma8(a.clone()).into_luma_alpha8(),
                )),
                ValueType::GrayImage => Ok(Value::GrayImage(
                    DynamicImage::ImageLuma8(a.clone()).into_luma8(),
                )),
                ValueType::FilterType => Err(ConversionError {
                    message: "Unable to convert image to filter type.".to_string(),
                }),
                ValueType::ImageFormat => Ok(Value::ImageFormat(ImageFormat::ImageGray8)),
                ValueType::UiButton => todo!(),
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
                ValueType::Rgba32FImage => todo!(),
                ValueType::Rgb32FImage => todo!(),
                ValueType::Rgba16Image => todo!(),
                ValueType::Rgb16Image => todo!(),
                ValueType::GrayAlpha16Image => todo!(),
                ValueType::Gray16Image => todo!(),
                ValueType::RgbaImage => todo!(),
                ValueType::RgbImage => todo!(),
                ValueType::GrayAlphaImage => todo!(),
                ValueType::GrayImage => todo!(),
                ValueType::FilterType => todo!(),
                ValueType::ImageFormat => Err(ConversionError {
                    message: "Unable to convert filter type to image format.".to_string(),
                }),
                ValueType::UiButton => todo!(),
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
                ValueType::Rgba32FImage => Err(ConversionError {
                    message: "Unable to convert image type to image.".to_string(),
                }),
                ValueType::Rgb32FImage => Err(ConversionError {
                    message: "Unable to convert image type to image.".to_string(),
                }),
                ValueType::Rgba16Image => Err(ConversionError {
                    message: "Unable to convert image type to image.".to_string(),
                }),
                ValueType::Rgb16Image => Err(ConversionError {
                    message: "Unable to convert image type to image.".to_string(),
                }),
                ValueType::GrayAlpha16Image => Err(ConversionError {
                    message: "Unable to convert image type to image.".to_string(),
                }),
                ValueType::Gray16Image => Err(ConversionError {
                    message: "Unable to convert image type to image.".to_string(),
                }),
                ValueType::RgbaImage => Err(ConversionError {
                    message: "Unable to convert image type to image.".to_string(),
                }),
                ValueType::RgbImage => Err(ConversionError {
                    message: "Unable to convert image type to image.".to_string(),
                }),
                ValueType::GrayAlphaImage => Err(ConversionError {
                    message: "Unable to convert image type to image.".to_string(),
                }),
                ValueType::GrayImage => Err(ConversionError {
                    message: "Unable to convert image type to image.".to_string(),
                }),
                ValueType::FilterType => Err(ConversionError {
                    message: "Unable to convert image type to image.".to_string(),
                }),
                ValueType::ImageFormat => Ok(Value::ImageFormat(a.clone())),
                ValueType::UiButton => todo!(),
            },
            Value::UiButton(_) => todo!(),
        }
    }
}

#[derive(Debug, Clone)]
pub enum ValueType {
    Bool,
    Integer,
    Decimal,
    String,
    Rgba32FImage,
    Rgb32FImage,
    Rgba16Image,
    Rgb16Image,
    GrayAlpha16Image,
    Gray16Image,
    RgbaImage,
    RgbImage,
    GrayAlphaImage,
    GrayImage,
    FilterType,
    ImageFormat,
    UiButton,
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