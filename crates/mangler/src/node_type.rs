use std::path::PathBuf;
use serde::{Deserialize, Serialize};
use tokio::sync::mpsc::{Receiver};
use crate::{operation::Operation, graph::Graph, NodeChangedMessage};

#[derive(Serialize, Deserialize, Debug)]
pub enum NodeType {
    Operation {
        operation: Operation,
    },
    Subgraph {
        path: PathBuf,
        #[serde(skip)]
        graph: Option<Graph>,
        #[serde(skip)]
        rx_node_changed: Option<Receiver<NodeChangedMessage>>,
    }
}

impl Clone for NodeType {
    fn clone(&self) -> Self {
        match self {
            NodeType::Operation { operation } => {
                NodeType::Operation { operation: operation.clone() }
            },
            NodeType::Subgraph {path, graph:_, rx_node_changed:_ } => {
                NodeType::Subgraph { path: path.clone(), graph: None, rx_node_changed: None }
            }
        }
    }
}