use image::ImageBuffer;

use crate::value::Value;

#[derive(Debug, Clone)]
pub struct Output {
    pub name: String,
    pub value: Value,
    pub connection: Option<Vec<(String, usize)>>, // id of input node, index of input
}

impl Output {
    pub fn new(name: String, value: Value) -> Output {
        Output {
            name,
            value,
            connection: None,
        }
    }
}
