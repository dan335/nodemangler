//! Discriminant that determines how a node executes: as a concrete operation
//! or as an embedded subgraph.

use std::path::PathBuf;
use serde::{Deserialize, Serialize};
use tokio::sync::mpsc::{Receiver};
use crate::{operations::Operation, graph::Graph, NodeChangedMessage};

/// The execution variant of a node.
///
/// Every node in the graph is either a self-contained [`Operation`] that runs
/// directly, or a [`Subgraph`](NodeType::Subgraph) that loads and executes an
/// entire child graph from a `.mangle` file on disk.
#[derive(Serialize, Deserialize, Debug)]
pub enum NodeType {
    /// A concrete operation node that performs a single computation.
    Operation {
        /// The operation that defines this node's behavior.
        operation: Operation,
    },
    /// A subgraph node that embeds an entire child graph for composition.
    Subgraph {
        /// Path to the `.mangle` file containing the subgraph definition.
        path: PathBuf,
        /// The loaded child graph instance (not serialized).
        #[serde(skip)]
        graph: Option<Graph>,
        /// Channel receiver for node-changed messages from the child graph (not serialized).
        #[serde(skip)]
        rx_node_changed: Option<Receiver<NodeChangedMessage>>,
    }
}

/// Manual `Clone` implementation because `Graph` and `Receiver` are not cloneable.
/// Cloning a subgraph node produces a shell without the loaded graph or channel.
impl Clone for NodeType {
    fn clone(&self) -> Self {
        match self {
            NodeType::Operation { operation } => {
                NodeType::Operation { operation: operation.clone() }
            },
            // Only clone the path; the graph and channel must be re-created
            NodeType::Subgraph {path, graph:_, rx_node_changed:_ } => {
                NodeType::Subgraph { path: path.clone(), graph: None, rx_node_changed: None }
            }
        }
    }
}