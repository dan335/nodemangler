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

    /// Load a graph from a `.mangle.json` (or `.json`) file on disk.
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
                    error
                ))),
            },
            Err(error) => Err(NewGraphError(format!(
                "Error loading graph. Error: {}",
                error
            ))),
        }
    }

    /// Serialize this graph into a [`GraphSaveData`] snapshot for saving to JSON.
    pub fn to_save_data(&self) -> GraphSaveData {
        GraphSaveData {
            id: self.id.clone(),
            name: self.name.clone(),
            nodes: self.nodes.clone(),
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
    /// which `.mangle.json` file to load. Returns the node ID.
    pub async fn add_node(&mut self, node_id: String, node_type: AddNodeType, position: Vec2) -> String {
        let mut node = Node::new(node_id.clone(), node_type.clone(), position);
        let mut is_subgraph = false;

        if let AddNodeType::Subgraph = node_type {
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
                    if from_output.outputs.len() >= output_connection_index && to_input.inputs.len() >= input_connection_index
                        && from_output.outputs[output_connection_index].is_valid_connection(&to_input.inputs[input_connection_index]) {
                            is_valid = true;
                        }
                }
            }

            if is_valid {
                // If the input already has a connection, remove the stale entry
                // from the old source's output connection list before wiring the
                // new one. Without this, the old source node would still propagate
                // its output into this input during graph execution.
                if let Some(old_conn) = self.nodes.get(&input_node_id)
                    .and_then(|n| n.inputs.get(input_connection_index))
                    .and_then(|inp| inp.connection.clone())
                {
                    let (old_output_node_id, old_output_index) = old_conn;
                    if let Some(old_source) = self.nodes.get_mut(&old_output_node_id) {
                        if let Some(output) = old_source.outputs.get_mut(old_output_index) {
                            if let Some(conns) = output.connection.as_mut() {
                                conns.retain(|item| *item != (input_node_id.clone(), input_connection_index));
                            }
                        }
                    }
                }

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

                // immediately propagate the source node's current output value to the
                // downstream input so the right panel shows the correct value before
                // the graph runs
                let source_value = self.nodes.get(&output_node_id)
                    .map(|n| n.outputs[output_connection_index].value.clone());
                if let Some(value) = source_value {
                    if let Some(node) = self.nodes.get_mut(&input_node_id) {
                        node.inputs[input_connection_index].value = value.clone();
                    }
                    if let Some(tx) = &self.tx_node_changed {
                        let _ = tx.try_send(NodeChangedMessage::InputChanged {
                            node_id: input_node_id.clone(),
                            input_index: input_connection_index,
                            value,
                        });
                    }
                }

                // adapt accepts_any_type inputs/outputs to match the connected type
                let source_type = self.nodes.get(&output_node_id)
                    .map(|n| n.outputs[output_connection_index].value.value_type());
                if let Some(source_type) = source_type {
                    if let Some(node) = self.nodes.get_mut(&input_node_id) {
                        if node.inputs[input_connection_index].accepts_any_type {
                            let default_val = source_type.default_value();
                            // update all accepts_any_type inputs and all outputs to match
                            for input in node.inputs.iter_mut() {
                                if input.accepts_any_type {
                                    input.value = default_val.clone();
                                    input.default_value = default_val.clone();
                                }
                            }
                            for output in node.outputs.iter_mut() {
                                output.value = default_val.clone();
                                output.default_value = default_val.clone();
                            }
                            // notify UI of the type changes
                            if let Some(tx) = &self.tx_node_changed {
                                for (i, input) in node.inputs.iter().enumerate() {
                                    if input.accepts_any_type {
                                        let _ = tx.try_send(NodeChangedMessage::InputChanged {
                                            node_id: input_node_id.clone(),
                                            input_index: i,
                                            value: input.value.clone(),
                                        });
                                    }
                                }
                            }
                        }
                    }
                }

                // mark graph as dirty
                self.is_dirty = true;

                // send message to ui
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

        // re-adapt accepts_any_type inputs/outputs after disconnection
        {
            let is_any_type_input = self.nodes.get(&node_id)
                .and_then(|n| n.inputs.get(input_index))
                .is_some_and(|i| i.accepts_any_type);

            if is_any_type_input {
                // look up the actual source output type for each still-connected accepts_any_type input
                let remaining_source_type = self.nodes.get(&node_id)
                    .and_then(|node| {
                        node.inputs.iter()
                            .filter(|i| i.accepts_any_type && i.connection.is_some())
                            .filter_map(|i| i.connection.as_ref())
                            .next()
                            .cloned()
                    })
                    .and_then(|(src_node_id, src_output_index)| {
                        self.nodes.get(&src_node_id)
                            .and_then(|n| n.outputs.get(src_output_index))
                            .map(|o| o.value.value_type())
                    });

                // adapt to remaining connection's source type, or reset to Decimal
                let new_val = match remaining_source_type {
                    Some(vt) => vt.default_value(),
                    None => Value::Decimal(0.0),
                };

                if let Some(node) = self.nodes.get_mut(&node_id) {
                    for input in node.inputs.iter_mut() {
                        if input.accepts_any_type && input.connection.is_none() {
                            input.value = new_val.clone();
                            input.default_value = new_val.clone();
                        }
                    }
                    for output in node.outputs.iter_mut() {
                        output.value = new_val.clone();
                        output.default_value = new_val.clone();
                    }

                    // mark dirty so the node re-runs and updates its output/thumbnail
                    node.is_dirty = true;

                    // notify UI of the type changes
                    if let Some(tx) = &self.tx_node_changed {
                        for (i, input) in node.inputs.iter().enumerate() {
                            if input.accepts_any_type && input.connection.is_none() {
                                let _ = tx.try_send(NodeChangedMessage::InputChanged {
                                    node_id: node_id.clone(),
                                    input_index: i,
                                    value: input.value.clone(),
                                });
                            }
                        }
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
                                for subgraph_input in subgraph_node.inputs.iter() {
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

            // Passthrough: if the node is disabled, copy the first type-matching
            // input value to each output instead of running the operation.
            let is_disabled = self.nodes.get(&node_id).is_some_and(|n| !n.is_enabled);
            if is_disabled {
                let mut output_data: Vec<(String, usize, Value)> = Vec::new();

                if let Some(node) = self.nodes.get_mut(&node_id) {
                    node.cached_input_hash = Some(input_hash);

                    for (out_idx, output) in node.outputs.iter_mut().enumerate() {
                        let out_type = output.value.value_type();
                        // Find the first input whose type matches this output's type.
                        let passthrough_value = node.inputs.iter()
                            .find(|inp| inp.value.value_type() == out_type)
                            .map(|inp| inp.value.clone())
                            .unwrap_or_else(|| output.default_value.clone());

                        output.value = passthrough_value.clone();

                        // Notify UI of output change.
                        if let Some(tx) = &self.tx_node_changed {
                            let _ = tx.try_send(NodeChangedMessage::OutputChanged {
                                node_id: node_id.clone(),
                                output_index: out_idx,
                                value: passthrough_value.clone(),
                                thumbnail: passthrough_value.create_thumbnail(),
                            });
                        }

                        // Gather downstream connections.
                        if let Some(connections) = &output.connection {
                            for (connected_node_id, input_index) in connections.iter() {
                                output_data.push((
                                    connected_node_id.clone(),
                                    *input_index,
                                    passthrough_value.clone(),
                                ));
                            }
                        }
                    }
                }

                // Propagate passthrough values to downstream nodes.
                for (connected_node_id, input_index, value) in output_data.into_iter() {
                    if let Some(connected_node) = self.nodes.get_mut(&connected_node_id) {
                        if let Some(tx) = &self.tx_node_changed {
                            let _ = tx.try_send(NodeChangedMessage::InputChanged {
                                node_id: connected_node_id.clone(),
                                input_index,
                                value: value.clone(),
                            });
                        }
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
            adjacency.entry(node_id.clone()).or_default();
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
#[path = "graph_tests.rs"]
mod tests;
