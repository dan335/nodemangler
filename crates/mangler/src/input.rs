use crate::{value::{Value, ValueType}, nodes::operation::UiType};

#[derive(Debug, Clone)]
pub struct Input {
    pub name: String,
    pub value: Value,
    pub connection: Option<(String, usize)>, // id of node with output, index of output
    pub valid_types: Vec<ValueType>,
    pub ui_type: Option<UiType>,
}

impl Input {
    pub fn new(name: String, value: Value, ui_type: UiType) -> Input {
        Input {
            name,
            value,
            connection: None,
            valid_types: Vec::new(),
            ui_type: Some(ui_type)
        }
    }
}


