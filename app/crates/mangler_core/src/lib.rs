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
pub mod curve;
pub mod node_type;
pub mod thumbnail;
pub mod thumbnail_service;
pub mod app;
pub mod float_image;
pub mod float_image_serde;
pub mod color;
pub mod version;
pub mod saved_nodes;
pub mod naming;
pub mod run_context;
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
    /// Point a Subgraph node at a `.mangler.json` file on disk. The engine
    /// loads the child graph and populates the node's inputs/outputs from the
    /// child's exposed slots.
    SetSubgraphPath {
        /// The target node's unique identifier.
        node_id: String,
        /// Path to the child `.mangler.json` file.
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
    ///
    /// For `Value::Image` outputs the engine sets `thumbnail: None` and the
    /// actual thumbnail follows via [`NodeChangedMessage::ThumbnailReady`]
    /// once the async [`crate::thumbnail_service::ThumbnailService`] has
    /// computed it. All other value types have their (cheap) thumbnail
    /// computed inline on the engine thread and delivered here.
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
    /// A thumbnail computed asynchronously for an image output. The UI
    /// matches `change_id` against the current output value's change_id and
    /// discards the thumbnail if the output has since been superseded (stale
    /// write guard — see
    /// [`crate::thumbnail_service::ThumbnailService`]).
    ThumbnailReady {
        /// The affected node's unique identifier.
        node_id: String,
        /// Zero-based index of the output.
        output_index: usize,
        /// The `Value::Image.change_id` of the value the thumbnail was built
        /// from. Must match the current output's change_id for the thumbnail
        /// to be applied; otherwise drop.
        change_id: String,
        /// The computed thumbnail.
        thumbnail: Thumbnail,
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
        /// Initial input overrides `(input_index, value)` applied before the
        /// node is echoed back — used by paste/duplicate so the recreated node
        /// carries the copied values instead of defaults. Empty for a plain add.
        input_values: Vec<(usize, Value)>,
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
    /// Rename the graph's file on disk. `new_stem` is a user-entered display
    /// name; the engine sanitizes it, appends the canonical extension, and
    /// physically renames the file (see [`crate::graph::Graph::rename_file`]).
    /// The name is a pure function of the file stem, so there is no separate
    /// "set name" message anymore. On success the engine replies with
    /// [`GraphChangedMessage::FileRenamed`]; on failure (e.g. a name
    /// collision) with [`GraphChangedMessage::SaveError`].
    RenameFile {
        /// The new display name / file stem the user typed.
        new_stem: String,
    },
    /// Resolve a detected external-modification conflict (see
    /// [`GraphChangedMessage::FileConflict`]): the save file was rewritten
    /// by someone else while local edits were pending.
    ResolveFileConflict {
        /// `true` to overwrite the file with the in-memory graph (discarding
        /// the external edit); `false` to discard local edits and reload the
        /// file from disk instead. This is a resolution action, not itself
        /// an edit — the engine loop does not treat it as one for auto-save
        /// debounce purposes.
        keep_ours: bool,
    },
    /// Start a batch run over a "from folder" node: run the graph once per
    /// image file in that node's folder, stepping the node's `index` input
    /// from 0 to count-1 (one iteration per engine tick, so the UI stays
    /// responsive and thumbnails stream live) with output saving forced on
    /// for each iteration. Progress is reported via
    /// [`GraphChangedMessage::BatchProgress`] and the run ends with
    /// [`GraphChangedMessage::BatchFinished`]. Ignored if a batch is already
    /// running. Like [`ChangeGraphMessage::ResolveFileConflict`], this is not
    /// itself an edit — it does not trigger the auto-save debounce.
    RunBatch {
        /// The "from folder" node whose folder is iterated.
        node_id: String,
    },
    /// Stop the active batch run after the in-flight iteration completes.
    /// The engine restores the from-folder node's `index` input to its
    /// pre-batch value and replies with [`GraphChangedMessage::BatchFinished`]
    /// (`cancelled: true`). A no-op when no batch is running.
    CancelBatch,
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
    /// Sent once, immediately after a top-level graph finishes loading from
    /// disk — *before* any [`GraphChangedMessage::LoadedNode`] for that same
    /// load — when the load found something the user should know about: the
    /// file was written by a newer NodeMangler, and/or it contained one or
    /// more nodes that failed to parse and were replaced by placeholders
    /// (see [`crate::saved_nodes`]). Never sent for subgraph children (their
    /// `Graph::load` is always called with `tx_graph_changed: None`).
    LoadWarnings {
        /// The `version` string stamped in the loaded file (empty string for
        /// pre-versioning saves).
        file_version: String,
        /// Whether `file_version` is newer than this build's
        /// [`APP_VERSION`] — see [`crate::version::is_newer_than_app`].
        is_newer_than_app: bool,
        /// Display names of nodes that were replaced with placeholders.
        unknown_nodes: Vec<String>,
    },
    /// The graph's save file was modified externally (another tab, another
    /// machine on a network share, ...) while local edits were pending.
    /// Auto-save pauses — the write that would have happened is skipped —
    /// until the user resolves the conflict via
    /// [`ChangeGraphMessage::ResolveFileConflict`]. Sent at most once per
    /// conflict (the engine loop guards re-sending while unresolved).
    FileConflict {
        /// Path to the save file that was modified externally.
        path: PathBuf,
    },
    /// The UI should discard all current graph-editor state (nodes,
    /// selection, in-progress connection) because the graph is about to be
    /// replaced wholesale. Sent immediately before a fresh
    /// [`GraphChangedMessage::LoadedNode`] stream, e.g. when resolving a
    /// [`ChangeGraphMessage::ResolveFileConflict`] with `keep_ours: false`.
    GraphCleared,
    /// The graph's file was renamed on disk in response to a
    /// [`ChangeGraphMessage::RenameFile`]. The UI should adopt `new_path` as
    /// its save path; the tab title follows automatically because the display
    /// name is derived from the path.
    FileRenamed {
        /// The path the file was renamed to.
        new_path: PathBuf,
    },
    /// The engine wrote the graph to `path` in direct response to a
    /// [`ChangeGraphMessage::SetSavePath`] (first save of an unsaved graph,
    /// or save-as). Confirms the file exists on disk, so the UI can safely
    /// complete a close that was waiting on a save instead of racing the
    /// debounced auto-save.
    SavedTo {
        /// The path the graph was written to.
        path: PathBuf,
    },
    /// Writing the graph's save file failed (e.g. the parent directory
    /// vanished, or a permissions/disk-space error). Previously this only
    /// reached an `eprintln!` — this variant gives the UI a channel to
    /// surface it (see [`crate::graph::Graph::save_to_file`]).
    SaveError {
        /// The path the engine attempted to write to.
        path: PathBuf,
        /// Human-readable error description.
        message: String,
    },
    /// One iteration of an active batch run (see
    /// [`ChangeGraphMessage::RunBatch`]) finished: the graph has fully run
    /// with the from-folder node's `index` at `completed - 1` and any output
    /// nodes have written their files for that item. Sent once per item, in
    /// order.
    BatchProgress {
        /// The "from folder" node being iterated.
        node_id: String,
        /// Number of iterations finished so far (1-based; equals `total` on
        /// the last progress message).
        completed: usize,
        /// Total number of image files in the batch.
        total: usize,
    },
    /// The batch run ended — every file was processed, the user cancelled,
    /// the iterated node was deleted mid-run, or the run could not start at
    /// all (no valid from-folder node / empty or unreadable folder — reported
    /// as `cancelled: true` with `total: 0`). The node's `index` input has
    /// been restored to its pre-batch value and forced saving is off again.
    BatchFinished {
        /// The "from folder" node that was iterated.
        node_id: String,
        /// Number of iterations that fully completed.
        completed: usize,
        /// Total number of image files the batch set out to process.
        total: usize,
        /// `true` when the run ended early (user cancel, node deleted, or
        /// failure to start) rather than by finishing every file.
        cancelled: bool,
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

/// The application version, inherited from `[workspace.package] version` in
/// `app/Cargo.toml`. Stamped into every saved graph file.
pub const APP_VERSION: &str = env!("CARGO_PKG_VERSION");

/// Serializable snapshot of a graph for saving to and loading from JSON files.
#[derive(Serialize, Deserialize, Debug)]
pub struct GraphSaveData {
    /// NodeMangler version that wrote this file (see [`APP_VERSION`]).
    /// Empty string for files saved before versioning was added.
    #[serde(default)]
    pub version: String,
    /// Unique identifier for this graph.
    pub id: String,
    /// Human-readable name for this graph.
    pub name: String,
    /// All nodes in the graph, keyed by node ID. Deserialized/serialized
    /// tolerantly through [`saved_nodes`] so that a node type this build
    /// doesn't recognize (e.g. saved by a newer NodeMangler) becomes a
    /// placeholder instead of failing the whole graph load.
    #[serde(with = "crate::saved_nodes")]
    pub nodes: HashMap<String, Node>,
}

/// Summary of anomalies detected while loading a saved graph — used to warn
/// the user (see [`GraphChangedMessage::LoadWarnings`]) and to decide
/// whether auto-save should be held until the user makes an edit (see the
/// engine loop's `hold_saves` flag in `app.rs`).
///
/// [`crate::graph::Graph::load`] always populates this (as `Some`) for a
/// real file load, even when nothing anomalous was found — `None` on
/// [`crate::graph::Graph`] means "this graph was never loaded from a file"
/// (e.g. a brand-new graph from [`crate::graph::Graph::new`]).
#[derive(Debug, Clone)]
pub struct LoadReport {
    /// The `version` string stamped in the loaded file (empty string for
    /// pre-versioning saves).
    pub file_version: String,
    /// Whether `file_version` is newer than this build's [`APP_VERSION`] —
    /// see [`version::is_newer_than_app`].
    pub is_newer_than_app: bool,
    /// Display names of any nodes that failed to parse as a known node type
    /// and were replaced with placeholders (see [`saved_nodes`] /
    /// [`node::Node::placeholder_from_raw`]).
    pub unknown_node_names: Vec<String>,
}


