use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize, Default)]
pub struct NodeSettings {
    pub name: String,
    // pub is_input: bool,
    // pub is_output: bool,
}

// impl NodeSettings {
//     pub fn new(name: String) -> NodeSettings {
//         NodeSettings { name }
//     }
// }
