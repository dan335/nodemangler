#[derive(Debug, Clone)]
pub enum Value {
    Integer(usize),
    Decimal(f32),
    String(String)
}

impl Value {
    pub fn value_type(self) -> ValueType {
        match self {
            Value::Integer(_) => ValueType::Integer,
            Value::Decimal(_) => ValueType::Decimal,
            Value::String(_) => ValueType::String,
        }
    }
}

#[derive(Debug, Clone)]
pub enum ValueType {
    Integer,
    Decimal,
    String,
}

