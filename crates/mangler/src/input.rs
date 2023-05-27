use crate::value::{Value, ValueType};
use crate::operation::{ConnectionSettings, UiType};
use serde::{Serialize, Deserialize};


#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Input {
    pub name: String,
    value: Value,
    pub connection: Option<(String, usize)>, // id of node with output, index of output
    pub valid_types: Vec<ValueType>,
    pub ui_type: Option<UiType>,
}

impl Input {
    pub fn new(settings: ConnectionSettings) -> Input {
        Input {
            name: settings.name,
            value: settings.default_value,
            connection: None,
            valid_types: settings.valid_types,
            ui_type: settings.ui_type,
        }
    }

    pub fn get_value(&self) -> &Value {
        &self.value
    }

    pub fn set_value(&mut self, value: Value) {
        self.value = value;
    }
}
