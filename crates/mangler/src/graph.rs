use crate::{
    node::Node, operation::Operation, value::Value, AddedConnectionMessage, AddedNodeMessage,
    GraphSaveData, LoadedNodeMessage, NewGraphError, NodeInputChangedMessage,
    NodeOutputChangedMessage, RemovedConnectionMessage, RemovedNodeMessage,
};
use glam::f32::Vec2;
use std::fs;
use std::{
    collections::{HashMap, HashSet, VecDeque},
    path::PathBuf,
};
use tokio::sync::mpsc::Sender;

pub struct Graph {
    pub id: String,
    pub name: String,
    pub tx_output_changed: Sender<NodeOutputChangedMessage>,
    pub tx_input_changed: Sender<NodeInputChangedMessage>,
    pub tx_added_node: Sender<AddedNodeMessage>,
    pub tx_removed_node: Sender<RemovedNodeMessage>,
    pub tx_loaded_node: Sender<LoadedNodeMessage>,
    pub tx_added_connection: Sender<AddedConnectionMessage>,
    pub tx_removed_connection: Sender<RemovedConnectionMessage>,
    pub nodes: HashMap<String, Node>, // node_id, node
    pub is_dirty: bool,               // needs to run
    pub save_path: Option<PathBuf>,
}

impl Graph {
    pub fn new(
        id: String,
        tx_output_changed: Sender<NodeOutputChangedMessage>,
        tx_input_changed: Sender<NodeInputChangedMessage>,
        tx_added_node: Sender<AddedNodeMessage>,
        tx_removed_node: Sender<RemovedNodeMessage>,
        tx_loaded_node: Sender<LoadedNodeMessage>,
        tx_added_connection: Sender<AddedConnectionMessage>,
        tx_removed_connection: Sender<RemovedConnectionMessage>,
    ) -> Result<Graph, NewGraphError> {
        Ok(Graph {
            nodes: HashMap::new(),
            is_dirty: false,
            tx_output_changed,
            tx_input_changed,
            tx_added_node,
            tx_removed_node,
            tx_loaded_node,
            tx_added_connection,
            tx_removed_connection,
            save_path: None,
            id,
            name: "New Graph".to_string(),
        })
    }

    pub fn load(
        save_path: PathBuf,
        tx_output_changed: Sender<NodeOutputChangedMessage>,
        tx_input_changed: Sender<NodeInputChangedMessage>,
        tx_added_node: Sender<AddedNodeMessage>,
        tx_removed_node: Sender<RemovedNodeMessage>,
        tx_loaded_node: Sender<LoadedNodeMessage>,
        tx_added_connection: Sender<AddedConnectionMessage>,
        tx_removed_connection: Sender<RemovedConnectionMessage>,
    ) -> Result<Graph, NewGraphError> {
        match fs::read_to_string(&save_path) {
            Ok(data) => match serde_json::from_str::<GraphSaveData>(&data) {
                Ok(json) => {
                    let mut graph = Graph {
                        is_dirty: false,
                        tx_output_changed,
                        tx_input_changed,
                        tx_added_node,
                        tx_removed_node,
                        tx_loaded_node,
                        tx_added_connection,
                        tx_removed_connection,
                        save_path: Some(save_path),
                        nodes: json.nodes,
                        id: json.id,
                        name: json.name,
                    };

                    for (_node_id, node) in graph.nodes.iter_mut() {
                        node.is_dirty = true;

                        let added_node_message = LoadedNodeMessage { node: node.clone() };

                        match graph.tx_loaded_node.try_send(added_node_message) {
                            Ok(_) => {}
                            Err(err) => {
                                println!("Error sending added_node_message: {:?}", err);
                            }
                        }
                    }

                    Ok(graph)
                }
                Err(error) => Err(NewGraphError(format!(
                    "Error loading graph. Error: {}",
                    error.to_string()
                ))),
            },
            Err(error) => Err(NewGraphError(format!(
                "Error loading graph. Error: {}",
                error.to_string()
            ))),
        }
    }

    pub fn set_node_position(&mut self, node_id: String, position: glam::f32::Vec2) {
        if let Some(node) = self.nodes.get_mut(&node_id) {
            node.position = position;
            self.save_to_file();
        }
    }

    pub async fn add_node(&mut self, node_id: String, operation: Operation, position: Vec2) {
        let node = Node::new(node_id.clone(), operation.clone(), position);

        self.nodes.insert(node_id.clone(), node);
        self.is_dirty = true;

        let added_node_message = AddedNodeMessage {
            node_id,
            position,
            operation,
        };

        match self.tx_added_node.try_send(added_node_message) {
            Ok(_) => {}
            Err(err) => {
                println!("Error sending added_node_message: {:?}", err);
            }
        }

        self.save_to_file();
    }

    pub async fn remove_node(&mut self, node_id: String) {
        // get nodes that connect to this one
        let mut output_connections: Vec<(String, usize)> = Vec::new();
        let mut input_indexes: Vec<usize> = Vec::new();

        if let Some(node) = self.nodes.get(&node_id) {
            for input_index in 0..node.inputs.len() {
                input_indexes.push(input_index);
            }

            for output in node.outputs.iter() {
                if let Some(connections) = &output.connection {
                    for (other_node_id, input_index) in connections.iter() {
                        output_connections.push((other_node_id.clone(), *input_index));
                    }
                }
            }
        }

        for input_index in input_indexes.iter() {
            self.remove_connection(node_id.clone(), *input_index).await;
        }

        // remove connections
        for (connected_node_id, input_index) in output_connections.iter() {
            self.remove_connection(connected_node_id.clone(), *input_index)
                .await;
        }

        // remove node
        self.nodes.remove(&node_id);

        let removed_node_message = RemovedNodeMessage {
            node_id: node_id.clone(),
        };

        match self.tx_removed_node.try_send(removed_node_message) {
            Ok(_) => {}
            Err(err) => {
                println!("Error sending removed_node_message: {:?}", err);
            }
        }

        self.save_to_file();
    }

    pub async fn add_connection(
        &mut self,
        input_node_id: String,
        input_connection_index: usize,
        output_node_id: String,
        output_connection_index: usize,
    ) {
        if self.nodes.get_mut(&input_node_id).is_some()
            && self.nodes.get_mut(&output_node_id).is_some()
        {
            // set output connection
            if let Some(from) = self.nodes.get_mut(&output_node_id) {
                from.set_output_connection(
                    output_connection_index,
                    input_node_id.clone(),
                    input_connection_index,
                );

                from.is_dirty = true;
            }

            // set input connection
            if let Some(to) = self.nodes.get_mut(&input_node_id) {
                to.set_input_connection(
                    input_connection_index,
                    output_node_id.clone(),
                    output_connection_index,
                );
            }

            // mark graph as dirty
            self.is_dirty = true;

            let added_connection_message = AddedConnectionMessage {
                input_node_id,
                input_connection_index,
                output_node_id,
                output_connection_index,
            };

            match self.tx_added_connection.try_send(added_connection_message) {
                Ok(_) => {}
                Err(err) => {
                    println!("Error sending added_connection_message: {:?}", err);
                }
            }

            self.save_to_file();
        }
    }

    pub async fn remove_connection(&mut self, node_id: String, input_index: usize) {
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

        let removed_connection_message = RemovedConnectionMessage {
            node_id,
            input_index,
        };

        match self
            .tx_removed_connection
            .try_send(removed_connection_message)
        {
            Ok(_) => {}
            Err(err) => {
                println!("Error sending removed_connection_message: {:?}", err);
            }
        }

        self.save_to_file();
    }

    pub fn set_input(&mut self, node_id: String, input_index: usize, value: Value) {
        if let Some(node) = self.nodes.get_mut(&node_id) {
            if let Some(input) = node.inputs.get_mut(input_index) {
                input.value = value;
                node.is_dirty = true;
                self.save_to_file();
            }
        }
    }

    pub fn set_save_path(&mut self, save_path: PathBuf) {
        self.save_path = Some(save_path);
        self.save_to_file();
    }

    // https://github.com/emilk/egui/discussions/484
    // pub async fn run_async(&mut self) -> HashSet<String> {
    //     self.run().await
    // }

    // returns a list of node_ids that ran
    // so that their thumbnails will know to update
    pub async fn run(&mut self) {
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

        if nodes_to_check.is_empty() {
            return;
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
        let sorted_nodes = self.topological_sort(&self.nodes, &dirty_nodes);

        for node_id in sorted_nodes.into_iter() {
            // run node
            let mut output_data: Vec<(String, usize, Value)> = Vec::new(); // connected_node_id, input_index, output.value

            if let Some(node) = self.nodes.get_mut(&node_id) {
                // run
                node.run(self.tx_output_changed.clone()).await;

                // gather data to pass to connections
                for output in node.outputs.iter() {
                    if let Some(connections) = &output.connection {
                        for (connected_node_id, input_index) in connections.iter() {
                            output_data.push((
                                connected_node_id.clone(),
                                input_index.clone(),
                                output.value.clone(),
                            ));
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

                    // todo: send to editor

                    connected_node.set_input_value(*input_index, value.clone());

                    let node_input_changed_message = NodeInputChangedMessage {
                        node_id: connected_node_id.clone(),
                        input_index: input_index.clone(),
                        value: value.clone(),
                    };

                    match self.tx_input_changed.try_send(node_input_changed_message) {
                        Ok(_) => {}
                        Err(err) => {
                            println!("Error sending NodeInputChangedMessage: {:?}", err);
                        }
                    }
                    //connected_node.inputs[*input_index].value = value.clone();
                    //}
                }
            }
        }

        self.save_to_file();
    }

    pub fn save_to_file(&self) {
        if let Some(save_path) = &self.save_path {
            let data = GraphSaveData {
                nodes: self.nodes.clone(),
                id: self.id.clone(),
                name: self.name.clone(),
            };

            match serde_json::to_string(&data) {
                Ok(data_string) => {
                    let _result = fs::write(save_path, data_string);
                }
                Err(error) => {
                    println!("Error saving file.  {:?}", error);
                }
            }
        }
    }

    // Perform topological sorting on the dirty nodes
    fn topological_sort(
        &self,
        nodes: &HashMap<String, Node>,
        dirty_nodes: &HashSet<String>,
    ) -> Vec<String> {
        let mut visited: HashSet<String> = HashSet::new();
        let mut sorted_order: VecDeque<String> = VecDeque::new();

        for node_id in dirty_nodes {
            if !visited.contains(node_id) {
                self.visit_node(nodes, &node_id, &mut visited, &mut sorted_order);
            }
        }

        sorted_order.into_iter().collect()
    }

    // Recursive function to visit a node and its dependencies
    fn visit_node(
        &self,
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
                            self.visit_node(nodes, connection_node_id, visited, sorted_order);
                        }
                    }
                }
            }
        }

        sorted_order.push_front(node_id.clone());
    }
}
