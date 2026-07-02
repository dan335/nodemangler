//! Discriminant that determines how a node executes: as a concrete operation
//! or as an embedded subgraph.

use std::path::PathBuf;
use std::time::SystemTime;
use serde::{Deserialize, Serialize};
use crate::{operations::Operation, graph::Graph};

/// The execution variant of a node.
///
/// Every node in the graph is either a self-contained [`Operation`] that runs
/// directly, or a [`Subgraph`](NodeType::Subgraph) that loads and executes an
/// entire child graph from a `.mangle.json` file on disk.
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
    }
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
        }
    }
}