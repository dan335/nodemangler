use image::{imageops::FilterType, GrayImage, Rgba32FImage, RgbaImage};

#[derive(Debug, Clone)]
pub enum Value {
    Bool(bool),
    Integer(i32),
    Decimal(f32),
    String(String),
    ImageRgba32F(Rgba32FImage),
    ImageRgba8(RgbaImage),
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
            Value::FilterType(_) => ValueType::FilterType, // filter type for resizing images
        }
    }

    pub fn convert(&self, other: &Value) -> Value {
        let result = match (self, other) {
            (Value::Integer(a), Value::Integer(b)) => {
                Value::Integer(a + b)
            }
            (Value::Bool(_), Value::Bool(_)) => todo!(),
            (Value::Bool(_), Value::Integer(_)) => todo!(),
            (Value::Bool(_), Value::Decimal(_)) => todo!(),
            (Value::Bool(_), Value::String(_)) => todo!(),
            (Value::Bool(_), Value::ImageRgba32F(_)) => todo!(),
            (Value::Bool(_), Value::ImageRgba8(_)) => todo!(),
            (Value::Bool(_), Value::ImageGray8(_)) => todo!(),
            (Value::Bool(_), Value::FilterType(_)) => todo!(),
            (Value::Integer(_), Value::Bool(_)) => todo!(),
            (Value::Integer(_), Value::Decimal(_)) => todo!(),
            (Value::Integer(_), Value::String(_)) => todo!(),
            (Value::Integer(_), Value::ImageRgba32F(_)) => todo!(),
            (Value::Integer(_), Value::ImageRgba8(_)) => todo!(),
            (Value::Integer(_), Value::ImageGray8(_)) => todo!(),
            (Value::Integer(_), Value::FilterType(_)) => todo!(),
            (Value::Decimal(_), Value::Bool(_)) => todo!(),
            (Value::Decimal(_), Value::Integer(_)) => todo!(),
            (Value::Decimal(_), Value::Decimal(_)) => todo!(),
            (Value::Decimal(_), Value::String(_)) => todo!(),
            (Value::Decimal(_), Value::ImageRgba32F(_)) => todo!(),
            (Value::Decimal(_), Value::ImageRgba8(_)) => todo!(),
            (Value::Decimal(_), Value::ImageGray8(_)) => todo!(),
            (Value::Decimal(_), Value::FilterType(_)) => todo!(),
            (Value::String(_), Value::Bool(_)) => todo!(),
            (Value::String(_), Value::Integer(_)) => todo!(),
            (Value::String(_), Value::Decimal(_)) => todo!(),
            (Value::String(_), Value::String(_)) => todo!(),
            (Value::String(_), Value::ImageRgba32F(_)) => todo!(),
            (Value::String(_), Value::ImageRgba8(_)) => todo!(),
            (Value::String(_), Value::ImageGray8(_)) => todo!(),
            (Value::String(_), Value::FilterType(_)) => todo!(),
            (Value::ImageRgba32F(_), Value::Bool(_)) => todo!(),
            (Value::ImageRgba32F(_), Value::Integer(_)) => todo!(),
            (Value::ImageRgba32F(_), Value::Decimal(_)) => todo!(),
            (Value::ImageRgba32F(_), Value::String(_)) => todo!(),
            (Value::ImageRgba32F(_), Value::ImageRgba32F(_)) => todo!(),
            (Value::ImageRgba32F(_), Value::ImageRgba8(_)) => todo!(),
            (Value::ImageRgba32F(_), Value::ImageGray8(_)) => todo!(),
            (Value::ImageRgba32F(_), Value::FilterType(_)) => todo!(),
            (Value::ImageRgba8(_), Value::Bool(_)) => todo!(),
            (Value::ImageRgba8(_), Value::Integer(_)) => todo!(),
            (Value::ImageRgba8(_), Value::Decimal(_)) => todo!(),
            (Value::ImageRgba8(_), Value::String(_)) => todo!(),
            (Value::ImageRgba8(_), Value::ImageRgba32F(_)) => todo!(),
            (Value::ImageRgba8(_), Value::ImageRgba8(_)) => todo!(),
            (Value::ImageRgba8(_), Value::ImageGray8(_)) => todo!(),
            (Value::ImageRgba8(_), Value::FilterType(_)) => todo!(),
            (Value::ImageGray8(_), Value::Bool(_)) => todo!(),
            (Value::ImageGray8(_), Value::Integer(_)) => todo!(),
            (Value::ImageGray8(_), Value::Decimal(_)) => todo!(),
            (Value::ImageGray8(_), Value::String(_)) => todo!(),
            (Value::ImageGray8(_), Value::ImageRgba32F(_)) => todo!(),
            (Value::ImageGray8(_), Value::ImageRgba8(_)) => todo!(),
            (Value::ImageGray8(_), Value::ImageGray8(_)) => todo!(),
            (Value::ImageGray8(_), Value::FilterType(_)) => todo!(),
            (Value::FilterType(_), Value::Bool(_)) => todo!(),
            (Value::FilterType(_), Value::Integer(_)) => todo!(),
            (Value::FilterType(_), Value::Decimal(_)) => todo!(),
            (Value::FilterType(_), Value::String(_)) => todo!(),
            (Value::FilterType(_), Value::ImageRgba32F(_)) => todo!(),
            (Value::FilterType(_), Value::ImageRgba8(_)) => todo!(),
            (Value::FilterType(_), Value::ImageGray8(_)) => todo!(),
            (Value::FilterType(_), Value::FilterType(_)) => todo!(),
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
