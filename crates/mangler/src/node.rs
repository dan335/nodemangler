//! A single node in the processing graph.
//!
//! Each node wraps either an [`Operation`](crate::operations::Operation) or a
//! [`Subgraph`](crate::node_type::NodeType::Subgraph), holds its own inputs and
//! outputs, and tracks execution state such as dirty flags, error status, and
//! cached input hashes for skip-if-unchanged optimization.

use crate::node_type::NodeType;
use crate::{AddNodeType, NodeChangedMessage};
use glam::f32::Vec2;
use serde::{Deserialize, Serialize};
use std::fmt::Debug;
use std::path::PathBuf;
use tokio::sync::mpsc::Sender;
use tokio::time::Duration;
use crate::{input::Input, output::Output, value::Value};
use super::node_settings::NodeSettings;

/// A single node in the processing graph.
///
/// Nodes are the fundamental units of computation. They receive data through
/// their inputs, execute an operation or subgraph, and produce results on
/// their outputs. The graph engine uses dirty tracking and input hashing to
/// avoid redundant re-computation.
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Node {
    /// Unique identifier for this node within the graph.
    pub id: String,
    /// Display metadata (name and description).
    pub settings: NodeSettings,
    /// Ordered list of inputs that feed data into this node.
    pub inputs: Vec<Input>,
    /// Ordered list of outputs that carry results to downstream nodes.
    pub outputs: Vec<Output>,
    /// How long the last execution took, if the node has run.
    pub time: Option<Duration>,
    /// Whether this node needs to be re-run on the next graph execution pass.
    pub is_dirty: bool,
    /// 2D position on the graph editor canvas.
    pub position: Vec2,
    /// Whether this node is an operation or a subgraph.
    pub node_type: NodeType,
    /// Whether the node is currently executing.
    pub is_busy: bool,
    /// Whether the last execution resulted in an error.
    pub is_error: bool,
    /// Human-readable error message from the last failed execution.
    pub error_message: Option<String>,
    /// Hash of all input values from the last successful run, used to skip
    /// re-execution when inputs have not changed. Not serialized.
    #[serde(skip)]
    pub cached_input_hash: Option<u64>,
}

/// Nodes are compared by identity (ID) only, ignoring all other fields.
impl PartialEq for Node {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id
    }
}

impl Node {
    /// Create a new node from a type specification and initial position.
    ///
    /// For operation nodes, the inputs, outputs, and settings are derived from the
    /// operation's trait methods. For subgraph nodes, empty inputs/outputs are created
    /// and will be populated when the subgraph file is loaded.
    pub fn new(id: String, node_type: AddNodeType, position: glam::f32::Vec2) -> Self {
        match node_type {            
            AddNodeType::Operation(operation) => Node {
                id,
                settings: operation.settings(),
                inputs: operation.create_inputs(),
                outputs: operation.create_outputs(),
                time: None,
                is_dirty: true,
                position,
                node_type: NodeType::Operation { operation },
                is_busy: false,
                is_error: false,
                error_message: None,
                cached_input_hash: None,
            },
            AddNodeType::Subgraph => Node {
                id,
                settings: NodeSettings {
                    name: "subgraph".to_string(),
                    description: "A subgraph.".to_string(),
                },
                inputs: Vec::new(),
                outputs: Vec::new(),
                time: None,
                is_dirty: true,
                position,
                node_type: NodeType::Subgraph {
                    path: PathBuf::new(),
                    graph: None,
                    rx_node_changed: None,
                },
                is_busy: false,
                is_error: false,
                error_message: None,
                cached_input_hash: None,
            },
        }
    }

    /// Set the value of an input at the given index and mark the node as dirty.
    ///
    /// # Panics
    /// Panics if `index` is out of bounds.
    pub fn set_input_value(&mut self, index: usize, value: Value) {
        if let Some(input) = self.inputs.get_mut(index) {
            input.value = value; //value = value;
            self.is_dirty = true;
        } else {
            panic!("Invalid input index: {}", index);
        }
    }

    /// Get a reference to the input at the given index.
    ///
    /// # Panics
    /// Panics if `index` is out of bounds.
    pub fn get_input(&self, index: usize) -> &Input {
        &self.inputs[index]
    }

    /// Get a reference to the full list of inputs.
    pub fn get_inputs(&self) -> &Vec<Input> {
        &self.inputs
    }

    /// Record that this node's input at `input_index` is connected to the output
    /// of node `output_id` at `output_index`.
    pub fn set_input_connection(
        &mut self,
        input_index: usize,
        output_id: String,
        output_index: usize,
    ) {
        self.inputs[input_index].connection = Some((output_id, output_index));
    }

    /// Remove the connection from this node's input at `input_index`.
    pub fn clear_input_connection(&mut self, input_index: usize) {
        self.inputs[input_index].connection = None;
    }

    /// Record that this node's output at `output_index` feeds into the input of
    /// node `input_id` at `input_index`. Outputs support fan-out (multiple connections).
    pub fn set_output_connection(
        &mut self,
        output_index: usize,
        input_id: String,
        input_index: usize,
    ) {
        if self.outputs[output_index].connection.is_some() {
            self.outputs[output_index]
                .connection
                .as_mut()
                .unwrap()
                .push((input_id, input_index));
        } else {
            self.outputs[output_index].connection = Some(vec![(input_id, input_index)]);
        }
    }

    /// Execute this node: run its operation or subgraph, update outputs, and
    /// send state-change messages to the UI via `tx_node_changed`.
    ///
    /// For operation nodes, this clears errors, runs the operation, stores results,
    /// generates thumbnails, and sends busy/error/output-changed messages.
    ///
    /// For subgraph nodes, this propagates inputs into the child graph, runs it,
    /// and copies exposed outputs back to this node's outputs.
    pub async fn run(&mut self, tx_node_changed: Option<Sender<NodeChangedMessage>>) {
        match &mut self.node_type {
            // if node is an operation
            NodeType::Operation { operation } => {
                // run operation
                // collect results

                if let Some(tx) = tx_node_changed.clone() {
                    let message = NodeChangedMessage::Busy { node_id: self.id.clone(), is_busy: true };

                    match tx.try_send(message) {
                        Ok(_) => {}
                        Err(err) => {
                            println!(
                                "Error sending NodeChangedMessage::OutputChanged: {:?}",
                                err
                            );
                        }
                    }
                }

                // clear node error
                if self.is_error {
                    self.is_error = false;

                    if let Some(tx) = &tx_node_changed {
                        let message = NodeChangedMessage::Error {
                            node_id: self.id.clone(),
                            is_error: false,
                            message: None,
                        };
    
                        match tx.try_send(message) {
                            Ok(_) => {}
                            Err(err) => {
                                println!("Error sending NodeChangedMessage::InputChanged: {:?}", err);
                            }
                        }
                    }
                }

                // clear input errors
                // notify ui of change
                for (input_index, input) in self.inputs.iter_mut().enumerate() {
                    if input.is_error {
                        input.is_error = false;

                        if let Some(tx) = &tx_node_changed {
                            let message = NodeChangedMessage::InputErrorChanged {
                                node_id: self.id.clone(),
                                input_index: input_index,
                                is_error: false,
                                message: None,
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

                match operation.run(&mut self.inputs).await {
                    Ok(operation_response) => {
                        // time node took to run
                        self.time = Some(operation_response.time);

                        if let Some(tx) = tx_node_changed.clone() {
                            let message = NodeChangedMessage::InfoChanged {
                                node_id: self.id.clone(),
                                time: operation_response.time,
                            };

                            match tx.try_send(message) {
                                Ok(_) => {}
                                Err(err) => {
                                    println!(
                                        "Error sending NodeChangedMessage::OutputChanged: {:?}",
                                        err
                                    );
                                }
                            }
                        }

                        // TODO: change response to a Result?
                        for (index, response) in operation_response.responses.into_iter().enumerate() {
                            // send messages to ui that outputs changed
                            if let Some(tx) = tx_node_changed.clone() {
                                let message = NodeChangedMessage::OutputChanged {
                                    node_id: self.id.clone(),
                                    output_index: index,
                                    value: response.value.clone(),
                                    thumbnail: response.value.create_thumbnail(),
                                };

                                match tx.try_send(message) {
                                    Ok(_) => {}
                                    Err(err) => {
                                        println!(
                                            "Error sending NodeChangedMessage::OutputChanged: {:?}",
                                            err
                                        );
                                    }
                                }
                            }

                            // set output's value
                            if let Some(output) = self.outputs.get_mut(index) {
                                output.value = response.value;
                            }
                        }
                    },
                    Err(operation_error) => {
                        // store node error message or none
                        let mut node_error_message: Option<String> = operation_error.node_error.clone();

                        // update inputs
                        // send input error messages
                        for input_error in operation_error.input_errors.iter() {
                            let (input_index, error_message) = input_error;

                            if let Some(input) = self.inputs.get_mut(*input_index) {
                                input.is_error = true;
                                input.error_message = Some(error_message.clone());

                                // if node error is empty fill it in
                                if node_error_message.is_none() {
                                    node_error_message = Some("Input error.".to_string());
                                }

                                // send message
                                if let Some(tx) = tx_node_changed.clone() {
                                    let message = NodeChangedMessage::InputErrorChanged {
                                        node_id: self.id.clone(),
                                        input_index: input_index.clone(),
                                        is_error: true,
                                        message: Some(error_message.clone()),
                                    };
    
                                    match tx.try_send(message) {
                                        Ok(_) => {}
                                        Err(err) => {
                                            println!(
                                                "Error sending NodeChangedMessage::OutputChanged: {:?}",
                                                err
                                            );
                                        }
                                    }
                                }
                            } else {
                                panic!("Invalid input index: {}", input_index);
                            }
                        }

                        // set node error
                        self.is_error = true;
                        self.error_message = node_error_message.clone();

                        // send node error changed
                        if let Some(tx) = tx_node_changed.clone() {
                            let message = NodeChangedMessage::Error {
                                node_id: self.id.clone(),
                                is_error: true,
                                message: node_error_message,
                            };

                            match tx.try_send(message) {
                                Ok(_) => {}
                                Err(err) => {
                                    println!(
                                        "Error sending NodeChangedMessage::OutputChanged: {:?}",
                                        err
                                    );
                                }
                            }
                        }
                    },
                }

                if let Some(tx) = tx_node_changed.clone() {
                    let message = NodeChangedMessage::Busy { node_id: self.id.clone(), is_busy: false };

                    match tx.try_send(message) {
                        Ok(_) => {}
                        Err(err) => {
                            println!(
                                "Error sending NodeChangedMessage::OutputChanged: {:?}",
                                err
                            );
                        }
                    }
                }
            }

            // if node is a subgraph
            NodeType::Subgraph {
                path: _,
                graph: subgraph_option,
                rx_node_changed,
            } => {
                match subgraph_option {
                    Some(subgraph) => {
                        // pass node's input to subgraph's input before running
                        for (_input_index, input) in self.inputs.iter().enumerate() {
                            if let Value::Path(_) = input.value {
                                // nothing
                            } else {
                                if let Some(link) = &input.link {
                                    if let Some(subgraph_node) =
                                        subgraph.nodes.get_mut(&link.node_id)
                                    {
                                        if let Some(i) = subgraph_node
                                            .inputs
                                            .iter_mut()
                                            .position(|i| i.id == link.input_id)
                                        {
                                            subgraph_node.set_input_value(i, input.value.clone());
                                        }
                                    }
                                }
                            }
                        }

                        // run subgraph
                        subgraph.run().await;

                        // receive messages about which nodes changed in subgraph
                        // if one changed that is exposed then pass it's output to this node's output
                        if let Some(rx) = rx_node_changed {
                            // receive messages
                            while let Ok(node_changed_message) = rx.try_recv() {
                                match node_changed_message {
                                    NodeChangedMessage::OutputChanged {
                                        node_id: subgraph_node_id,
                                        output_index: subgraph_output_index,
                                        value: subgraph_value,
                                        thumbnail: _subgraph_thumbnail,
                                    } => {
                                        // find output that is linked to subgraph output that changed
                                        for (_output_index, output) in
                                            self.outputs.iter_mut().enumerate()
                                        {
                                            if let Some(link) = &mut output.link {
                                                if link.node_id == subgraph_node_id
                                                    && link.output_index == subgraph_output_index
                                                {
                                                    // set output value to subgraph's new value
                                                    output.value = subgraph_value.clone();
                                                }
                                            }
                                        }

                                        //self.time = Some(subgraph_time);
                                    }
                                    // don't care about other messages
                                    _ => {}
                                }
                            }
                        }

                        // let ui know that outputs changed
                        if let Some(tx) = tx_node_changed {
                            for (output_index, output) in self.outputs.iter().enumerate() {
                                let message = NodeChangedMessage::OutputChanged {
                                    node_id: self.id.clone(),
                                    output_index,
                                    value: output.value.clone(),
                                    thumbnail: output.value.create_thumbnail(),
                                };

                                match tx.try_send(message) {
                                    Ok(_) => {}
                                    Err(err) => {
                                        println!(
                                            "Error sending NodeChangedMessage::OutputChanged: {:?}",
                                            err
                                        );
                                    }
                                }
                            }
                        }
                    }
                    None => {}
                }
            }
        };
    }
}

#[cfg(test)]
mod tests {
    use glam::f32::Vec2;
    use crate::input::Input;
    use crate::node_type::NodeType;
    use crate::operations::Operation;
    use crate::output::Output;
    use crate::value::Value;
    use crate::AddNodeType;
    use super::*;

    // === Helper ===

    fn make_operation_node() -> Node {
        Node::new(
            "test-node".to_string(),
            AddNodeType::Operation(Operation::OpLogicBoolNot),
            Vec2::new(10.0, 20.0),
        )
    }

    fn make_subgraph_node() -> Node {
        Node::new(
            "sub-node".to_string(),
            AddNodeType::Subgraph,
            Vec2::new(0.0, 0.0),
        )
    }

    /// Build a node with a specific number of inputs and outputs for connection tests.
    fn make_node_with_io(id: &str, num_inputs: usize, num_outputs: usize) -> Node {
        let mut node = make_operation_node();
        node.id = id.to_string();
        node.inputs = (0..num_inputs)
            .map(|i| Input::new(format!("in_{i}"), Value::Decimal(0.0), None, None))
            .collect();
        node.outputs = (0..num_outputs)
            .map(|i| Output::new(format!("out_{i}"), Value::Decimal(0.0), None))
            .collect();
        node
    }

    // === new ===

    #[test]
    fn test_new_operation_node() {
        let node = make_operation_node();
        assert_eq!(node.id, "test-node");
        assert_eq!(node.settings.name, "not");
        assert!(node.is_dirty);
        assert!(!node.is_busy);
        assert!(!node.is_error);
        assert!(node.error_message.is_none());
        assert!(node.cached_input_hash.is_none());
        assert!(node.time.is_none());
        assert_eq!(node.position, Vec2::new(10.0, 20.0));
        assert_eq!(node.inputs.len(), 1);
        assert_eq!(node.outputs.len(), 1);
        assert!(matches!(node.node_type, NodeType::Operation { .. }));
    }

    #[test]
    fn test_new_subgraph_node() {
        let node = make_subgraph_node();
        assert_eq!(node.id, "sub-node");
        assert_eq!(node.settings.name, "subgraph");
        assert!(node.is_dirty);
        assert!(node.inputs.is_empty());
        assert!(node.outputs.is_empty());
        assert!(matches!(node.node_type, NodeType::Subgraph { .. }));
    }

    // === PartialEq ===

    #[test]
    fn test_partial_eq_same_id() {
        let a = make_operation_node();
        let mut b = a.clone();
        b.is_dirty = false;
        b.position = Vec2::new(999.0, 999.0);
        assert_eq!(a, b);
    }

    #[test]
    fn test_partial_eq_different_id() {
        let a = make_operation_node();
        let mut b = a.clone();
        b.id = "other-id".to_string();
        assert_ne!(a, b);
    }

    // === set_input_value ===

    #[test]
    fn test_set_input_value_sets_value_and_marks_dirty() {
        let mut node = make_operation_node();
        node.is_dirty = false;
        node.set_input_value(0, Value::Bool(true));
        assert!(matches!(node.inputs[0].value, Value::Bool(true)));
        assert!(node.is_dirty);
    }

    #[test]
    fn test_set_input_value_multiple_times() {
        let mut node = make_node_with_io("n", 3, 0);
        node.set_input_value(0, Value::Decimal(1.0));
        node.set_input_value(1, Value::Decimal(2.0));
        node.set_input_value(2, Value::Decimal(3.0));
        assert!(matches!(node.inputs[0].value, Value::Decimal(v) if v == 1.0));
        assert!(matches!(node.inputs[1].value, Value::Decimal(v) if v == 2.0));
        assert!(matches!(node.inputs[2].value, Value::Decimal(v) if v == 3.0));
    }

    #[test]
    fn test_set_input_value_last_index() {
        let mut node = make_node_with_io("n", 5, 0);
        node.set_input_value(4, Value::Decimal(42.0));
        assert!(matches!(node.inputs[4].value, Value::Decimal(v) if v == 42.0));
    }

    #[test]
    #[should_panic(expected = "Invalid input index: 1")]
    fn test_set_input_value_out_of_bounds_panics() {
        let mut node = make_operation_node(); // 1 input
        node.set_input_value(1, Value::Bool(false));
    }

    #[test]
    #[should_panic(expected = "Invalid input index: 0")]
    fn test_set_input_value_on_empty_inputs_panics() {
        let mut node = make_subgraph_node(); // 0 inputs
        node.set_input_value(0, Value::Bool(false));
    }

    #[test]
    #[should_panic(expected = "Invalid input index: 100")]
    fn test_set_input_value_way_out_of_bounds_panics() {
        let mut node = make_operation_node();
        node.set_input_value(100, Value::Bool(false));
    }

    // === get_input / get_inputs ===

    #[test]
    fn test_get_input() {
        let node = make_operation_node();
        let input = node.get_input(0);
        assert_eq!(input.name, "input");
    }

    #[test]
    #[should_panic]
    fn test_get_input_out_of_bounds_panics() {
        let node = make_operation_node();
        let _ = node.get_input(5);
    }

    #[test]
    fn test_get_inputs() {
        let node = make_node_with_io("n", 3, 0);
        let inputs = node.get_inputs();
        assert_eq!(inputs.len(), 3);
    }

    // === set_input_connection / clear_input_connection ===

    #[test]
    fn test_set_input_connection() {
        let mut node = make_node_with_io("n", 2, 0);
        node.set_input_connection(0, "other-node".to_string(), 3);
        assert_eq!(
            node.inputs[0].connection,
            Some(("other-node".to_string(), 3))
        );
        // Other input untouched
        assert!(node.inputs[1].connection.is_none());
    }

    #[test]
    fn test_set_input_connection_overwrite() {
        let mut node = make_node_with_io("n", 1, 0);
        node.set_input_connection(0, "a".to_string(), 0);
        node.set_input_connection(0, "b".to_string(), 1);
        assert_eq!(node.inputs[0].connection, Some(("b".to_string(), 1)));
    }

    #[test]
    fn test_clear_input_connection() {
        let mut node = make_node_with_io("n", 1, 0);
        node.set_input_connection(0, "other".to_string(), 0);
        assert!(node.inputs[0].connection.is_some());
        node.clear_input_connection(0);
        assert!(node.inputs[0].connection.is_none());
    }

    #[test]
    fn test_clear_input_connection_already_none() {
        let mut node = make_node_with_io("n", 1, 0);
        node.clear_input_connection(0);
        assert!(node.inputs[0].connection.is_none());
    }

    #[test]
    #[should_panic]
    fn test_set_input_connection_out_of_bounds_panics() {
        let mut node = make_node_with_io("n", 1, 0);
        node.set_input_connection(5, "x".to_string(), 0);
    }

    #[test]
    #[should_panic]
    fn test_clear_input_connection_out_of_bounds_panics() {
        let mut node = make_node_with_io("n", 1, 0);
        node.clear_input_connection(5);
    }

    // === set_output_connection ===

    #[test]
    fn test_set_output_connection_first() {
        let mut node = make_node_with_io("n", 0, 2);
        node.set_output_connection(0, "target".to_string(), 1);
        let conn = node.outputs[0].connection.as_ref().unwrap();
        assert_eq!(conn.len(), 1);
        assert_eq!(conn[0], ("target".to_string(), 1));
        assert!(node.outputs[1].connection.is_none());
    }

    #[test]
    fn test_set_output_connection_append_multiple() {
        let mut node = make_node_with_io("n", 0, 1);
        node.set_output_connection(0, "a".to_string(), 0);
        node.set_output_connection(0, "b".to_string(), 1);
        node.set_output_connection(0, "c".to_string(), 2);
        let conn = node.outputs[0].connection.as_ref().unwrap();
        assert_eq!(conn.len(), 3);
        assert_eq!(conn[0], ("a".to_string(), 0));
        assert_eq!(conn[1], ("b".to_string(), 1));
        assert_eq!(conn[2], ("c".to_string(), 2));
    }

    #[test]
    fn test_set_output_connection_duplicate_allowed() {
        let mut node = make_node_with_io("n", 0, 1);
        node.set_output_connection(0, "a".to_string(), 0);
        node.set_output_connection(0, "a".to_string(), 0);
        let conn = node.outputs[0].connection.as_ref().unwrap();
        assert_eq!(conn.len(), 2);
    }

    #[test]
    fn test_set_output_connection_different_outputs() {
        let mut node = make_node_with_io("n", 0, 3);
        node.set_output_connection(0, "a".to_string(), 0);
        node.set_output_connection(1, "b".to_string(), 1);
        node.set_output_connection(2, "c".to_string(), 2);
        assert_eq!(node.outputs[0].connection.as_ref().unwrap().len(), 1);
        assert_eq!(node.outputs[1].connection.as_ref().unwrap().len(), 1);
        assert_eq!(node.outputs[2].connection.as_ref().unwrap().len(), 1);
    }

    #[test]
    #[should_panic]
    fn test_set_output_connection_out_of_bounds_panics() {
        let mut node = make_node_with_io("n", 0, 1);
        node.set_output_connection(5, "x".to_string(), 0);
    }
}
