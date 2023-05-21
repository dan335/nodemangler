use crate::{nodes::node::Node, value::Value};
use std::{collections::{HashMap, HashSet, VecDeque}};
use std::future;

#[derive(Default)]
pub struct Graph {
    pub nodes: HashMap<String, Node>, // node_id, node
    pub is_dirty: bool,               // needs to run
}

impl Graph {
    pub fn add_node(&mut self, id: String, node: Node) {
        self.nodes.insert(id, node);
        self.is_dirty = true;
    }

    pub fn remove_node(&mut self, id: &String) {

        // get nodes that connect to this one
        let mut connected_nodes: Vec<String> = Vec::new();

        if let Some(node) = self.nodes.get(id) {
            for input in node.get_inputs().iter() {
                if let Some((other_node_id, _)) = &input.connection {
                    connected_nodes.push(other_node_id.clone());
                }
            }

            for output in node.outputs.iter() {
                if let Some(connections) = &output.connection {
                    for (other_node_id, _) in connections.iter() {
                        connected_nodes.push(other_node_id.clone());
                    }
                }
            }
        }

        // remove connections
        for node_id in connected_nodes.iter() {
            if let Some(node) = self.nodes.get_mut(node_id) {

                // inputs
                let mut inputs_to_clear: Vec<usize> = Vec::new();

                for (index, input) in node.get_inputs().iter().enumerate() {
                    if let Some((other_node_id, _)) = &input.connection {
                        if other_node_id == id {
                            inputs_to_clear.push(index);
                        }
                    }
                }

                for index in inputs_to_clear.iter() {
                    node.clear_input_connection(*index);
                    //node.inputs[*index].connection = None;
                }

                // outputs
                let mut outputs_to_clear: Vec<(usize, usize)> = Vec::new(); // output index, output connection index

                for (output_index, output) in node.outputs.iter().enumerate() {
                    if let Some(connections) = &output.connection {
                        for (output_connection_index, (other_node_id, _)) in connections.iter().enumerate() {
                            if other_node_id == id {
                                outputs_to_clear.push((output_index, output_connection_index));
                            }
                        }
                    }
                }

                for (output_index, output_connection_index) in outputs_to_clear.iter() {
                    if let Some(c) = node.outputs.get_mut(output_index.clone()) {
                        let d = c.connection.as_mut().unwrap();
                        d.remove(output_connection_index.clone());
                    }
                }
            }

            // remove node
            self.nodes.remove(id);
        }
    }

    pub fn remove_connection(&mut self, node_id: String, input_index: usize) {
        let mut output: Option<(String, usize)> = None;

        if let Some(node) = self.nodes.get_mut(&node_id) {

            if let Some((output_node_id, output_index)) = &node.get_input(input_index).connection {
                output = Some((output_node_id.clone(), output_index.clone()));
            }

            node.clear_input_connection(input_index);
            //node.inputs[input_index].connection = None;
        }

        if let Some((output_node_id, output_index)) = output {
            if let Some(node) = self.nodes.get_mut(&output_node_id) {
                if let Some(c) = node.outputs.get_mut(output_index.clone()) {
                    let d = c.connection.as_mut().unwrap();
                    d.remove(output_index.clone());
                }
            }
        }
    }

    // https://github.com/emilk/egui/discussions/484
    // pub async fn run_async(&mut self) -> HashSet<String> {
    //     self.run().await
    // }

    // returns a list of node_ids that ran
    // so that their thumbnails will know to update
    pub fn run(&mut self) -> HashSet<String> {
        let mut dirty_nodes: HashSet<String> = HashSet::new();
        let mut checked_nodes: HashSet<String> = HashSet::new();
        let mut nodes_to_check: VecDeque<String> = VecDeque::new();

        // find all dirty nodes
        for (node_id, node) in self.nodes.iter_mut() {
            if node.is_dirty {
                nodes_to_check.push_back(node_id.clone());
                node.is_dirty = false;
            }
        }

        // loop through dirty nodes and their dependecies
        // add to list to run
        while let Some(node_id) = nodes_to_check.pop_front() {
            dirty_nodes.insert(node_id.clone());

            if !checked_nodes.contains(&node_id) {
                checked_nodes.insert(node_id.clone());

                // add connections to queue
                if let Some(node) = self.nodes.get_mut(&node_id.clone()) {
                    for output in node.outputs.iter_mut() {
                        if let Some(connections) = &output.connection {
                            for (connection_node_id, _connection_input_index) in connections {
                                nodes_to_check.push_back(connection_node_id.clone());
                            }
                        }
                    }
                }
            }
        }

        // sort list to run
        let sorted_nodes = topological_sort(&self.nodes, &dirty_nodes);

        for node_id in sorted_nodes.into_iter() {
            // run node
            let mut output_data : Vec<(String, usize, Value)> = Vec::new();  // connected_node_id, input_index, output.value

            if let Some(node) = self.nodes.get_mut(&node_id) {
                // run
                node.run();
                
                // gather data to pass to connections
                for output in node.outputs.iter() {
                    if let Some(connections) = &output.connection {
                        for (connected_node_id, input_index) in connections.iter() {
                            output_data.push((connected_node_id.clone(), input_index.clone(), output.value.clone()));
                        }
                    }
                }
            }

            for (connected_node_id, input_index, value) in output_data.iter() {
                if let Some(connected_node) = self.nodes.get_mut(&connected_node_id.clone()) {
                    //if connected_node.inputs.len() > *input_index {
                        // should things be converted with code below?

                        // let converted = value.convert_to(connected_node.inputs[*input_index].value.clone().value_type());
                        // match converted {
                        //     Ok(converted_value) => {
                        //         connected_node.inputs[*input_index].value = converted_value;
                        //     }
                        //     Err(_) => {
                        //         panic!("Unable to convert.");
                        //     }
                        // }

                        connected_node.set_input_value(*input_index, value.clone());
                        //connected_node.inputs[*input_index].value = value.clone();
                    //}
                }
            }
        }

        // Perform topological sorting on the dirty nodes
        fn topological_sort(
            nodes: &HashMap<String, Node>,
            dirty_nodes: &HashSet<String>,
        ) -> Vec<String> {
            let mut visited: HashSet<String> = HashSet::new();
            let mut sorted_order: VecDeque<String> = VecDeque::new();

            for node_id in dirty_nodes {
                if !visited.contains(node_id) {
                    visit_node(nodes, &node_id, &mut visited, &mut sorted_order);
                }
            }

            sorted_order.into_iter().collect()
        }

        // Recursive function to visit a node and its dependencies
        fn visit_node(
            nodes: &HashMap<String, Node>,
            node_id: &String,
            visited: &mut HashSet<String>,
            sorted_order: &mut VecDeque<String>,
        ) {
            visited.insert(node_id.clone());

            if let Some(node) = nodes.get(node_id) {
                for output in node.outputs.iter() {
                    if let Some(connections) = &output.connection {
                        for (connection_node_id, _connection_input_index) in connections {
                            if !visited.contains(connection_node_id) {
                                visit_node(nodes, connection_node_id, visited, sorted_order);
                            }
                        }
                    }
                }
            }

            sorted_order.push_front(node_id.clone());
        }

        dirty_nodes.clone()
    }
}
