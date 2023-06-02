use std::path::PathBuf;
use serde::{Deserialize, Serialize};
use tokio::sync::mpsc::{Receiver, Sender};
use crate::{operation::Operation, graph::Graph, NodeChangedMessage};

#[derive(Debug, Serialize, Deserialize)]
pub enum NodeType {
    Operation {
        operation: Operation
    },
    Subgraph {
        path: PathBuf,
        #[serde(skip)]
        graph: Option<Graph>,
        // #[serde(skip)]
        // tx_node_changed: Option<Sender<NodeChangedMessage>>,
    }
}

impl Clone for NodeType {
    fn clone(&self) -> Self {
        match self {
            NodeType::Operation { operation } => {
                NodeType::Operation { operation: operation.clone() }
            },
            NodeType::Subgraph {path, graph } => {
                NodeType::Subgraph { path: path.clone(), graph: None }
            }
        }
    }
}