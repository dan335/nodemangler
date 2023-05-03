use crate::value::Value;

#[derive(Debug)]
pub struct Output {
    pub name: String,
    pub value: Value,
    pub connection: Option<Vec<String>>,     // id of input
}

impl Output {
    pub fn new(name: String, value: Value) -> Output {
        Output { name, value, connection: None }
    }
}

pub struct OutputSettings {
    pub name: String,
    pub default_value: Value,
}