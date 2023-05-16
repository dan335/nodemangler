#[derive(Clone, Debug)]
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