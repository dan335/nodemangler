//! The node graph engine: stores nodes, manages connections, executes the
//! processing pipeline, and handles save/load to JSON.
//!
//! The [`Graph`] is the central data structure that owns all nodes, tracks dirty
//! state, and orchestrates execution. When run, it performs a topological sort of
//! dirty nodes and their downstream dependents, then executes them in order while
//! propagating output values through connections. An input-hash cache skips nodes
//! whose inputs have not changed since the last run.

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

/// The node graph engine that owns all nodes, manages connections, and
/// orchestrates the processing pipeline.
///
/// Communication with the UI happens through two channel senders:
/// - `tx_node_changed`: notifies the UI when individual node state changes.
/// - `tx_graph_changed`: notifies the UI when graph structure changes.
#[derive(Debug)]
pub struct Graph {
    /// Unique identifier for this graph.
    pub id: String,
    /// Human-readable name for this graph.
    pub name: String,
    /// Channel for sending node state changes to the UI.
    pub tx_node_changed: Option<Sender<NodeChangedMessage>>,
    /// Channel for sending graph structure changes to the UI.
    pub tx_graph_changed: Option<Sender<GraphChangedMessage>>,
    /// All nodes in the graph, keyed by node ID.
    pub nodes: HashMap<String, Node>,
    /// Whether the graph has pending changes that require execution.
    pub is_dirty: bool,
    /// File path for saving this graph, if set.
    pub save_path: Option<PathBuf>,
    /// Whether this graph is embedded inside a subgraph node (affects save behavior).
    pub is_subgraph: bool,
}

impl Graph {
    /// Create a new empty graph with the given channel senders for UI communication.
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

    /// Load a graph from a `.mangle` JSON file on disk.
    ///
    /// Deserializes the graph structure, marks all nodes as dirty so they will
    /// run on the next execution pass, and sends `LoadedNode` messages to the UI.
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

    /// Update a node's canvas position. No-op if the node does not exist.
    pub fn set_node_position(&mut self, node_id: String, position: glam::f32::Vec2) {
        if let Some(node) = self.nodes.get_mut(&node_id) {
            node.position = position;
        }
    }

    /// Add a new node to the graph and notify the UI.
    ///
    /// For subgraph nodes, a file path input is created so the user can select
    /// which `.mangle` file to load. Returns the node ID.
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

    /// Remove a node from the graph, cleaning up all its inbound and outbound
    /// connections, and notify the UI.
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

    /// Create a connection from an output to an input, after validating type
    /// compatibility. No-op if either node doesn't exist or the types are incompatible.
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
                    to.cached_input_hash = None;
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

    /// Remove the connection feeding into a specific input, clearing both the
    /// input side and the corresponding entry on the upstream output's connection list.
    pub async fn remove_connection(&mut self, node_id: String, input_index: usize) {
        let mut output: Option<(String, usize)> = None;

        if let Some(node) = self.nodes.get_mut(&node_id) {
            if let Some((output_node_id, output_index)) = &node.get_input(input_index).connection {
                output = Some((output_node_id.clone(), *output_index));
            }

            node.clear_input_connection(input_index);
            node.cached_input_hash = None;
        }

        if let Some((output_node_id, output_index)) = output {
            if let Some(node) = self.nodes.get_mut(&output_node_id) {
                if let Some(c) = node.outputs.get_mut(output_index) {
                    if let Some(d) = c.connection.as_mut() {
                        d.retain(|item| *item != (node_id.clone(), input_index));
                    }
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


    /// Set an input value directly (from user interaction, not from a connection).
    ///
    /// Marks the node as dirty and invalidates its cached input hash. If the input
    /// has a subgraph link, the value is also forwarded into the child graph. If the
    /// node is a subgraph and the value is a Path, the subgraph file is loaded and
    /// the node's inputs/outputs are populated from the child graph's exposed I/O.
    pub fn set_input(&mut self, node_id: String, input_index: usize, value: Value) {
        if let Some(node) = self.nodes.get_mut(&node_id) {
            if let Some(input) = node.inputs.get_mut(input_index) {
                // set value
                input.value = value.clone();

                // mark node as dirty so that it will run next time graph runs
                node.is_dirty = true;
                node.cached_input_hash = None;

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

    /// Set the file path where this graph will be saved.
    pub fn set_save_path(&mut self, save_path: PathBuf) {
        self.save_path = Some(save_path);
    }

    // returns a list of node_ids that ran
    // so that their thumbnails will know to update
    #[async_recursion]
    pub async fn run(&mut self) {
        let run_start = std::time::Instant::now();
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
                if let Some(node) = self.nodes.get(&node_id) {
                    for output in node.outputs.iter() {
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
            // Compute input hash for cache check
            let input_hash = if let Some(node) = self.nodes.get(&node_id) {
                use std::hash::{Hash, Hasher};
                use std::collections::hash_map::DefaultHasher;
                let mut h = DefaultHasher::new();
                for input in &node.inputs {
                    input.value.fingerprint().hash(&mut h);
                }
                h.finish()
            } else {
                continue;
            };

            // Skip if inputs unchanged since last run
            let skip = if let Some(node) = self.nodes.get(&node_id) {
                node.cached_input_hash == Some(input_hash)
            } else {
                false
            };

            if skip {
                // Still propagate existing outputs to downstream nodes
                let mut output_data: Vec<(String, usize, Value)> = Vec::new();
                if let Some(node) = self.nodes.get(&node_id) {
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
                for (connected_node_id, input_index, value) in output_data.into_iter() {
                    if let Some(connected_node) = self.nodes.get_mut(&connected_node_id) {
                        connected_node.inputs[input_index].value = value;
                    }
                }
                continue;
            }

            // Run node
            let mut output_data: Vec<(String, usize, Value)> = Vec::new();

            if let Some(node) = self.nodes.get_mut(&node_id) {
                node.run(self.tx_node_changed.clone()).await;
                node.cached_input_hash = Some(input_hash);

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

            for (connected_node_id, input_index, value) in output_data.into_iter() {
                if let Some(connected_node) = self.nodes.get_mut(&connected_node_id) {
                    if let Some(tx) = &self.tx_node_changed {
                        let message = NodeChangedMessage::InputChanged {
                            node_id: connected_node_id.clone(),
                            input_index,
                            value: value.clone(),
                        };
                        match tx.try_send(message) {
                            Ok(_) => {}
                            Err(err) => {
                                println!("Error sending NodeChangedMessage::InputChanged: {:?}", err);
                            }
                        }
                    }

                    // Move value into the connected input (no clone)
                    connected_node.inputs[input_index].value = value;
                }
            }
        }

        // Send total graph run time
        if let Some(tx) = &self.tx_node_changed {
            let _ = tx.try_send(NodeChangedMessage::GraphRunCompleted {
                total_time: run_start.elapsed(),
            });
        }
    }

    /// Serialize and write this graph to its save path as JSON.
    ///
    /// No-op if this is a subgraph (subgraphs are saved separately) or if
    /// no save path has been set.
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

    /// Topological sort that returns levels for parallel execution.
    /// Each level contains nodes that are independent and can run concurrently.
    #[allow(dead_code)]
    fn topological_sort_levels(
        &self,
        nodes: &HashMap<String, Node>,
        dirty_nodes: &HashSet<String>,
    ) -> Vec<Vec<String>> {
        // Build adjacency and in-degree maps restricted to dirty_nodes
        let mut in_degree: HashMap<String, usize> = HashMap::new();
        let mut adjacency: HashMap<String, Vec<String>> = HashMap::new();

        for node_id in dirty_nodes {
            in_degree.entry(node_id.clone()).or_insert(0);
            adjacency.entry(node_id.clone()).or_insert_with(Vec::new);
        }

        for node_id in dirty_nodes {
            if let Some(node) = nodes.get(node_id) {
                for output in &node.outputs {
                    if let Some(connections) = &output.connection {
                        for (connected_id, _) in connections {
                            if dirty_nodes.contains(connected_id) {
                                *in_degree.entry(connected_id.clone()).or_insert(0) += 1;
                                adjacency.entry(node_id.clone()).or_default().push(connected_id.clone());
                            }
                        }
                    }
                }
            }
        }

        let mut levels: Vec<Vec<String>> = Vec::new();
        let mut queue: Vec<String> = in_degree.iter()
            .filter(|(_, &deg)| deg == 0)
            .map(|(id, _)| id.clone())
            .collect();

        while !queue.is_empty() {
            levels.push(queue.clone());
            let mut next_queue = Vec::new();
            for node_id in &queue {
                if let Some(neighbors) = adjacency.get(node_id) {
                    for neighbor in neighbors {
                        let deg = in_degree.get_mut(neighbor).unwrap();
                        *deg -= 1;
                        if *deg == 0 {
                            next_queue.push(neighbor.clone());
                        }
                    }
                }
            }
            queue = next_queue;
        }

        levels
    }

    /// Perform a depth-first topological sort on the dirty nodes, returning them
    /// in dependency order (upstream nodes first) so that each node runs after
    /// all its inputs are available.
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

    /// Recursive DFS visitor for topological sort. Visits downstream neighbors
    /// first, then pushes the current node to the front of the sorted order.
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

#[cfg(test)]
mod tests {
    use tokio::sync::mpsc;

    use crate::{
        get_id, graph::Graph, operations::Operation, value::Value, AddNodeType,
        GraphChangedMessage, NodeChangedMessage,
    };

    fn create_test_graph() -> Graph {
        let (tx_graph_changed, _rx_graph_changed) = mpsc::channel::<GraphChangedMessage>(32);
        let (tx_node_changed, _rx_node_changed) = mpsc::channel::<NodeChangedMessage>(32);
        Graph::new(get_id(), tx_node_changed, tx_graph_changed, false).unwrap()
    }

    #[tokio::test]
    async fn test_graph_new() {
        let graph = create_test_graph();
        assert!(graph.nodes.is_empty());
        assert!(!graph.is_dirty);
        assert!(!graph.is_subgraph);
    }

    #[tokio::test]
    async fn test_add_node() {
        let mut graph = create_test_graph();
        let node_id = graph
            .add_node(
                get_id(),
                AddNodeType::Operation(Operation::OpNumberMathAdd),
                glam::Vec2::ZERO,
            )
            .await;

        assert!(graph.nodes.contains_key(&node_id));
        assert!(graph.is_dirty);

        let node = graph.nodes.get(&node_id).unwrap();
        assert_eq!(node.inputs.len(), 2); // a, b
        assert_eq!(node.outputs.len(), 1);
        assert_eq!(node.settings.name, "add");
    }

    #[tokio::test]
    async fn test_add_decimal_input_node() {
        let mut graph = create_test_graph();
        let node_id = graph
            .add_node(
                get_id(),
                AddNodeType::Operation(Operation::OpNumberInputDecimal),
                glam::Vec2::ZERO,
            )
            .await;

        let node = graph.nodes.get(&node_id).unwrap();
        assert_eq!(node.inputs.len(), 1);
        assert_eq!(node.outputs.len(), 1);
        assert_eq!(node.settings.name, "decimal");
    }

    #[tokio::test]
    async fn test_remove_node() {
        let mut graph = create_test_graph();
        let node_id = graph
            .add_node(
                get_id(),
                AddNodeType::Operation(Operation::OpNumberInputDecimal),
                glam::Vec2::ZERO,
            )
            .await;

        assert!(graph.nodes.contains_key(&node_id));
        graph.remove_node(node_id.clone()).await;
        assert!(!graph.nodes.contains_key(&node_id));
    }

    #[tokio::test]
    async fn test_set_input() {
        let mut graph = create_test_graph();
        let node_id = graph
            .add_node(
                get_id(),
                AddNodeType::Operation(Operation::OpNumberMathAdd),
                glam::Vec2::ZERO,
            )
            .await;

        graph.set_input(node_id.clone(), 0, Value::Decimal(42.0));

        let node = graph.nodes.get(&node_id).unwrap();
        match &node.inputs[0].value {
            Value::Decimal(v) => assert_eq!(*v, 42.0),
            other => panic!("Expected Decimal, got {:?}", other),
        }
        assert!(node.is_dirty);
    }

    #[tokio::test]
    async fn test_add_connection() {
        let mut graph = create_test_graph();

        let decimal_node_id = graph
            .add_node(
                get_id(),
                AddNodeType::Operation(Operation::OpNumberInputDecimal),
                glam::Vec2::new(0.0, 0.0),
            )
            .await;

        let add_node_id = graph
            .add_node(
                get_id(),
                AddNodeType::Operation(Operation::OpNumberMathAdd),
                glam::Vec2::new(200.0, 0.0),
            )
            .await;

        // Connect decimal output 0 -> add input 0
        graph
            .add_connection(add_node_id.clone(), 0, decimal_node_id.clone(), 0)
            .await;

        // Verify input side
        let add_node = graph.nodes.get(&add_node_id).unwrap();
        assert!(add_node.inputs[0].connection.is_some());
        let (conn_node_id, conn_output_idx) = add_node.inputs[0].connection.as_ref().unwrap();
        assert_eq!(conn_node_id, &decimal_node_id);
        assert_eq!(*conn_output_idx, 0);

        // Verify output side
        let decimal_node = graph.nodes.get(&decimal_node_id).unwrap();
        assert!(decimal_node.outputs[0].connection.is_some());
    }

    #[tokio::test]
    async fn test_run_single_node() {
        let mut graph = create_test_graph();
        let node_id = graph
            .add_node(
                get_id(),
                AddNodeType::Operation(Operation::OpNumberMathAdd),
                glam::Vec2::ZERO,
            )
            .await;

        graph.set_input(node_id.clone(), 0, Value::Decimal(5.0));
        graph.set_input(node_id.clone(), 1, Value::Decimal(10.0));

        graph.run().await;

        let node = graph.nodes.get(&node_id).unwrap();
        match &node.outputs[0].value {
            Value::Decimal(v) => assert!((*v - 15.0).abs() < 1e-6, "Expected 15.0, got {}", v),
            other => panic!("Expected Decimal output, got {:?}", other),
        }
    }

    #[tokio::test]
    async fn test_run_connected_nodes() {
        let mut graph = create_test_graph();

        // Create two decimal input nodes
        let input_a_id = graph
            .add_node(
                get_id(),
                AddNodeType::Operation(Operation::OpNumberInputDecimal),
                glam::Vec2::new(0.0, 0.0),
            )
            .await;
        let input_b_id = graph
            .add_node(
                get_id(),
                AddNodeType::Operation(Operation::OpNumberInputDecimal),
                glam::Vec2::new(0.0, 100.0),
            )
            .await;

        // Create add node
        let add_node_id = graph
            .add_node(
                get_id(),
                AddNodeType::Operation(Operation::OpNumberMathAdd),
                glam::Vec2::new(200.0, 0.0),
            )
            .await;

        // Set input values
        graph.set_input(input_a_id.clone(), 0, Value::Decimal(7.0));
        graph.set_input(input_b_id.clone(), 0, Value::Decimal(3.0));

        // Connect: input_a output 0 -> add input 0
        graph
            .add_connection(add_node_id.clone(), 0, input_a_id.clone(), 0)
            .await;
        // Connect: input_b output 0 -> add input 1
        graph
            .add_connection(add_node_id.clone(), 1, input_b_id.clone(), 0)
            .await;

        graph.run().await;

        let add_node = graph.nodes.get(&add_node_id).unwrap();
        match &add_node.outputs[0].value {
            Value::Decimal(v) => assert!((*v - 10.0).abs() < 1e-6, "Expected 10.0, got {}", v),
            other => panic!("Expected Decimal output, got {:?}", other),
        }
    }

    #[tokio::test]
    async fn test_set_node_position() {
        let mut graph = create_test_graph();
        let node_id = graph
            .add_node(
                get_id(),
                AddNodeType::Operation(Operation::OpNumberInputDecimal),
                glam::Vec2::ZERO,
            )
            .await;

        graph.set_node_position(node_id.clone(), glam::Vec2::new(100.0, 200.0));

        let node = graph.nodes.get(&node_id).unwrap();
        assert_eq!(node.position, glam::Vec2::new(100.0, 200.0));
    }

    #[tokio::test]
    async fn test_multiple_nodes_multiple_types() {
        let mut graph = create_test_graph();

        // Integer + Integer through add
        let add_id = graph
            .add_node(
                get_id(),
                AddNodeType::Operation(Operation::OpNumberMathAdd),
                glam::Vec2::ZERO,
            )
            .await;

        graph.set_input(add_id.clone(), 0, Value::Integer(100));
        graph.set_input(add_id.clone(), 1, Value::Integer(200));

        graph.run().await;

        let node = graph.nodes.get(&add_id).unwrap();
        match &node.outputs[0].value {
            Value::Integer(v) => assert_eq!(*v, 300),
            other => panic!("Expected Integer output, got {:?}", other),
        }
    }

    // === new() edge cases ===

    #[tokio::test]
    async fn test_graph_new_subgraph() {
        let (tx_graph_changed, _rx) = mpsc::channel::<GraphChangedMessage>(32);
        let (tx_node_changed, _rx) = mpsc::channel::<NodeChangedMessage>(32);
        let graph = Graph::new(get_id(), tx_node_changed, tx_graph_changed, true).unwrap();
        assert!(graph.is_subgraph);
        assert!(graph.save_path.is_none());
        assert_eq!(graph.name, "new graph");
    }

    // === remove_connection ===

    #[tokio::test]
    async fn test_remove_connection() {
        let mut graph = create_test_graph();

        let decimal_id = graph
            .add_node(get_id(), AddNodeType::Operation(Operation::OpNumberInputDecimal), glam::Vec2::ZERO)
            .await;
        let add_id = graph
            .add_node(get_id(), AddNodeType::Operation(Operation::OpNumberMathAdd), glam::Vec2::new(200.0, 0.0))
            .await;

        graph.add_connection(add_id.clone(), 0, decimal_id.clone(), 0).await;

        // Verify connection exists
        assert!(graph.nodes.get(&add_id).unwrap().inputs[0].connection.is_some());

        // Remove it
        graph.remove_connection(add_id.clone(), 0).await;

        // Input side cleared
        assert!(graph.nodes.get(&add_id).unwrap().inputs[0].connection.is_none());

        // Output side cleared
        let decimal_node = graph.nodes.get(&decimal_id).unwrap();
        let conns = decimal_node.outputs[0].connection.as_ref();
        assert!(conns.is_none() || conns.unwrap().is_empty());
    }

    // === remove_node with connections ===

    #[tokio::test]
    async fn test_remove_node_cleans_up_connections() {
        let mut graph = create_test_graph();

        let decimal_id = graph
            .add_node(get_id(), AddNodeType::Operation(Operation::OpNumberInputDecimal), glam::Vec2::ZERO)
            .await;
        let add_id = graph
            .add_node(get_id(), AddNodeType::Operation(Operation::OpNumberMathAdd), glam::Vec2::new(200.0, 0.0))
            .await;

        graph.add_connection(add_id.clone(), 0, decimal_id.clone(), 0).await;

        // Remove the decimal node (has outgoing connection to add)
        graph.remove_node(decimal_id.clone()).await;

        assert!(!graph.nodes.contains_key(&decimal_id));
        // The add node's input connection should be cleaned up
        let add_node = graph.nodes.get(&add_id).unwrap();
        assert!(add_node.inputs[0].connection.is_none());
    }

    #[tokio::test]
    async fn test_remove_connected_downstream_node() {
        let mut graph = create_test_graph();

        let decimal_id = graph
            .add_node(get_id(), AddNodeType::Operation(Operation::OpNumberInputDecimal), glam::Vec2::ZERO)
            .await;
        let add_id = graph
            .add_node(get_id(), AddNodeType::Operation(Operation::OpNumberMathAdd), glam::Vec2::new(200.0, 0.0))
            .await;

        graph.add_connection(add_id.clone(), 0, decimal_id.clone(), 0).await;

        // Remove the downstream add node
        graph.remove_node(add_id.clone()).await;

        assert!(!graph.nodes.contains_key(&add_id));
        // The decimal node's output connection should be cleaned up
        let decimal_node = graph.nodes.get(&decimal_id).unwrap();
        let conns = decimal_node.outputs[0].connection.as_ref();
        assert!(conns.is_none() || conns.unwrap().is_empty());
    }

    // === add_connection edge cases ===

    #[tokio::test]
    async fn test_add_connection_nonexistent_input_node() {
        let mut graph = create_test_graph();
        let decimal_id = graph
            .add_node(get_id(), AddNodeType::Operation(Operation::OpNumberInputDecimal), glam::Vec2::ZERO)
            .await;

        // Try to connect to a node that doesn't exist — should be a no-op
        graph.add_connection("nonexistent".to_string(), 0, decimal_id.clone(), 0).await;

        // decimal node output should have no connection
        let decimal_node = graph.nodes.get(&decimal_id).unwrap();
        assert!(decimal_node.outputs[0].connection.is_none());
    }

    #[tokio::test]
    async fn test_add_connection_nonexistent_output_node() {
        let mut graph = create_test_graph();
        let add_id = graph
            .add_node(get_id(), AddNodeType::Operation(Operation::OpNumberMathAdd), glam::Vec2::ZERO)
            .await;

        graph.add_connection(add_id.clone(), 0, "nonexistent".to_string(), 0).await;

        let add_node = graph.nodes.get(&add_id).unwrap();
        assert!(add_node.inputs[0].connection.is_none());
    }

    // === set_input edge cases ===

    #[tokio::test]
    async fn test_set_input_nonexistent_node() {
        let mut graph = create_test_graph();
        // Should be a no-op, not panic
        graph.set_input("nonexistent".to_string(), 0, Value::Decimal(1.0));
        assert!(graph.nodes.is_empty());
    }

    #[tokio::test]
    async fn test_set_input_out_of_bounds_index() {
        let mut graph = create_test_graph();
        let node_id = graph
            .add_node(get_id(), AddNodeType::Operation(Operation::OpNumberMathAdd), glam::Vec2::ZERO)
            .await;

        // Add node has 2 inputs (indices 0, 1). Index 99 should be a no-op.
        graph.set_input(node_id.clone(), 99, Value::Decimal(1.0));

        // Node should still have original values
        let node = graph.nodes.get(&node_id).unwrap();
        assert_eq!(node.inputs.len(), 2);
    }

    // === set_node_position edge cases ===

    #[tokio::test]
    async fn test_set_position_nonexistent_node() {
        let mut graph = create_test_graph();
        // Should be a no-op, not panic
        graph.set_node_position("nonexistent".to_string(), glam::Vec2::new(100.0, 200.0));
    }

    // === set_save_path ===

    #[test]
    fn test_set_save_path() {
        let (tx_gc, _) = mpsc::channel::<GraphChangedMessage>(32);
        let (tx_nc, _) = mpsc::channel::<NodeChangedMessage>(32);
        let mut graph = Graph::new(get_id(), tx_nc, tx_gc, false).unwrap();

        assert!(graph.save_path.is_none());
        graph.set_save_path(std::path::PathBuf::from("/tmp/test.mangle"));
        assert_eq!(graph.save_path, Some(std::path::PathBuf::from("/tmp/test.mangle")));
    }

    // === run() edge cases ===

    #[tokio::test]
    async fn test_run_empty_graph() {
        let mut graph = create_test_graph();
        // Should return immediately, not panic
        graph.run().await;
        assert!(graph.nodes.is_empty());
    }

    #[tokio::test]
    async fn test_run_clean_graph_no_dirty_nodes() {
        let mut graph = create_test_graph();
        let node_id = graph
            .add_node(get_id(), AddNodeType::Operation(Operation::OpNumberMathAdd), glam::Vec2::ZERO)
            .await;

        graph.set_input(node_id.clone(), 0, Value::Decimal(1.0));
        graph.set_input(node_id.clone(), 1, Value::Decimal(2.0));
        graph.run().await;

        // After run, nodes are no longer dirty. Running again should be a no-op.
        let output_before = match &graph.nodes.get(&node_id).unwrap().outputs[0].value {
            Value::Decimal(v) => *v,
            _ => panic!("Expected Decimal"),
        };

        graph.run().await;

        let output_after = match &graph.nodes.get(&node_id).unwrap().outputs[0].value {
            Value::Decimal(v) => *v,
            _ => panic!("Expected Decimal"),
        };

        assert_eq!(output_before, output_after);
    }

    #[tokio::test]
    async fn test_run_caching_same_inputs() {
        let mut graph = create_test_graph();
        let node_id = graph
            .add_node(get_id(), AddNodeType::Operation(Operation::OpNumberMathAdd), glam::Vec2::ZERO)
            .await;

        graph.set_input(node_id.clone(), 0, Value::Decimal(5.0));
        graph.set_input(node_id.clone(), 1, Value::Decimal(10.0));
        graph.run().await;

        // Set same values again — should use cache
        graph.set_input(node_id.clone(), 0, Value::Decimal(5.0));
        graph.set_input(node_id.clone(), 1, Value::Decimal(10.0));
        graph.run().await;

        match &graph.nodes.get(&node_id).unwrap().outputs[0].value {
            Value::Decimal(v) => assert!((*v - 15.0).abs() < 1e-6),
            other => panic!("Expected Decimal, got {:?}", other),
        }
    }

    #[tokio::test]
    async fn test_run_cache_invalidation_on_changed_input() {
        let mut graph = create_test_graph();
        let node_id = graph
            .add_node(get_id(), AddNodeType::Operation(Operation::OpNumberMathAdd), glam::Vec2::ZERO)
            .await;

        graph.set_input(node_id.clone(), 0, Value::Decimal(5.0));
        graph.set_input(node_id.clone(), 1, Value::Decimal(10.0));
        graph.run().await;

        // Change one input — should invalidate cache and recompute
        graph.set_input(node_id.clone(), 1, Value::Decimal(20.0));
        graph.run().await;

        match &graph.nodes.get(&node_id).unwrap().outputs[0].value {
            Value::Decimal(v) => assert!((*v - 25.0).abs() < 1e-6, "Expected 25.0, got {}", v),
            other => panic!("Expected Decimal, got {:?}", other),
        }
    }

    // === run() with chains and fan-out ===

    #[tokio::test]
    async fn test_run_three_node_chain() {
        let mut graph = create_test_graph();

        // decimal(5) → add(_, 10) → add(_, 100)
        let input_id = graph
            .add_node(get_id(), AddNodeType::Operation(Operation::OpNumberInputDecimal), glam::Vec2::ZERO)
            .await;
        let add1_id = graph
            .add_node(get_id(), AddNodeType::Operation(Operation::OpNumberMathAdd), glam::Vec2::new(200.0, 0.0))
            .await;
        let add2_id = graph
            .add_node(get_id(), AddNodeType::Operation(Operation::OpNumberMathAdd), glam::Vec2::new(400.0, 0.0))
            .await;

        graph.set_input(input_id.clone(), 0, Value::Decimal(5.0));
        graph.set_input(add1_id.clone(), 1, Value::Decimal(10.0));
        graph.set_input(add2_id.clone(), 1, Value::Decimal(100.0));

        graph.add_connection(add1_id.clone(), 0, input_id.clone(), 0).await;
        graph.add_connection(add2_id.clone(), 0, add1_id.clone(), 0).await;

        graph.run().await;

        // 5 + 10 = 15, then 15 + 100 = 115
        match &graph.nodes.get(&add2_id).unwrap().outputs[0].value {
            Value::Decimal(v) => assert!((*v - 115.0).abs() < 1e-6, "Expected 115.0, got {}", v),
            other => panic!("Expected Decimal, got {:?}", other),
        }
    }

    #[tokio::test]
    async fn test_run_fan_out() {
        let mut graph = create_test_graph();

        // decimal(10) → add1(_, 1) and add2(_, 2)
        let input_id = graph
            .add_node(get_id(), AddNodeType::Operation(Operation::OpNumberInputDecimal), glam::Vec2::ZERO)
            .await;
        let add1_id = graph
            .add_node(get_id(), AddNodeType::Operation(Operation::OpNumberMathAdd), glam::Vec2::new(200.0, 0.0))
            .await;
        let add2_id = graph
            .add_node(get_id(), AddNodeType::Operation(Operation::OpNumberMathAdd), glam::Vec2::new(200.0, 100.0))
            .await;

        graph.set_input(input_id.clone(), 0, Value::Decimal(10.0));
        graph.set_input(add1_id.clone(), 1, Value::Decimal(1.0));
        graph.set_input(add2_id.clone(), 1, Value::Decimal(2.0));

        // Same output feeds both add nodes
        graph.add_connection(add1_id.clone(), 0, input_id.clone(), 0).await;
        graph.add_connection(add2_id.clone(), 0, input_id.clone(), 0).await;

        graph.run().await;

        match &graph.nodes.get(&add1_id).unwrap().outputs[0].value {
            Value::Decimal(v) => assert!((*v - 11.0).abs() < 1e-6, "Expected 11.0, got {}", v),
            other => panic!("Expected Decimal, got {:?}", other),
        }
        match &graph.nodes.get(&add2_id).unwrap().outputs[0].value {
            Value::Decimal(v) => assert!((*v - 12.0).abs() < 1e-6, "Expected 12.0, got {}", v),
            other => panic!("Expected Decimal, got {:?}", other),
        }
    }

    #[tokio::test]
    async fn test_run_value_propagation_through_connection() {
        let mut graph = create_test_graph();

        let decimal_id = graph
            .add_node(get_id(), AddNodeType::Operation(Operation::OpNumberInputDecimal), glam::Vec2::ZERO)
            .await;
        let add_id = graph
            .add_node(get_id(), AddNodeType::Operation(Operation::OpNumberMathAdd), glam::Vec2::new(200.0, 0.0))
            .await;

        graph.set_input(decimal_id.clone(), 0, Value::Decimal(42.0));
        graph.set_input(add_id.clone(), 1, Value::Decimal(0.0));
        graph.add_connection(add_id.clone(), 0, decimal_id.clone(), 0).await;

        graph.run().await;

        // The add node's input 0 should have received the propagated value
        match &graph.nodes.get(&add_id).unwrap().inputs[0].value {
            Value::Decimal(v) => assert!((*v - 42.0).abs() < 1e-6, "Expected propagated 42.0, got {}", v),
            other => panic!("Expected Decimal input, got {:?}", other),
        }
    }

    // === save_to_file / load round-trip ===

    #[tokio::test]
    async fn test_save_and_load_round_trip() {
        let mut graph = create_test_graph();
        let graph_id = graph.id.clone();

        let node_id = graph
            .add_node(get_id(), AddNodeType::Operation(Operation::OpNumberMathAdd), glam::Vec2::new(50.0, 75.0))
            .await;
        graph.set_input(node_id.clone(), 0, Value::Decimal(42.0));

        let tmp_path = std::env::temp_dir().join(format!("test_graph_{}.mangle", get_id()));
        graph.set_save_path(tmp_path.clone());
        graph.save_to_file();

        // Load it back
        let (tx_nc, _) = mpsc::channel::<NodeChangedMessage>(32);
        let (tx_gc, _) = mpsc::channel::<GraphChangedMessage>(32);
        let loaded = Graph::load(tmp_path.clone(), Some(tx_nc), Some(tx_gc), false).unwrap();

        assert_eq!(loaded.id, graph_id);
        assert!(loaded.nodes.contains_key(&node_id));
        let loaded_node = loaded.nodes.get(&node_id).unwrap();
        assert_eq!(loaded_node.settings.name, "add");
        assert_eq!(loaded_node.position, glam::Vec2::new(50.0, 75.0));
        match &loaded_node.inputs[0].value {
            Value::Decimal(v) => assert_eq!(*v, 42.0),
            other => panic!("Expected Decimal, got {:?}", other),
        }

        // Clean up
        let _ = std::fs::remove_file(tmp_path);
    }

    #[tokio::test]
    async fn test_save_to_file_subgraph_is_noop() {
        let (tx_gc, _) = mpsc::channel::<GraphChangedMessage>(32);
        let (tx_nc, _) = mpsc::channel::<NodeChangedMessage>(32);
        let mut graph = Graph::new(get_id(), tx_nc, tx_gc, true).unwrap();

        let tmp_path = std::env::temp_dir().join(format!("test_subgraph_{}.mangle", get_id()));
        graph.set_save_path(tmp_path.clone());
        graph.save_to_file();

        // File should NOT be created for subgraphs
        assert!(!tmp_path.exists());
    }

    #[tokio::test]
    async fn test_save_to_file_no_path_is_noop() {
        let mut graph = create_test_graph();
        assert!(graph.save_path.is_none());
        // Should be a no-op, not panic
        graph.save_to_file();
    }

    // === load() error cases ===

    #[test]
    fn test_load_nonexistent_file() {
        let result = Graph::load(
            std::path::PathBuf::from("/nonexistent/path/graph.mangle"),
            None, None, false,
        );
        assert!(result.is_err());
    }

    #[test]
    fn test_load_invalid_json() {
        let tmp_path = std::env::temp_dir().join(format!("test_bad_json_{}.mangle", get_id()));
        std::fs::write(&tmp_path, "this is not valid json").unwrap();

        let result = Graph::load(tmp_path.clone(), None, None, false);
        assert!(result.is_err());

        let _ = std::fs::remove_file(tmp_path);
    }

    // === remove_node on nonexistent node ===

    #[tokio::test]
    async fn test_remove_nonexistent_node() {
        let mut graph = create_test_graph();
        // Should be a no-op, not panic
        graph.remove_node("nonexistent".to_string()).await;
        assert!(graph.nodes.is_empty());
    }

    // === remove_connection on unconnected input ===

    #[tokio::test]
    async fn test_remove_connection_when_none_exists() {
        let mut graph = create_test_graph();
        let add_id = graph
            .add_node(get_id(), AddNodeType::Operation(Operation::OpNumberMathAdd), glam::Vec2::ZERO)
            .await;

        // Input 0 has no connection — should be a no-op, not panic
        graph.remove_connection(add_id.clone(), 0).await;

        let add_node = graph.nodes.get(&add_id).unwrap();
        assert!(add_node.inputs[0].connection.is_none());
    }

    // === add multiple nodes, remove all ===

    #[tokio::test]
    async fn test_add_and_remove_multiple_nodes() {
        let mut graph = create_test_graph();

        let id1 = graph.add_node(get_id(), AddNodeType::Operation(Operation::OpNumberInputDecimal), glam::Vec2::ZERO).await;
        let id2 = graph.add_node(get_id(), AddNodeType::Operation(Operation::OpNumberInputInteger), glam::Vec2::ZERO).await;
        let id3 = graph.add_node(get_id(), AddNodeType::Operation(Operation::OpNumberMathAdd), glam::Vec2::ZERO).await;

        assert_eq!(graph.nodes.len(), 3);

        graph.remove_node(id1).await;
        graph.remove_node(id2).await;
        graph.remove_node(id3).await;

        assert_eq!(graph.nodes.len(), 0);
    }

    // === run() propagates updated upstream value downstream ===

    #[tokio::test]
    async fn test_run_upstream_change_propagates() {
        let mut graph = create_test_graph();

        let input_id = graph
            .add_node(get_id(), AddNodeType::Operation(Operation::OpNumberInputDecimal), glam::Vec2::ZERO)
            .await;
        let add_id = graph
            .add_node(get_id(), AddNodeType::Operation(Operation::OpNumberMathAdd), glam::Vec2::new(200.0, 0.0))
            .await;

        graph.set_input(input_id.clone(), 0, Value::Decimal(5.0));
        graph.set_input(add_id.clone(), 1, Value::Decimal(10.0));
        graph.add_connection(add_id.clone(), 0, input_id.clone(), 0).await;

        graph.run().await;

        match &graph.nodes.get(&add_id).unwrap().outputs[0].value {
            Value::Decimal(v) => assert!((*v - 15.0).abs() < 1e-6),
            other => panic!("Expected Decimal, got {:?}", other),
        }

        // Change the upstream input
        graph.set_input(input_id.clone(), 0, Value::Decimal(100.0));
        graph.run().await;

        match &graph.nodes.get(&add_id).unwrap().outputs[0].value {
            Value::Decimal(v) => assert!((*v - 110.0).abs() < 1e-6, "Expected 110.0, got {}", v),
            other => panic!("Expected Decimal, got {:?}", other),
        }
    }
}
