use crate::{value::Value, get_id, Input};
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

impl PartialEq for Output {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id
    }
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

    pub fn is_valid_connection(&self, input: &Input) -> bool {
        self.value.value_type().valid_conversions().contains(&input.value.value_type())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OutputLink {
    pub node_id: String,
    pub output_index: usize,
}
