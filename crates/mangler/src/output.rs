use crate::value::Value;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Output {
    pub name: String,
    pub value: Value,
    pub connection: Option<Vec<(String, usize)>>, // id of input node, index of input
    pub expose: bool,
}

impl Output {
    pub fn new(name: String, value: Value) -> Output {
        Output {
            name,
            value,
            connection: None,
            expose: false,
        }
    }
}
