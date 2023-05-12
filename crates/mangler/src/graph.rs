use std::collections::HashMap;

use crate::nodes::node::Node;

pub struct Graph {
    pub nodes: HashMap<String, Node>,
}

impl Graph {
    pub fn new() -> Graph {
        Graph {
            nodes: HashMap::new(),
        }
    }

    pub fn add_node(&mut self, id: String, node: Node) {
        self.nodes.insert(id.clone(), node);
    }

    pub fn run(&mut self) {
        self.nodes.values_mut().for_each(|node| { node.operation.run(&node.inputs, &mut node.outputs); });
    }
}