use crate::{value::Value, get_id};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Output {
    pub id: String,
    pub name: String,
    pub value: Value,
    pub connection: Option<Vec<(String, usize)>>, // id of input node, index of input
    pub is_exposed: bool,
    #[serde(skip)]
    pub link: Option<OutputLink>,
}

impl Output {
    pub fn new(name: String, value: Value, link: Option<OutputLink>) -> Output {
        Output {
            name,
            value,
            connection: None,
            is_exposed: false,
            link,
            id: get_id(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OutputLink {
    pub node_id: String,
    pub output_index: usize,
}
