use crate::value::Value;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Output {
    pub name: String,
    pub value: Value,
    pub connection: Option<Vec<(String, usize)>>, // id of input node, index of input
    pub is_exposed: bool,
}

impl Output {
    pub fn new(name: String, value: Value) -> Output {
        Output {
            name,
            value,
            connection: None,
            is_exposed: false,
        }
    }
}
