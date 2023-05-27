use serde::{Serialize, Deserialize};

#[derive(Clone, Debug, Serialize, Deserialize, Default)]
pub struct NodeSettings {
    pub name: String,
}

impl NodeSettings {
    pub fn new(name: String) -> NodeSettings {
        NodeSettings {
            name,
        }
    }
}