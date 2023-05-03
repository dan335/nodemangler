#[derive(Debug, Clone)]
pub enum Value {
    Integer {
        value: usize
    },
    Decimal {
        value: f32
    },
    String {
        value: String
    }
}

impl Value {
    pub fn value_type(self) -> ValueType {
        match self {
            Self::Decimal {value: _ } => ValueType::Decimal,
            Self::Integer {value: _ } => ValueType::Integer,
            Self::String {value: _ } => ValueType::String,
        }
    }
}

#[derive(Debug, Clone)]
pub enum ValueType {
    Integer,
    Decimal,
    String,
}