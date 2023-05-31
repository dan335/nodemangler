use std::path::PathBuf;
use serde::{Deserialize, Serialize};
use crate::{operation::Operation, graph::Graph};

#[derive(Debug, Serialize, Deserialize)]
pub enum NodeType {
    Operation {
        operation: Operation
    },
    Subgraph {
        path: PathBuf,
        #[serde(skip)]
        graph: Option<Graph>
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