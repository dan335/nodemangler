use crate::value::{Value, ValueType};

#[derive(Debug)]
pub struct Input {
    pub name: String,
    pub value: Value,
    pub connection: Option<String>,
    pub valid_types: Vec<ValueType>,
}

impl Input {
    pub fn new(name: String, value: Value) -> Input {
        Input { name, value, connection: None, valid_types: Vec::new() }
    }
}


pub struct InputSettings {
    pub name: String,
    pub default_value: Value,
    pub valid_types: Vec<ValueType>,
}