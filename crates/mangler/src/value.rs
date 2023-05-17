use std::panic;

use image::{
    imageops::FilterType, GrayAlphaImage, GrayImage, ImageBuffer, Luma, LumaA, Rgb, Rgb32FImage,
    RgbImage, Rgba, Rgba32FImage, RgbaImage,
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
        }
    }

    pub fn convert_to(&self, other: ValueType) -> Result<Value, ConvertError> {
        match self {
            Value::Bool(a) => match other {},
        }

        // match (self, other) {
        //     (Value::Integer(a), ValueType::Integer) => Ok(Value::Integer(*a)),
        //     (Value::Bool(a), ValueType::Bool) => Ok(Value::Bool(*a)),
        //     (Value::Bool(a), ValueType::Integer) => {
        //         if *a {
        //             Ok(Value::Integer(1))
        //         } else {
        //             Ok(Value::Integer(0))
        //         }
        //     }
        //     (Value::Bool(a), ValueType::Decimal) => {
        //         if *a {
        //             Ok(Value::Decimal(1.0))
        //         } else {
        //             Ok(Value::Decimal(0.0))
        //         }
        //     }
        //     (Value::Bool(a), ValueType::String) => Ok(Value::String(a.to_string())),
        //     (Value::Bool(_), ValueType::ImageRgba32F) => Err(ConvertError {
        //         message: "Unable to convert bool to image.".to_string(),
        //     }),
        //     (Value::Bool(_), ValueType::ImageRgba8) => Err(ConvertError {
        //         message: "Unable to convert bool to image.".to_string(),
        //     }),
        //     (Value::Bool(_), ValueType::ImageGray8) => Err(ConvertError {
        //         message: "Unable to convert bool to image.".to_string(),
        //     }),
        //     (Value::Bool(_), ValueType::FilterType) => Err(ConvertError {
        //         message: "Unable to convert bool to image.".to_string(),
        //     }),
        //     (Value::Integer(a), ValueType::Bool) => Ok(Value::Bool(*a != 0)),
        //     (Value::Integer(a), ValueType::Decimal) => Ok(Value::Decimal(*a as f32)),
        //     (Value::Integer(a), ValueType::String) => Ok(Value::String(a.to_string())),
        //     (Value::Integer(_), ValueType::ImageRgba32F) => Err(ConvertError {
        //         message: "Unable to convert integer to image.".to_string(),
        //     }),
        //     (Value::Integer(_), ValueType::ImageRgba8) => Err(ConvertError {
        //         message: "Unable to convert integer to image.".to_string(),
        //     }),
        //     (Value::Integer(_), ValueType::ImageGray8) => Err(ConvertError {
        //         message: "Unable to convert integer to image.".to_string(),
        //     }),
        //     (Value::Integer(a), ValueType::FilterType) => match a {
        //         0 => Ok(Value::FilterType(FilterType::Nearest)),
        //         0 => Ok(Value::FilterType(FilterType::Triangle)),
        //         0 => Ok(Value::FilterType(FilterType::CatmullRom)),
        //         0 => Ok(Value::FilterType(FilterType::Gaussian)),
        //         0 => Ok(Value::FilterType(FilterType::Lanczos3)),
        //         _ => Err(ConvertError {
        //             message: "Index for FilterType beyond bounds.".to_string(),
        //         }),
        //     },
        //     (Value::Decimal(a), ValueType::Bool) => Ok(Value::Bool(*a != 0.0)),
        //     (Value::Decimal(a), ValueType::Integer) => Ok(Value::Integer(*a as i32)),
        //     (Value::Decimal(a), ValueType::Decimal) => Ok(Value::Decimal(*a)),
        //     (Value::Decimal(a), ValueType::String) => Ok(Value::String(a.to_string())),
        //     (Value::Decimal(_), ValueType::ImageRgba32F) => Err(ConvertError {
        //         message: "Unable to convert decimal to image.".to_string(),
        //     }),
        //     (Value::Decimal(_), ValueType::ImageRgba8) => Err(ConvertError {
        //         message: "Unable to convert decimal to image.".to_string(),
        //     }),
        //     (Value::Decimal(_), ValueType::ImageGray8) => Err(ConvertError {
        //         message: "Unable to convert decimal to image.".to_string(),
        //     }),
        //     (Value::Decimal(_), ValueType::FilterType) => Err(ConvertError {
        //         message: "Unable to convert decimal to filter type.".to_string(),
        //     }),
        //     (Value::String(a), ValueType::Bool) => {
        //         let result: Result<bool, _> = a.parse();
        //         match result {
        //             Ok(r) => Ok(Value::Bool(r)),
        //             Err(_) => Err(ConvertError {
        //                 message: "Error converting string to bool.".to_string(),
        //             }),
        //         }
        //     }
        //     (Value::String(a), ValueType::Integer) => {
        //         let result: Result<i32, _> = a.parse();
        //         match result {
        //             Ok(r) => Ok(Value::Integer(r)),
        //             Err(_) => Err(ConvertError {
        //                 message: "Error converting string to integer.".to_string(),
        //             }),
        //         }
        //     }
        //     (Value::String(a), ValueType::Decimal) => {
        //         let result: Result<f32, _> = a.parse();
        //         match result {
        //             Ok(r) => Ok(Value::Decimal(r)),
        //             Err(_) => Err(ConvertError {
        //                 message: "Error converting string to decimal.".to_string(),
        //             }),
        //         }
        //     }
        //     (Value::String(a), ValueType::String) => Ok(Value::String(a.clone())),
        //     (Value::String(_), ValueType::ImageRgba32F) => Err(ConvertError {
        //         message: "Unable to convert string to image.".to_string(),
        //     }),
        //     (Value::String(_), ValueType::ImageRgba8) => Err(ConvertError {
        //         message: "Unable to convert string to image.".to_string(),
        //     }),
        //     (Value::String(_), ValueType::ImageGray8) => Err(ConvertError {
        //         message: "Unable to convert string to image.".to_string(),
        //     }),
        //     // todo: improve this
        //     (Value::String(_), ValueType::FilterType) => Err(ConvertError {
        //         message: "Unable to convert string to filter type.".to_string(),
        //     }),
        //     (Value::ImageRgba32F(_), ValueType::Bool) => Err(ConvertError {
        //         message: "Unable to convert image to bool.".to_string(),
        //     }),
        //     (Value::ImageRgba32F(_), ValueType::Integer) => Err(ConvertError {
        //         message: "Unable to convert image to integer.".to_string(),
        //     }),
        //     (Value::ImageRgba32F(_), ValueType::Decimal) => Err(ConvertError {
        //         message: "Unable to convert image to decimal.".to_string(),
        //     }),
        //     (Value::ImageRgba32F(_), ValueType::String) => Err(ConvertError {
        //         message: "Unable to convert image to string.".to_string(),
        //     }),
        //     (Value::ImageRgba32F(_), ValueType::ImageRgba32F) => todo!(),
        //     (Value::ImageRgba32F(_), ValueType::ImageRgba8) => todo!(),
        //     (Value::ImageRgba32F(_), ValueType::ImageGray8) => todo!(),
        //     (Value::ImageRgba32F(_), ValueType::FilterType) => Err(ConvertError {
        //         message: "Unable to convert image to filter type.".to_string(),
        //     }),
        //     (Value::ImageRgb32F(_), ValueType::Bool) => Err(ConvertError {
        //         message: "Unable to convert image to bool.".to_string(),
        //     }),
        //     (Value::ImageRgb32F(_), ValueType::Integer) => Err(ConvertError {
        //         message: "Unable to convert image to integer.".to_string(),
        //     }),
        //     (Value::ImageRgb32F(_), ValueType::Decimal) => Err(ConvertError {
        //         message: "Unable to convert image to decimal.".to_string(),
        //     }),
        //     (Value::ImageRgb32F(_), ValueType::String) => Err(ConvertError {
        //         message: "Unable to convert image to string.".to_string(),
        //     }),
        //     (Value::ImageRgb32F(_), ValueType::ImageRgba32F) => todo!(),
        //     (Value::ImageRgb32F(_), ValueType::ImageRgba8) => todo!(),
        //     (Value::ImageRgb32F(_), ValueType::ImageGray8) => todo!(),
        //     (Value::ImageRgb32F(_), ValueType::FilterType) => Err(ConvertError {
        //         message: "Unable to convert image to filter type.".to_string(),
        //     }),
        //     (Value::ImageRgba16(_), ValueType::Bool) => Err(ConvertError {
        //         message: "Unable to convert image to bool.".to_string(),
        //     }),
        //     (Value::ImageRgba16(_), ValueType::Integer) => Err(ConvertError {
        //         message: "Unable to convert image to integer.".to_string(),
        //     }),
        //     (Value::ImageRgba16(_), ValueType::Decimal) => Err(ConvertError {
        //         message: "Unable to convert image to decimal.".to_string(),
        //     }),
        //     (Value::ImageRgba16(_), ValueType::String) => Err(ConvertError {
        //         message: "Unable to convert image to string.".to_string(),
        //     }),
        //     (Value::ImageRgba16(_), ValueType::ImageRgba32F) => todo!(),
        //     (Value::ImageRgba16(_), ValueType::ImageRgba8) => todo!(),
        //     (Value::ImageRgba16(_), ValueType::ImageGray8) => todo!(),
        //     (Value::ImageRgba16(_), ValueType::FilterType) => Err(ConvertError {
        //         message: "Unable to convert image to filter type.".to_string(),
        //     }),
        //     (Value::ImageRgb16(_), ValueType::Bool) => Err(ConvertError {
        //         message: "Unable to convert image to bool.".to_string(),
        //     }),
        //     (Value::ImageRgb16(_), ValueType::Integer) => Err(ConvertError {
        //         message: "Unable to convert image to integer.".to_string(),
        //     }),
        //     (Value::ImageRgb16(_), ValueType::Decimal) => Err(ConvertError {
        //         message: "Unable to convert image to decimal.".to_string(),
        //     }),
        //     (Value::ImageRgb16(_), ValueType::String) => Err(ConvertError {
        //         message: "Unable to convert image to string.".to_string(),
        //     }),
        //     (Value::ImageRgb16(_), ValueType::ImageRgba32F) => todo!(),
        //     (Value::ImageRgb16(_), ValueType::ImageRgba8) => todo!(),
        //     (Value::ImageRgb16(_), ValueType::ImageGray8) => todo!(),
        //     (Value::ImageRgb16(_), ValueType::FilterType) => Err(ConvertError {
        //         message: "Unable to convert image to filter type.".to_string(),
        //     }),
        //     (Value::ImageGrayA16(_), ValueType::Bool) => Err(ConvertError {
        //         message: "Unable to convert image to bool.".to_string(),
        //     }),
        //     (Value::ImageGrayA16(_), ValueType::Integer) => Err(ConvertError {
        //         message: "Unable to convert image to integer.".to_string(),
        //     }),
        //     (Value::ImageGrayA16(_), ValueType::Decimal) => Err(ConvertError {
        //         message: "Unable to convert image to decimal.".to_string(),
        //     }),
        //     (Value::ImageGrayA16(_), ValueType::String) => Err(ConvertError {
        //         message: "Unable to convert image to string.".to_string(),
        //     }),
        //     (Value::ImageGrayA16(_), ValueType::ImageRgba32F) => todo!(),
        //     (Value::ImageGrayA16(_), ValueType::ImageRgba8) => todo!(),
        //     (Value::ImageGrayA16(_), ValueType::ImageGray8) => todo!(),
        //     (Value::ImageGrayA16(_), ValueType::FilterType) => Err(ConvertError {
        //         message: "Unable to convert image to filter type.".to_string(),
        //     }),
        //     (Value::ImageGray16(_), ValueType::Bool) => Err(ConvertError {
        //         message: "Unable to convert image to bool.".to_string(),
        //     }),
        //     (Value::ImageGray16(_), ValueType::Integer) => Err(ConvertError {
        //         message: "Unable to convert image to integer.".to_string(),
        //     }),
        //     (Value::ImageGray16(_), ValueType::Decimal) => Err(ConvertError {
        //         message: "Unable to convert image to decimal.".to_string(),
        //     }),
        //     (Value::ImageGray16(_), ValueType::String) => Err(ConvertError {
        //         message: "Unable to convert image to string.".to_string(),
        //     }),
        //     (Value::ImageGray16(_), ValueType::ImageRgba32F) => todo!(),
        //     (Value::ImageGray16(_), ValueType::ImageRgba8) => todo!(),
        //     (Value::ImageGray16(_), ValueType::ImageGray8) => todo!(),
        //     (Value::ImageGray16(_), ValueType::FilterType) => Err(ConvertError {
        //         message: "Unable to convert image to filter type.".to_string(),
        //     }),
        //     (Value::ImageRgba8(_), ValueType::Bool) => Err(ConvertError {
        //         message: "Unable to convert image to bool.".to_string(),
        //     }),
        //     (Value::ImageRgba8(_), ValueType::Integer) => Err(ConvertError {
        //         message: "Unable to convert image to integer.".to_string(),
        //     }),
        //     (Value::ImageRgba8(_), ValueType::Decimal) => Err(ConvertError {
        //         message: "Unable to convert image to decimal.".to_string(),
        //     }),
        //     (Value::ImageRgba8(_), ValueType::String) => Err(ConvertError {
        //         message: "Unable to convert image to string.".to_string(),
        //     }),
        //     (Value::ImageRgba8(_), ValueType::ImageRgba32F) => todo!(),
        //     (Value::ImageRgba8(_), ValueType::ImageRgba8) => todo!(),
        //     (Value::ImageRgba8(_), ValueType::ImageGray8) => todo!(),
        //     (Value::ImageRgba8(_), ValueType::FilterType) => Err(ConvertError {
        //         message: "Unable to convert image to filter type.".to_string(),
        //     }),
        //     (Value::ImageRgb8(_), ValueType::Bool) => Err(ConvertError {
        //         message: "Unable to convert image to bool.".to_string(),
        //     }),
        //     (Value::ImageRgb8(_), ValueType::Integer) => Err(ConvertError {
        //         message: "Unable to convert image to integer.".to_string(),
        //     }),
        //     (Value::ImageRgb8(_), ValueType::Decimal) => Err(ConvertError {
        //         message: "Unable to convert image to decimal.".to_string(),
        //     }),
        //     (Value::ImageRgb8(_), ValueType::String) => Err(ConvertError {
        //         message: "Unable to convert image to string.".to_string(),
        //     }),
        //     (Value::ImageRgb8(_), ValueType::ImageRgba32F) => todo!(),
        //     (Value::ImageRgb8(_), ValueType::ImageRgba8) => todo!(),
        //     (Value::ImageRgb8(_), ValueType::ImageGray8) => todo!(),
        //     (Value::ImageRgb8(_), ValueType::FilterType) => Err(ConvertError {
        //         message: "Unable to convert image to filter type.".to_string(),
        //     }),
        //     (Value::ImageGrayA8(_), ValueType::Bool) => Err(ConvertError {
        //         message: "Unable to convert image to bool.".to_string(),
        //     }),
        //     (Value::ImageGrayA8(_), ValueType::Integer) => Err(ConvertError {
        //         message: "Unable to convert image to integer.".to_string(),
        //     }),
        //     (Value::ImageGrayA8(_), ValueType::Decimal) => Err(ConvertError {
        //         message: "Unable to convert image to decimal.".to_string(),
        //     }),
        //     (Value::ImageGrayA8(_), ValueType::String) => Err(ConvertError {
        //         message: "Unable to convert image to string.".to_string(),
        //     }),
        //     (Value::ImageGrayA8(_), ValueType::ImageRgba32F) => todo!(),
        //     (Value::ImageGrayA8(_), ValueType::ImageRgba8) => todo!(),
        //     (Value::ImageGrayA8(_), ValueType::ImageGray8) => todo!(),
        //     (Value::ImageGrayA8(_), ValueType::FilterType) => Err(ConvertError {
        //         message: "Unable to convert image to filter type.".to_string(),
        //     }),
        //     (Value::ImageGray8(_), ValueType::Bool) => Err(ConvertError {
        //         message: "Unable to convert image to bool.".to_string(),
        //     }),
        //     (Value::ImageGray8(_), ValueType::Integer) => Err(ConvertError {
        //         message: "Unable to convert image to integer.".to_string(),
        //     }),
        //     (Value::ImageGray8(_), ValueType::Decimal) => Err(ConvertError {
        //         message: "Unable to convert image to decimal.".to_string(),
        //     }),
        //     (Value::ImageGray8(_), ValueType::String) => Err(ConvertError {
        //         message: "Unable to convert image to string.".to_string(),
        //     }),
        //     (Value::ImageGray8(_), ValueType::ImageRgba32F) => todo!(),
        //     (Value::ImageGray8(_), ValueType::ImageRgba8) => todo!(),
        //     (Value::ImageGray8(_), ValueType::ImageGray8) => todo!(),
        //     (Value::ImageGray8(_), ValueType::FilterType) => Err(ConvertError {
        //         message: "Unable to convert image to filter type.".to_string(),
        //     }),
        //     (Value::FilterType(_), ValueType::Bool) => Err(ConvertError {
        //         message: "Unable to convert filter type to bool.".to_string(),
        //     }),
        //     // todo: improve this
        //     (Value::FilterType(_), ValueType::Integer) => Err(ConvertError {
        //         message: "Unable to convert filter type to bool.".to_string(),
        //     }),
        //     (Value::FilterType(_), ValueType::Decimal) => Err(ConvertError {
        //         message: "Unable to convert filter type to bool.".to_string(),
        //     }),
        //     (Value::FilterType(_), ValueType::String) => todo!(),
        //     (Value::FilterType(_), ValueType::ImageRgba32F) => todo!(),
        //     (Value::FilterType(_), ValueType::ImageRgba8) => todo!(),
        //     (Value::FilterType(_), ValueType::ImageGray8) => todo!(),
        //     (Value::FilterType(a), ValueType::FilterType) => Ok(Value::FilterType(*a)),
        // }
    }

    pub fn show_editable_property(value: &mut Value, ui: &mut egui::Ui) {
        match value.clone().value_type() {}
    }

    pub fn show_uneditable_property(value: &Value, ui: &mut egui::Ui) {
        match value.clone().value_type() {}
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
}

#[derive(Debug)]
pub struct ConvertError {
    pub message: String,
}
