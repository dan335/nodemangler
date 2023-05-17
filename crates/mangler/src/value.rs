use std::panic;

use image::{imageops::FilterType, GrayImage, Rgba32FImage, RgbaImage, ImageBuffer, Rgb32FImage, Rgba, Rgb, LumaA, Luma, RgbImage, GrayAlphaImage};

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
            Value::ImageRgb32F(_) => todo!(),
            Value::ImageRgba16(_) => todo!(),
            Value::ImageRgb16(_) => todo!(),
            Value::ImageGrayA16(_) => todo!(),
            Value::ImageGray16(_) => todo!(),
            Value::ImageRgb8(_) => todo!(),
            Value::ImageGrayA8(_) => todo!(), // filter type for resizing images
        }
    }

    pub fn convert(&self, other: &Value) -> Value {
        let result = match (self, other.value_type()) {
            (Value::Integer(a), ValueType::Integer) => {
                Value::Integer(*a)
            }
            (Value::Bool(a), ValueType::Bool) => {
                Value::Bool(*a)
            },
            (Value::Bool(a), ValueType::Integer) => {
                if *a { Value::Integer(1) } else { Value::Integer(0) }
            },
            (Value::Bool(a), ValueType::Decimal) => {
                if *a { Value::Decimal(1.0) } else { Value::Decimal(0.0) }
            },
            (Value::Bool(a), ValueType::String) => {
                Value::String(a.to_string())
            },
            (Value::Bool(_), ValueType::ImageRgba32F) => panic!("Unable to convert."),
            (Value::Bool(_), ValueType::ImageRgba8) => panic!("Unable to convert."),
            (Value::Bool(_), ValueType::ImageGray8) => panic!("Unable to convert."),
            (Value::Bool(_), ValueType::FilterType) => panic!("Unable to convert."),
            (Value::Integer(a), ValueType::Bool) => {
                Value::Bool(*a != 0)
            },
            (Value::Integer(a), ValueType::Decimal) => {
                Value::Decimal(*a as f32)
            },
            (Value::Integer(a), ValueType::String) => {

            },
            (Value::Integer(_), ValueType::ImageRgba32F) => panic!("Unable to convert."),
            (Value::Integer(_), ValueType::ImageRgba8) => panic!("Unable to convert."),
            (Value::Integer(_), ValueType::ImageGray8) => panic!("Unable to convert."),
            (Value::Integer(a), ValueType::FilterType) => {

            },
            (Value::Decimal(_), ValueType::Bool) => todo!(),
            (Value::Decimal(_), ValueType::Integer) => todo!(),
            (Value::Decimal(_), ValueType::Decimal) => todo!(),
            (Value::Decimal(_), ValueType::String) => todo!(),
            (Value::Decimal(_), ValueType::ImageRgba32F) => todo!(),
            (Value::Decimal(_), ValueType::ImageRgba8) => todo!(),
            (Value::Decimal(_), ValueType::ImageGray8) => todo!(),
            (Value::Decimal(_), ValueType::FilterType) => todo!(),
            (Value::String(_), ValueType::Bool) => todo!(),
            (Value::String(_), ValueType::Integer) => todo!(),
            (Value::String(_), ValueType::Decimal) => todo!(),
            (Value::String(_), ValueType::String) => todo!(),
            (Value::String(_), ValueType::ImageRgba32F) => todo!(),
            (Value::String(_), ValueType::ImageRgba8) => todo!(),
            (Value::String(_), ValueType::ImageGray8) => todo!(),
            (Value::String(_), ValueType::FilterType) => todo!(),
            (Value::ImageRgba32F(_), ValueType::Bool) => todo!(),
            (Value::ImageRgba32F(_), ValueType::Integer) => todo!(),
            (Value::ImageRgba32F(_), ValueType::Decimal) => todo!(),
            (Value::ImageRgba32F(_), ValueType::String) => todo!(),
            (Value::ImageRgba32F(_), ValueType::ImageRgba32F) => todo!(),
            (Value::ImageRgba32F(_), ValueType::ImageRgba8) => todo!(),
            (Value::ImageRgba32F(_), ValueType::ImageGray8) => todo!(),
            (Value::ImageRgba32F(_), ValueType::FilterType) => todo!(),
            (Value::ImageRgb32F(_), ValueType::Bool) => todo!(),
            (Value::ImageRgb32F(_), ValueType::Integer) => todo!(),
            (Value::ImageRgb32F(_), ValueType::Decimal) => todo!(),
            (Value::ImageRgb32F(_), ValueType::String) => todo!(),
            (Value::ImageRgb32F(_), ValueType::ImageRgba32F) => todo!(),
            (Value::ImageRgb32F(_), ValueType::ImageRgba8) => todo!(),
            (Value::ImageRgb32F(_), ValueType::ImageGray8) => todo!(),
            (Value::ImageRgb32F(_), ValueType::FilterType) => todo!(),
            (Value::ImageRgba16(_), ValueType::Bool) => todo!(),
            (Value::ImageRgba16(_), ValueType::Integer) => todo!(),
            (Value::ImageRgba16(_), ValueType::Decimal) => todo!(),
            (Value::ImageRgba16(_), ValueType::String) => todo!(),
            (Value::ImageRgba16(_), ValueType::ImageRgba32F) => todo!(),
            (Value::ImageRgba16(_), ValueType::ImageRgba8) => todo!(),
            (Value::ImageRgba16(_), ValueType::ImageGray8) => todo!(),
            (Value::ImageRgba16(_), ValueType::FilterType) => todo!(),
            (Value::ImageRgb16(_), ValueType::Bool) => todo!(),
            (Value::ImageRgb16(_), ValueType::Integer) => todo!(),
            (Value::ImageRgb16(_), ValueType::Decimal) => todo!(),
            (Value::ImageRgb16(_), ValueType::String) => todo!(),
            (Value::ImageRgb16(_), ValueType::ImageRgba32F) => todo!(),
            (Value::ImageRgb16(_), ValueType::ImageRgba8) => todo!(),
            (Value::ImageRgb16(_), ValueType::ImageGray8) => todo!(),
            (Value::ImageRgb16(_), ValueType::FilterType) => todo!(),
            (Value::ImageGrayA16(_), ValueType::Bool) => todo!(),
            (Value::ImageGrayA16(_), ValueType::Integer) => todo!(),
            (Value::ImageGrayA16(_), ValueType::Decimal) => todo!(),
            (Value::ImageGrayA16(_), ValueType::String) => todo!(),
            (Value::ImageGrayA16(_), ValueType::ImageRgba32F) => todo!(),
            (Value::ImageGrayA16(_), ValueType::ImageRgba8) => todo!(),
            (Value::ImageGrayA16(_), ValueType::ImageGray8) => todo!(),
            (Value::ImageGrayA16(_), ValueType::FilterType) => todo!(),
            (Value::ImageGray16(_), ValueType::Bool) => todo!(),
            (Value::ImageGray16(_), ValueType::Integer) => todo!(),
            (Value::ImageGray16(_), ValueType::Decimal) => todo!(),
            (Value::ImageGray16(_), ValueType::String) => todo!(),
            (Value::ImageGray16(_), ValueType::ImageRgba32F) => todo!(),
            (Value::ImageGray16(_), ValueType::ImageRgba8) => todo!(),
            (Value::ImageGray16(_), ValueType::ImageGray8) => todo!(),
            (Value::ImageGray16(_), ValueType::FilterType) => todo!(),
            (Value::ImageRgba8(_), ValueType::Bool) => todo!(),
            (Value::ImageRgba8(_), ValueType::Integer) => todo!(),
            (Value::ImageRgba8(_), ValueType::Decimal) => todo!(),
            (Value::ImageRgba8(_), ValueType::String) => todo!(),
            (Value::ImageRgba8(_), ValueType::ImageRgba32F) => todo!(),
            (Value::ImageRgba8(_), ValueType::ImageRgba8) => todo!(),
            (Value::ImageRgba8(_), ValueType::ImageGray8) => todo!(),
            (Value::ImageRgba8(_), ValueType::FilterType) => todo!(),
            (Value::ImageRgb8(_), ValueType::Bool) => todo!(),
            (Value::ImageRgb8(_), ValueType::Integer) => todo!(),
            (Value::ImageRgb8(_), ValueType::Decimal) => todo!(),
            (Value::ImageRgb8(_), ValueType::String) => todo!(),
            (Value::ImageRgb8(_), ValueType::ImageRgba32F) => todo!(),
            (Value::ImageRgb8(_), ValueType::ImageRgba8) => todo!(),
            (Value::ImageRgb8(_), ValueType::ImageGray8) => todo!(),
            (Value::ImageRgb8(_), ValueType::FilterType) => todo!(),
            (Value::ImageGrayA8(_), ValueType::Bool) => todo!(),
            (Value::ImageGrayA8(_), ValueType::Integer) => todo!(),
            (Value::ImageGrayA8(_), ValueType::Decimal) => todo!(),
            (Value::ImageGrayA8(_), ValueType::String) => todo!(),
            (Value::ImageGrayA8(_), ValueType::ImageRgba32F) => todo!(),
            (Value::ImageGrayA8(_), ValueType::ImageRgba8) => todo!(),
            (Value::ImageGrayA8(_), ValueType::ImageGray8) => todo!(),
            (Value::ImageGrayA8(_), ValueType::FilterType) => todo!(),
            (Value::ImageGray8(_), ValueType::Bool) => todo!(),
            (Value::ImageGray8(_), ValueType::Integer) => todo!(),
            (Value::ImageGray8(_), ValueType::Decimal) => todo!(),
            (Value::ImageGray8(_), ValueType::String) => todo!(),
            (Value::ImageGray8(_), ValueType::ImageRgba32F) => todo!(),
            (Value::ImageGray8(_), ValueType::ImageRgba8) => todo!(),
            (Value::ImageGray8(_), ValueType::ImageGray8) => todo!(),
            (Value::ImageGray8(_), ValueType::FilterType) => todo!(),
            (Value::FilterType(_), ValueType::Bool) => todo!(),
            (Value::FilterType(_), ValueType::Integer) => todo!(),
            (Value::FilterType(_), ValueType::Decimal) => todo!(),
            (Value::FilterType(_), ValueType::String) => todo!(),
            (Value::FilterType(_), ValueType::ImageRgba32F) => todo!(),
            (Value::FilterType(_), ValueType::ImageRgba8) => todo!(),
            (Value::FilterType(_), ValueType::ImageGray8) => todo!(),
            (Value::FilterType(_), ValueType::FilterType) => todo!(),
        }

        result
    }
}

#[derive(Debug, Clone)]
pub enum ValueType {
    Bool,
    Integer,
    Decimal,
    String,
    ImageRgba32F,
    ImageRgba8,
    ImageGray8,
    FilterType,
}
