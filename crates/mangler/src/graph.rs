use std::collections::HashMap;

use crate::node::Node;

pub struct Graph {
    pub nodes: HashMap<String, Box<dyn Node>>,
}

impl Graph {
    pub fn new() -> Graph {
        Graph {
            nodes: HashMap::new(),
        }
    }

    pub fn add_node(&mut self, id: String, node: Box<dyn Node>) {
        self.nodes.insert(id.clone(), node);
    }

    pub fn run(&mut self) {
        self.nodes.values_mut().for_each(|node| node.run());
    }
}