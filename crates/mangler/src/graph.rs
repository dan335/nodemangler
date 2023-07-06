use crate::input::{Input, InputLink, InputSettings};
use crate::node_type::NodeType;
use crate::output::{Output, OutputLink};
use crate::{AddNodeType, NodeChangedMessage, GraphChangedMessage};
use crate::{
    node::Node, value::Value,
    GraphSaveData, NewGraphError,
};
use glam::f32::Vec2;
use std::fs;
use std::{
    collections::{HashMap, HashSet, VecDeque},
    path::PathBuf,
};
use tokio::sync::mpsc::{Sender, self};
use async_recursion::async_recursion;
use crate::NodeChangedMessage::SubgraphLoaded;


#[derive(Debug)]
pub struct Graph {
    pub id: String,
    pub name: String,
    pub tx_node_changed: Option<Sender<NodeChangedMessage>>,
    pub tx_graph_changed: Option<Sender<GraphChangedMessage>>,
    pub nodes: HashMap<String, Node>, // node_id, node
    pub is_dirty: bool,               // needs to run
    pub save_path: Option<PathBuf>,
    pub is_subgraph: bool,
}

impl Graph {
    pub fn new(
        id: String,
        tx_node_changed: Sender<NodeChangedMessage>,
        tx_graph_changed: Sender<GraphChangedMessage>,
        is_subgraph: bool,
    ) -> Result<Graph, NewGraphError> {
        Ok(Graph {
            nodes: HashMap::new(),
            is_dirty: false,
            tx_node_changed: Some(tx_node_changed),
            tx_graph_changed: Some(tx_graph_changed),
            save_path: None,
            id,
            name: "new graph".to_string(),
            is_subgraph,
        })
    }

    pub fn load(
        save_path: PathBuf,
        tx_node_changed: Option<Sender<NodeChangedMessage>>,
        tx_graph_changed: Option<Sender<GraphChangedMessage>>,
        is_subgraph: bool,
    ) -> Result<Graph, NewGraphError> {
        match fs::read_to_string(&save_path) {
            Ok(data) => match serde_json::from_str::<GraphSaveData>(&data) {
                Ok(json) => {
                    let mut graph = Graph {
                        is_dirty: false,
                        tx_node_changed,
                        save_path: Some(save_path),
                        nodes: json.nodes,
                        id: json.id,
                        name: json.name,
                        tx_graph_changed,
                        is_subgraph
                    };

                    for (_node_id, node) in graph.nodes.iter_mut() {
                        node.is_dirty = true;

                        // let ui know node was created
                        if let Some(tx) = &graph.tx_graph_changed {
                            let message = GraphChangedMessage::LoadedNode { node: node.clone() };

                            match tx.try_send(message) {
                                Ok(_) => {}
                                Err(err) => {
                                    println!("Error sending added_node_message: {:?}", err);
                                }
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
        }
    }

    pub async fn add_node(&mut self, node_id: String, node_type: AddNodeType, position: Vec2) -> String {
        let mut node = Node::new(node_id.clone(), node_type.clone(), position);
        let mut is_subgraph = false;

        match node_type {
            AddNodeType::Subgraph => {
                node.inputs.clear();
                node.outputs.clear();

                let input_settings = Some(InputSettings::Path {
                    extension_filter: vec!["mangle".to_string()],
                    set_directory: None,
                    set_file_name: None,
                    set_title: Some("open subgraph".to_string()),
                    file_dialog_type: crate::input::FileDialogType::PickFile,
                });

                node.inputs.push(Input::new("file path".to_string(), Value::Path(PathBuf::new()), input_settings, None));
                is_subgraph = true;
            },
            _ => {}
        }

        if let Some(tx) = &self.tx_graph_changed {
            let message = GraphChangedMessage::AddedNode {
                node_id: node_id.clone(),
                position,
                settings: node.settings.clone(),
                inputs: node.inputs.clone(),
                outputs: node.outputs.clone(),
                is_subgraph,
            };
    
            match tx.try_send(message) {
                Ok(_) => {}
                Err(err) => {
                    println!("Error sending GraphChangedMessage::AddedNode: {:?}", err);
                }
            }
        }

        self.is_dirty = true;
        self.nodes.insert(node_id.clone(), node);

        node_id
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

        if let Some(tx) = &self.tx_graph_changed {
            let message = GraphChangedMessage::RemovedNode {
                node_id: node_id.clone(),
            };

            match tx.try_send(message) {
                Ok(_) => {}
                Err(err) => {
                    println!("Error sending removed_node_message: {:?}", err);
                }
            }
        }
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
            let mut is_valid = false;
            
            // check if valid connection
            if let Some(from_output) = self.nodes.get(&output_node_id) {
                if let Some(to_input) = self.nodes.get(&input_node_id) {
                    if from_output.outputs.len() >= output_connection_index && to_input.inputs.len() >= input_connection_index {
                        if from_output.outputs[output_connection_index].is_valid_connection(&to_input.inputs[input_connection_index]) {
                            is_valid = true;
                        }
                    }
                }
            }

            if is_valid {
                // set output connection
                if let Some(from_output) = self.nodes.get_mut(&output_node_id) {
                    from_output.set_output_connection(
                        output_connection_index,
                        input_node_id.clone(),
                        input_connection_index,
                    );
    
                    from_output.is_dirty = true;
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

                // send message ot ui
                if let Some(tx) = &self.tx_graph_changed {
                    let message = GraphChangedMessage::AddedConnection {
                        input_node_id,
                        input_connection_index,
                        output_node_id,
                        output_connection_index,
                    };
    
                    match tx.try_send(message) {
                        Ok(_) => {}
                        Err(err) => {
                            println!("Error sending added_connection_message: {:?}", err);
                        }
                    }
                }
            }
        }
    }

    pub async fn remove_connection(&mut self, node_id: String, input_index: usize) {
        let mut output: Option<(String, usize)> = None;

        if let Some(node) = self.nodes.get_mut(&node_id) {
            if let Some((output_node_id, output_index)) = &node.get_input(input_index).connection {
                output = Some((output_node_id.clone(), *output_index));
            }

            node.clear_input_connection(input_index);
        }

        if let Some((output_node_id, output_index)) = output {
            if let Some(node) = self.nodes.get_mut(&output_node_id) {
                if let Some(c) = node.outputs.get_mut(output_index) {
                    let d = c.connection.as_mut().unwrap();
                    d.remove(output_index);
                }
            }
        }

        if let Some(tx) = &self.tx_graph_changed {
            let message = GraphChangedMessage::RemovedConnection {
                node_id,
                input_index,
            };
    
            match tx.try_send(message)
            {
                Ok(_) => {}
                Err(err) => {
                    println!("Error sending GraphChangedMessage::RemovedConnection: {:?}", err);
                }
            }
        }
    }


    // when getting message that input should change
    // not passed from other node
    pub fn set_input(&mut self, node_id: String, input_index: usize, value: Value) {
        if let Some(node) = self.nodes.get_mut(&node_id) {
            if let Some(input) = node.inputs.get_mut(input_index) {
                // set value
                input.value = value.clone();

                // mark node as dirty so that it will run next time graph runs
                node.is_dirty = true;

                // if input has a link then pass value to linked input
                if let Some(link) = &input.link {
                    if let NodeType::Subgraph { path:_, graph:possible_subgraph, rx_node_changed:_ } = &mut node.node_type {
                        if let Some(subgraph) = possible_subgraph {
                            if let Some(subgraph_node) = subgraph.nodes.get_mut(&link.node_id) {

                                if let Some(i) = subgraph_node.inputs.iter_mut().position(|i| i.id == link.input_id) {
                                    subgraph_node.set_input_value(i, value.clone());
                                }
                            }
                            
                        }
                    }
                }
            }

            // if this node is a subgraph
            if let NodeType::Subgraph { path:_, graph:_, rx_node_changed:_ } = &node.node_type {
                // if value is subgraph location
                // load subgraph
                if let Value::Path(path) = value {
                    
                    // create graph from path
                    let (tx_node_changed, rx_node_changed) = mpsc::channel::<NodeChangedMessage>(32);
                    match Graph::load(path.clone(), Some(tx_node_changed), None, true) {
                        Ok(subgraph) => {

                            for (subgraph_node_id, subgraph_node) in subgraph.nodes.iter() {
                                // create inputs for node
                                // from subgraph's exposed inputs
                                for (_input_index, subgraph_input) in subgraph_node.inputs.iter().enumerate() {
                                    if subgraph_input.is_exposed {
                                        let input_settings = Some(InputSettings::Path {
                                            extension_filter: vec!["mangle".to_string()],
                                            set_directory: None,
                                            set_file_name: None,
                                            set_title: Some("open subgraph".to_string()),
                                            file_dialog_type: crate::input::FileDialogType::PickFile,
                                        });

                                        node.inputs.push(
                                            Input::new(
                                                subgraph_input.name.clone(),
                                                subgraph_input.value.clone(),
                                                input_settings,
                                                Some(InputLink {node_id: subgraph_node_id.clone(), input_id: subgraph_input.id.clone()})
                                            )
                                        );
                                    }
                                }

                                // create outputs for node
                                // from subgraph's exposed outputs
                                for (output_index, subgraph_output) in subgraph_node.outputs.iter().enumerate() {
                                    if subgraph_output.is_exposed {
                                        node.outputs.push(Output::new(subgraph_output.name.clone(), subgraph_output.value.clone(), Some(OutputLink { node_id: subgraph_node_id.clone(), output_index })));
                                    }
                                }
                            }

                            // other settings for node
                            node.settings.name = subgraph.name.clone();
                            node.node_type = NodeType::Subgraph { path: path.to_path_buf(), graph: Some(subgraph), rx_node_changed: Some(rx_node_changed) };

                            // mark dirty so that it runs
                            node.is_dirty = true;

                            // send message to ui
                            if let Some(tx) = &self.tx_node_changed {
                                let message = SubgraphLoaded {
                                    node_id,
                                    settings: node.settings.clone(),
                                    inputs: node.inputs.clone(),
                                    outputs: node.outputs.clone(),
                                };
    
                                match tx.try_send(message) {
                                    Ok(_) => {}
                                    Err(err) => {
                                        println!("Error sending SubgraphLoaded: {:?}", err);
                                    }
                                }
                            }
                        }
                        Err(error) => {
                            println!("Error loading subgraph. {:#?}", error);
                        },
                        
                    }
                }

                
            }
        }
    }

    pub fn set_save_path(&mut self, save_path: PathBuf) {
        self.save_path = Some(save_path);
    }

    // returns a list of node_ids that ran
    // so that their thumbnails will know to update
    #[async_recursion]
    pub async fn run(&mut self) {
        let mut dirty_nodes: HashSet<String> = HashSet::new();
        let mut checked_nodes: HashSet<String> = HashSet::new();
        let mut nodes_to_check: VecDeque<String> = VecDeque::new();

        // find all dirty nodes
        // return early if node is busy
        for (node_id, node) in self.nodes.iter_mut() {
            if node.is_busy {
                return;
            }

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
                node.run(self.tx_node_changed.clone()).await;

                // gather data to pass to connections
                for output in node.outputs.iter() {
                    if let Some(connections) = &output.connection {
                        for (connected_node_id, input_index) in connections.iter() {
                            output_data.push((
                                connected_node_id.clone(),
                                *input_index,
                                output.value.clone(),
                            ));
                        }
                    }
                }
            }

            for (connected_node_id, input_index, value) in output_data.iter() {
                if let Some(connected_node) = self.nodes.get_mut(&connected_node_id.clone()) {

                    //connected_node.set_input_value(*input_index, value.clone());
                    connected_node.inputs[*input_index].value = value.clone();

                    if let Some(tx) = &self.tx_node_changed {
                        let message = NodeChangedMessage::InputChanged {
                            node_id: connected_node_id.clone(),
                            input_index: *input_index,
                            value: value.clone(),
                        };
    
                        match tx.try_send(message) {
                            Ok(_) => {}
                            Err(err) => {
                                println!("Error sending NodeChangedMessage::InputChanged: {:?}", err);
                            }
                        }
                    }
                }
            }
        }
    }

    // save graph to disk
    // unless this is a subgraph
    pub fn save_to_file(&self) {
        if self.is_subgraph {
            return;
        }
        
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
                self.visit_node(nodes, node_id, &mut visited, &mut sorted_order);
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
