use crate::value::Value;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Input {
    pub name: String,
    pub value: Value,
    pub connection: Option<(String, usize)>, // id of node with output, index of output
    pub is_exposed: bool,
}

impl Input {
    pub fn new(name: String, value: Value) -> Input {
        Input {
            name,
            value,
            connection: None,
            is_exposed: false,
        }
    }
}
