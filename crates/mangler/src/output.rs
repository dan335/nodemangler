use crate::value::{Value, ValueType};

#[derive(Debug, Clone)]
pub struct Output {
    pub name: String,
    pub value: Value,
    pub value_type: ValueType,
    pub connection: Option<Vec<(String, usize)>>, // id of input node, index of input
}

impl Output {
    pub fn new(name: String, value: Value) -> Output {
        Output {
            name,
            value_type: value.value_type(),
            value,
            connection: None,
        }
    }
}
