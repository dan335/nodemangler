use image::{Rgba32FImage, RgbaImage, GrayImage };

#[derive(Debug, Clone)]
pub enum Value {
    Integer(i32),
    Decimal(f32),
    String(String),
    ImageRgba32F(Rgba32FImage),
    ImageRgba8(RgbaImage),
    ImageGray8(GrayImage),
}

impl Value {
    pub fn value_type(self) -> ValueType {
        match self {
            Value::Integer(_) => ValueType::Integer,
            Value::Decimal(_) => ValueType::Decimal,
            Value::String(_) => ValueType::String,
            Value::ImageRgba32F(_) => ValueType::ImageRgba32F,
            Value::ImageRgba8(_) => ValueType::ImageRgba8,
            Value::ImageGray8(_) => ValueType::ImageGray8,
        }
    }
}

#[derive(Debug, Clone)]
pub enum ValueType {
    Integer,
    Decimal,
    String,
    ImageRgba32F,
    ImageRgba8,
    ImageGray8,
}

