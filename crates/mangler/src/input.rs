use crate::value::{Value, ValueType};

#[derive(Debug, Clone)]
pub struct Input {
    pub name: String,
    pub value: Value,
    pub connection: Option<(String, usize)>, // id, index of output
    pub valid_types: Vec<ValueType>,
}

impl Input {
    pub fn new(name: String, value: Value) -> Input {
        Input { name, value, connection: None, valid_types: Vec::new() }
    }
}


