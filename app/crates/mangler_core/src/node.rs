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

/// Default value for `is_enabled` — used by serde to handle old save files
/// that don't have the field.
fn default_true() -> bool {
    true
}

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
    /// Skipped during serialization — transient execution state.
    #[serde(skip)]
    pub time: Option<Duration>,
    /// Whether this node needs to be re-run on the next graph execution pass.
    pub is_dirty: bool,
    /// 2D position on the graph editor canvas.
    pub position: Vec2,
    /// Whether this node is an operation or a subgraph.
    pub node_type: NodeType,
    /// Whether the last execution resulted in an error.
    /// Skipped during serialization — transient execution state.
    #[serde(skip)]
    pub is_error: bool,
    /// Human-readable error message from the last failed execution.
    /// Skipped during serialization — transient execution state.
    #[serde(skip)]
    pub error_message: Option<String>,
    /// Hash of all input values from the last successful run, used to skip
    /// re-execution when inputs have not changed. Not serialized.
    #[serde(skip)]
    pub cached_input_hash: Option<u64>,
    /// Whether this node is enabled. Disabled nodes skip their operation and
    /// pass the first type-matching input through to each output (passthrough).
    #[serde(default = "default_true")]
    pub is_enabled: bool,
    /// Optional user-defined display name. When set, this is shown as the
    /// primary label on the node; the operation name becomes a secondary label.
    #[serde(default)]
    pub custom_name: Option<String>,
}

/// Send a [`NodeChangedMessage`] to the UI without blocking, logging the actual
/// message kind (`kind`) if the channel is full or closed. `try_send` semantics:
/// the message is dropped on failure.
fn try_send_node_changed(tx: &Sender<NodeChangedMessage>, kind: &str, message: NodeChangedMessage) {
    if let Err(err) = tx.try_send(message) {
        println!("Error sending NodeChangedMessage::{}: {:?}", kind, err);
    }
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
                is_error: false,
                error_message: None,
                cached_input_hash: None,
                is_enabled: true,
                custom_name: None,
            },
            AddNodeType::Subgraph => Node {
                id,
                settings: NodeSettings {
                    name: "subgraph".to_string(),
                    description: "A subgraph.".to_string(),
                    help: "A subgraph embeds another graph as a single node. Inputs and outputs exposed on the child surface here as sockets on the parent, so the subgraph behaves like any other operation.\n\nPick the subgraph file from the settings panel; its exposed I/O rebuild automatically when the file changes.".to_string(),
                },
                inputs: Vec::new(),
                outputs: Vec::new(),
                time: None,
                is_dirty: true,
                position,
                node_type: NodeType::Subgraph {
                    path: PathBuf::new(),
                    graph: None,
                    last_mtime: None,
                },
                is_error: false,
                error_message: None,
                cached_input_hash: None,
                is_enabled: true,
                custom_name: None,
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

    /// Build a placeholder `Node` from a node's raw JSON when it failed to
    /// deserialize as a normal `Node` — typically because it was saved by a
    /// newer NodeMangler that introduced an `Operation` variant (or other
    /// node shape) this build doesn't recognize. Called from
    /// `saved_nodes::deserialize`; see [`crate::saved_nodes`] for the full
    /// tolerant-load story.
    ///
    /// The placeholder carries `raw` verbatim on its `node_type` so it can be
    /// written back out almost byte-for-byte on the next save (only position
    /// and connections get patched — see `saved_nodes::serialize`). Sockets
    /// (inputs/outputs) are recovered best-effort: forward-compat breakage is
    /// almost always just the operation string changing, so the surrounding
    /// input/output JSON usually still parses and wires/values render
    /// normally even though the node itself can't run. When they don't
    /// parse, the node simply comes back with no sockets rather than
    /// aborting the whole graph load.
    pub fn placeholder_from_raw(id: String, raw: serde_json::Value) -> Node {
        // Recover the canvas position if present and well-formed; otherwise
        // default to the origin so the node still renders somewhere.
        let position = raw
            .get("position")
            .and_then(|p| serde_json::from_value::<Vec2>(p.clone()).ok())
            .unwrap_or(Vec2::ZERO);

        // Best-effort label: prefer the (now-unrecognized) operation variant
        // name so the user can see what the node used to be. `operation` can
        // serialize either as a bare string ("OpFoo", for unit variants) or
        // as `{"OpFoo": {...}}` (variants with fields) — handle both. Fall
        // back to whatever settings.name survived, if anything did.
        let op_name = raw
            .get("node_type")
            .and_then(|nt| nt.get("Operation"))
            .and_then(|o| o.get("operation"))
            .and_then(|op| match op {
                serde_json::Value::String(s) => Some(s.clone()),
                serde_json::Value::Object(map) => map.keys().next().cloned(),
                _ => None,
            })
            .or_else(|| {
                raw.get("settings")
                    .and_then(|s| s.get("name"))
                    .and_then(|n| n.as_str())
                    .map(|s| s.to_string())
            });

        let name = match op_name {
            Some(op) => format!("unknown: {op}"),
            None => "unknown node".to_string(),
        };

        // Best-effort socket recovery: if the inputs/outputs JSON no longer
        // matches Input/Output's shape, fall back to empty rather than
        // letting the failure propagate — an empty-socket placeholder is far
        // better than losing the whole graph load.
        let inputs: Vec<Input> = raw
            .get("inputs")
            .and_then(|v| serde_json::from_value(v.clone()).ok())
            .unwrap_or_default();
        let outputs: Vec<Output> = raw
            .get("outputs")
            .and_then(|v| serde_json::from_value(v.clone()).ok())
            .unwrap_or_default();

        Node {
            id,
            settings: NodeSettings {
                name,
                description: "This node's type was not recognized — likely saved by a newer \
                    version of NodeMangler. It will not run, but its position, values, and \
                    connections are preserved and will be written back out unchanged."
                    .to_string(),
                help: String::new(),
            },
            inputs,
            outputs,
            time: None,
            // Dirty so the graph runs it once (and reports the error via
            // `Node::run`'s Unknown arm); the input-hash cache then prevents
            // re-run spam on subsequent ticks.
            is_dirty: true,
            position,
            node_type: NodeType::Unknown { raw },
            is_error: true,
            error_message: Some("Unknown node type — saved with a newer NodeMangler?".to_string()),
            cached_input_hash: None,
            is_enabled: true,
            custom_name: None,
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
    ///
    /// `thumbnail_service` is consulted for `Value::Image` outputs; see
    /// [`crate::thumbnail_service`]. When `None` (detached render graphs,
    /// tests with no UI), thumbnails are computed inline instead.
    pub async fn run(
        &mut self,
        tx_node_changed: Option<Sender<NodeChangedMessage>>,
        thumbnail_service: Option<&crate::thumbnail_service::ThumbnailService>,
    ) {
        match &mut self.node_type {
            // if node is an operation
            NodeType::Operation { operation } => {
                // run operation
                // collect results

                if let Some(tx) = tx_node_changed.clone() {
                    let message = NodeChangedMessage::Busy { node_id: self.id.clone(), is_busy: true };
                    try_send_node_changed(&tx, "Busy", message);
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
                        try_send_node_changed(tx, "Error", message);
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
                                input_index,
                                is_error: false,
                                message: None,
                            };
                            try_send_node_changed(tx, "InputErrorChanged", message);
                        }
                    }
                }

                // Image operations are CPU-bound and can run for hundreds of
                // ms; executing them inline would pin this runtime worker and
                // starve every other task (thumbnail service, sibling graphs).
                // Move the compute to the blocking pool; inputs are moved in
                // and handed back so ops keep their `&mut [Input]` contract.
                let operation_clone = operation.clone();
                let mut op_inputs = std::mem::take(&mut self.inputs);
                let run_result = match tokio::task::spawn_blocking(move || {
                    let result = tokio::runtime::Handle::current()
                        .block_on(operation_clone.run(&mut op_inputs));
                    (result, op_inputs)
                })
                .await
                {
                    Ok((result, inputs)) => {
                        self.inputs = inputs;
                        result
                    }
                    Err(join_error) => std::panic::resume_unwind(join_error.into_panic()),
                };

                match run_result {
                    Ok(operation_response) => {
                        // time node took to run
                        self.time = Some(operation_response.time);

                        if let Some(tx) = tx_node_changed.clone() {
                            let message = NodeChangedMessage::InfoChanged {
                                node_id: self.id.clone(),
                                time: operation_response.time,
                            };
                            try_send_node_changed(&tx, "InfoChanged", message);
                        }

                        // TODO: change response to a Result?
                        for (index, response) in operation_response.responses.into_iter().enumerate() {
                            // send messages to ui that outputs changed
                            if let Some(tx) = tx_node_changed.clone() {
                                // Image thumbnails are slow (resize + to_rgba8 of a
                                // full-resolution FloatImage). When the async
                                // service is available, enqueue instead of
                                // computing inline; the UI receives a follow-up
                                // ThumbnailReady once ready. Scalar/enum
                                // thumbnails stay inline because they're trivial.
                                // No UI channel (nodes inside subgraphs, detached
                                // render graphs) → skip thumbnails entirely;
                                // nothing would consume them.
                                let thumbnail = match &response.value {
                                    Value::Image { data, change_id }
                                        if thumbnail_service.is_some() =>
                                    {
                                        thumbnail_service.unwrap().request(
                                            self.id.clone(),
                                            index,
                                            change_id.clone(),
                                            std::sync::Arc::clone(data),
                                        );
                                        None
                                    }
                                    _ => response.value.create_thumbnail(),
                                };

                                let message = NodeChangedMessage::OutputChanged {
                                    node_id: self.id.clone(),
                                    output_index: index,
                                    value: response.value.clone(),
                                    thumbnail,
                                };
                                try_send_node_changed(&tx, "OutputChanged", message);
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
                                        input_index: *input_index,
                                        is_error: true,
                                        message: Some(error_message.clone()),
                                    };
                                    try_send_node_changed(&tx, "InputErrorChanged", message);
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
                            try_send_node_changed(&tx, "Error", message);
                        }
                    },
                }

                if let Some(tx) = tx_node_changed.clone() {
                    let message = NodeChangedMessage::Busy { node_id: self.id.clone(), is_busy: false };
                    try_send_node_changed(&tx, "Busy", message);
                }
            }

            // if node is a subgraph
            NodeType::Subgraph {
                path: _,
                graph: subgraph_option,
                last_mtime: _,
            } => {
                if let Some(subgraph) = subgraph_option {
                    // pass node's input to subgraph's input before running
                    for input in self.inputs.iter() {
                        if let Value::Path(_) = input.value {
                            // nothing
                        } else if let Some(link) = &input.link {
                            if let Some(subgraph_node) =
                                subgraph.nodes.get_mut(&link.node_id)
                            {
                                if let Some(i) = subgraph_node
                                    .inputs
                                    .iter_mut()
                                    .position(|i| i.id == link.input_id)
                                {
                                    // Only forward when the value actually
                                    // changed: set_input_value dirties the
                                    // child node, and an unconditional set
                                    // forces a full child-graph traversal per
                                    // parent run. Triggers always forward —
                                    // their fingerprint is constant, so it
                                    // cannot distinguish a fresh firing.
                                    let unchanged = !matches!(input.value, Value::Trigger)
                                        && subgraph_node.inputs[i].value.fingerprint()
                                            == input.value.fingerprint();
                                    if !unchanged {
                                        subgraph_node.set_input_value(i, input.value.clone());
                                    }
                                }
                            }
                        }
                    }

                    // run subgraph (boxed: Graph::run -> Node::run -> Graph::run
                    // is recursive, so the future needs indirection)
                    Box::pin(subgraph.run()).await;

                    // Copy exposed outputs back by reading the linked values
                    // directly from the child graph's node storage. Each
                    // exposed output carries an OutputLink identifying the
                    // child node id + output index it mirrors.
                    //
                    // Results used to round-trip through a bounded mpsc
                    // channel drained here with try_recv; once the child
                    // graph emitted more messages than the channel capacity,
                    // OutputChanged messages were silently dropped and the
                    // exposed outputs went stale. Direct reads are lossless.
                    // Value::clone is cheap (images are Arc-shared) and the
                    // change_id travels inside Value::Image, so cache
                    // invalidation and stale-thumbnail rejection semantics
                    // are unchanged.
                    for output in self.outputs.iter_mut() {
                        if let Some(link) = &output.link {
                            if let Some(subgraph_node) = subgraph.nodes.get(&link.node_id) {
                                if let Some(subgraph_output) =
                                    subgraph_node.outputs.get(link.output_index)
                                {
                                    output.value = subgraph_output.value.clone();
                                }
                            }
                        }
                    }

                    // let ui know that outputs changed
                    if let Some(tx) = tx_node_changed {
                        for (output_index, output) in self.outputs.iter().enumerate() {
                            let thumbnail = match &output.value {
                                Value::Image { data, change_id }
                                    if thumbnail_service.is_some() =>
                                {
                                    thumbnail_service.unwrap().request(
                                        self.id.clone(),
                                        output_index,
                                        change_id.clone(),
                                        std::sync::Arc::clone(data),
                                    );
                                    None
                                }
                                _ => output.value.create_thumbnail(),
                            };

                            let message = NodeChangedMessage::OutputChanged {
                                node_id: self.id.clone(),
                                output_index,
                                value: output.value.clone(),
                                thumbnail,
                            };
                            try_send_node_changed(&tx, "OutputChanged", message);
                        }
                    }
                }
            }

            // A placeholder standing in for a node type this build doesn't
            // recognize (see `Node::placeholder_from_raw`). It cannot run —
            // there's no operation or subgraph behind it — so just re-affirm
            // the persistent error so the UI (and anyone driving the engine
            // headlessly) knows why this node's outputs never update. No
            // outputs are produced; any downstream node keeps whatever stale
            // value it last received.
            NodeType::Unknown { .. } => {
                self.is_error = true;
                self.error_message =
                    Some("Unknown node type — saved with a newer NodeMangler?".to_string());

                if let Some(tx) = tx_node_changed {
                    let message = NodeChangedMessage::Error {
                        node_id: self.id.clone(),
                        is_error: true,
                        message: self.error_message.clone(),
                    };
                    try_send_node_changed(&tx, "Error", message);
                }
            }
        };
    }
}

#[cfg(test)]
#[path = "node_tests.rs"]
mod tests;
