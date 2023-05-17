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
