//! Core library for the NodeMangler node-based visual programming engine.
//!
//! This crate provides the value system, node graph engine, operations, and color
//! space support. Communication between the GUI and the engine is driven by four
//! message types sent over `tokio::sync::mpsc` channels:
//!
//! - [`ChangeNodeMessage`] -- UI to engine: mutate individual node state.
//! - [`NodeChangedMessage`] -- Engine to UI: notify that node state changed.
//! - [`ChangeGraphMessage`] -- UI to engine: mutate graph structure.
//! - [`GraphChangedMessage`] -- Engine to UI: notify that graph structure changed.

use glam::f32::Vec2;
use input::Input;
use nanoid::nanoid;
use node::Node;
use node_settings::NodeSettings;
use operations::Operation;
use output::Output;
use serde::{Deserialize, Serialize};
use thumbnail::Thumbnail;
use std::{collections::HashMap, path::PathBuf, time::Duration};
use value::Value;

pub mod graph;
pub mod input;
pub mod node;
pub mod node_settings;
pub mod operations;
pub mod output;
pub mod value;
pub mod node_type;
pub mod thumbnail;
pub mod app;
pub mod dynamic_image_serde;
pub mod float_image;
pub mod float_image_serde;
pub mod color;
pub mod video;
#[cfg(feature = "video")]
pub mod render;
mod tests;

/// Generate a unique identifier using nanoid.
///
/// Used to assign stable IDs to nodes, inputs, outputs, and graphs.
pub fn get_id() -> String {
    nanoid!()
}

/// Messages sent from the UI to the engine to modify a single node's state.
#[derive(Debug)]
pub enum ChangeNodeMessage {
    /// Set the value of a specific input on a node.
    SetInput {
        /// The target node's unique identifier.
        node_id: String,
        /// Zero-based index of the input to modify.
        input_index: usize,
        /// The new value to assign to the input.
        value: Value,
    },
    /// Move a node to a new canvas position.
    SetPosition {
        /// The target node's unique identifier.
        node_id: String,
        /// The new 2D position on the graph canvas.
        position: glam::f32::Vec2,
    },
    /// Toggle whether a node input is exposed to the parent graph (for subgraphs).
    SetExposeInput {
        /// The target node's unique identifier.
        node_id: String,
        /// Zero-based index of the input to expose or hide.
        input_index: usize,
        /// `true` to expose, `false` to hide.
        set_to: bool,
    },
    /// Toggle whether a node output is exposed to the parent graph (for subgraphs).
    SetExposeOutput {
        /// The target node's unique identifier.
        node_id: String,
        /// Zero-based index of the output to expose or hide.
        output_index: usize,
        /// `true` to expose, `false` to hide.
        set_to: bool,
    },
    /// Enable or disable a node. Disabled nodes skip their operation and
    /// pass the first type-matching input through to each output.
    SetEnabled {
        /// The target node's unique identifier.
        node_id: String,
        /// `true` to enable, `false` to disable.
        set_to: bool,
    },
    /// Set or clear the user-defined custom name for a node.
    SetCustomName {
        /// The target node's unique identifier.
        node_id: String,
        /// The new custom name, or `None` to clear it.
        name: Option<String>,
    },
    /// Point a Subgraph node at a `.mangle.json` file on disk. The engine
    /// loads the child graph and populates the node's inputs/outputs from the
    /// child's exposed slots.
    SetSubgraphPath {
        /// The target node's unique identifier.
        node_id: String,
        /// Path to the child `.mangle.json` file.
        path: std::path::PathBuf,
    },
}

/// Messages sent from the engine to the UI when a node's state changes.
#[derive(Debug)]
pub enum NodeChangedMessage {
    /// An input value was updated (e.g. from an upstream connection).
    InputChanged {
        /// The affected node's unique identifier.
        node_id: String,
        /// Zero-based index of the changed input.
        input_index: usize,
        /// The new input value.
        value: Value,
    },
    /// An input's error state changed (validation failure or recovery).
    InputErrorChanged {
        /// The affected node's unique identifier.
        node_id: String,
        /// Zero-based index of the input.
        input_index: usize,
        /// Whether the input is currently in an error state.
        is_error: bool,
        /// Optional human-readable error description.
        message: Option<String>,
    },
    /// An output value was recomputed after a node run.
    OutputChanged {
        /// The affected node's unique identifier.
        node_id: String,
        /// Zero-based index of the changed output.
        output_index: usize,
        /// The new output value.
        value: Value,
        /// An optional thumbnail preview generated from the output value.
        thumbnail: Option<Thumbnail>,
    },
    /// An input's exposed state changed (for subgraph composition).
    ExposeInputChanged {
        /// The affected node's unique identifier.
        node_id: String,
        /// Zero-based index of the input.
        input_index: usize,
        /// Whether the input is now exposed.
        set_to: bool,
    },
    /// An output's exposed state changed (for subgraph composition).
    ExposeOutputChanged {
        /// The affected node's unique identifier.
        node_id: String,
        /// Zero-based index of the output.
        output_index: usize,
        /// Whether the output is now exposed.
        set_to: bool,
    },
    /// A subgraph node finished loading its child graph from disk.
    SubgraphLoaded {
        /// The subgraph node's unique identifier.
        node_id: String,
        /// Display settings derived from the loaded subgraph.
        settings: NodeSettings,
        /// Inputs created from the subgraph's exposed inputs.
        inputs: Vec<Input>,
        /// Outputs created from the subgraph's exposed outputs.
        outputs: Vec<Output>,
    },
    /// A node's busy state changed (started or finished processing).
    Busy {
        /// The affected node's unique identifier.
        node_id: String,
        /// `true` when the node begins processing, `false` when it completes.
        is_busy: bool,
    },
    /// A node's error state changed (operation failure or recovery).
    Error {
        /// The affected node's unique identifier.
        node_id: String,
        /// Whether the node is currently in an error state.
        is_error: bool,
        /// Optional human-readable error description.
        message: Option<String>,
    },
    /// Timing information updated after a node finishes running.
    InfoChanged {
        /// The affected node's unique identifier.
        node_id: String,
        /// How long the node's operation took to execute.
        time: Duration,
    },
    /// The entire graph run completed.
    GraphRunCompleted {
        /// Total wall-clock time for the graph execution pass.
        total_time: Duration,
    },
}

/// Messages sent from the UI to the engine to modify graph structure.
#[derive(Debug)]
pub enum ChangeGraphMessage {
    /// Add a new node to the graph.
    AddNode {
        /// Unique identifier for the new node.
        node_id: String,
        /// Whether to create an operation node or a subgraph node.
        node_type: AddNodeType,
        /// Initial canvas position.
        position: Vec2,
        /// Whether the node starts enabled. Defaults to `true`.
        is_enabled: bool,
        /// Optional user-defined display name.
        custom_name: Option<String>,
    },
    /// Remove a node and clean up all its connections.
    RemoveNode {
        /// The node to remove.
        node_id: String,
    },
    /// Create a connection between an output and an input.
    AddConnection {
        /// The node receiving data (downstream).
        input_node_id: String,
        /// Index of the input on the receiving node.
        input_connection_index: usize,
        /// The node providing data (upstream).
        output_node_id: String,
        /// Index of the output on the providing node.
        output_connection_index: usize,
    },
    /// Remove a connection from a specific input.
    RemoveConnection {
        /// The node whose input connection should be removed.
        node_id: String,
        /// Index of the input to disconnect.
        input_index: usize,
    },
    /// Set the file path where the graph will be saved.
    SetSavePath(PathBuf),
    /// Set the human-readable name for the graph.
    SetGraphName(String),
    /// Begin rendering a video out of the graph. The engine snapshots the
    /// graph and drives the render on a separate tokio task; the live engine
    /// keeps running normally. Progress is reported via `RenderProgress` /
    /// `RenderFinished` / `RenderFailed` on the graph-changed channel.
    StartRender {
        /// The Video Output node whose inputs (path, format, fps, duration)
        /// drive the render and whose `image` input supplies each frame.
        output_node_id: String,
    },
}

/// Messages sent from the engine to the UI when graph structure changes.
#[derive(Debug)]
pub enum GraphChangedMessage {
    /// A new node was added to the graph.
    AddedNode {
        /// The new node's unique identifier.
        node_id: String,
        /// Display settings for the node.
        settings: NodeSettings,
        /// The node's inputs.
        inputs: Vec<Input>,
        /// The node's outputs.
        outputs: Vec<Output>,
        /// Initial canvas position.
        position: Vec2,
        /// Whether this node is a subgraph container.
        is_subgraph: bool,
        /// The node type used to create this node (for copy/paste).
        node_type: AddNodeType,
        /// Whether this node is enabled.
        is_enabled: bool,
        /// Optional user-defined display name.
        custom_name: Option<String>,
    },
    /// A node was restored from a saved graph file.
    LoadedNode {
        /// The fully deserialized node.
        node: Node,
    },
    /// A node was removed from the graph.
    RemovedNode {
        /// The removed node's unique identifier.
        node_id: String,
    },
    /// A connection was established between two nodes.
    AddedConnection {
        /// The downstream node receiving data.
        input_node_id: String,
        /// Index of the input on the downstream node.
        input_connection_index: usize,
        /// The upstream node providing data.
        output_node_id: String,
        /// Index of the output on the upstream node.
        output_connection_index: usize,
    },
    /// A connection was removed from an input.
    RemovedConnection {
        /// The node whose input was disconnected.
        node_id: String,
        /// Index of the disconnected input.
        input_index: usize,
    },
    /// A render task has advanced by one or more frames.
    RenderProgress {
        /// How many frames have been pushed to the encoder so far.
        frame: u32,
        /// Total number of frames the render will produce.
        total: u32,
    },
    /// A render task has finished successfully.
    RenderFinished {
        /// The output file path that was written.
        path: PathBuf,
        /// Wall-clock time spent rendering.
        elapsed: Duration,
    },
    /// A render task failed.
    RenderFailed {
        /// Human-readable description of the failure.
        message: String,
    },
}

/// Specifies what kind of node to create when adding to a graph.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AddNodeType {
    /// A concrete operation node that performs computation.
    Operation(Operation),
    /// A subgraph node that embeds an entire child graph.
    Subgraph
}

/// Error returned when graph creation or loading fails.
#[derive(Debug)]
pub struct NewGraphError(pub String);

/// Serializable snapshot of a graph for saving to and loading from JSON files.
#[derive(Serialize, Deserialize, Debug)]
pub struct GraphSaveData {
    /// Unique identifier for this graph.
    pub id: String,
    /// Human-readable name for this graph.
    pub name: String,
    /// All nodes in the graph, keyed by node ID.
    pub nodes: HashMap<String, Node>,
}


