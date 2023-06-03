use crate::{value::Value, get_id};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Input {
    pub id: String,
    pub name: String,
    pub value: Value,
    pub connection: Option<(String, usize)>, // id of node with output, index of output
    pub is_exposed: bool,
    // todo: need to link this somehow with exposed input
    // maybe it needs an id?
    // input that this input should pass data to
    // used to pass node's input to subgraph input
    #[serde(skip)]
    pub link: Option<InputLink>,
}

impl PartialEq for Input {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id
    }
}

impl Input {
    pub fn new(name: String, value: Value, link: Option<InputLink>) -> Input {
        Input {
            id: get_id(),
            name,
            value,
            connection: None,
            is_exposed: false,
            link,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InputLink {
    pub node_id: String,
    pub input_id: String,
}