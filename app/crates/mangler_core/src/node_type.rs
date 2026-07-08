//! Discriminant that determines how a node executes: as a concrete operation
//! or as an embedded subgraph.

use std::path::PathBuf;
use std::time::SystemTime;
use serde::{Deserialize, Serialize};
use crate::{operations::Operation, graph::Graph};

/// The execution variant of a node.
///
/// Every node in the graph is either a self-contained [`Operation`] that runs
/// directly, a [`Subgraph`](NodeType::Subgraph) that loads and executes an
/// entire child graph from a `.mangle.json` file on disk, or an
/// [`Unknown`](NodeType::Unknown) placeholder standing in for a node type
/// this build doesn't recognize (see [`crate::saved_nodes`]).
#[derive(Serialize, Deserialize, Debug)]
pub enum NodeType {
    /// A concrete operation node that performs a single computation.
    Operation {
        /// The operation that defines this node's behavior.
        operation: Operation,
    },
    /// A subgraph node that embeds an entire child graph for composition.
    Subgraph {
        /// Path to the `.mangle.json` file containing the subgraph definition.
        path: PathBuf,
        /// The loaded child graph instance (not serialized).
        ///
        /// Exposed outputs are read directly from this graph's node storage
        /// after each run — no message channel sits between parent and child.
        #[serde(skip)]
        graph: Option<Graph>,
        /// Modified-time of `path` at the moment the child was last loaded.
        /// Used by `Graph::check_subgraphs_for_changes` to detect external edits
        /// (e.g. the child being saved from another tab) and trigger a reload.
        /// Not serialized — recomputed by `set_subgraph_path` on every load.
        #[serde(skip)]
        last_mtime: Option<SystemTime>,
    },
    /// A placeholder for a node whose type could not be recognized while
    /// loading — most commonly a graph saved by a *newer* NodeMangler that
    /// introduced an `Operation` variant (or other node shape) this build
    /// doesn't know about. `raw` holds the node's complete original JSON so
    /// it can be written back out verbatim on the next save (only its
    /// `position` and per-socket `connection` fields are patched from live
    /// edits — see [`crate::saved_nodes`]), preserving any fields this build
    /// can't interpret.
    ///
    /// Never produced by `NodeType`'s own (derived) deserializer — only by
    /// [`crate::node::Node::placeholder_from_raw`], called from
    /// `saved_nodes::deserialize` when a node fails to parse as a normal
    /// [`crate::node::Node`].
    Unknown {
        /// The complete original JSON object for this node, exactly as read
        /// from the save file.
        raw: serde_json::Value,
    },
}

/// Manual `Clone` implementation because `Graph` is not cloneable.
/// Cloning a subgraph node produces a shell without the loaded graph.
impl Clone for NodeType {
    fn clone(&self) -> Self {
        match self {
            NodeType::Operation { operation } => {
                NodeType::Operation { operation: operation.clone() }
            },
            // Only clone the path; the graph and mtime must be re-derived
            NodeType::Subgraph { path, graph: _, last_mtime: _ } => {
                NodeType::Subgraph {
                    path: path.clone(),
                    graph: None,
                    last_mtime: None,
                }
            }
            NodeType::Unknown { raw } => {
                NodeType::Unknown { raw: raw.clone() }
            }
        }
    }
}