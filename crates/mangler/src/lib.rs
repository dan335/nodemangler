use glam::f32::Vec2;
use image::{ImageBuffer, Rgba};
use nanoid::nanoid;
use node::Node;
use operation::Operation;
use serde::{Deserialize, Serialize};
use std::{collections::HashMap, path::PathBuf, time::Duration};
use value::Value;

#[macro_use]
extern crate lazy_static;

pub mod graph;
pub mod input;
pub mod node;
pub mod node_settings;
pub mod operation;
pub mod operations;
pub mod output;
pub mod value;

pub fn get_id() -> String {
    nanoid!()
}

// set a node's input to a value
// node will run next time graph runs
#[derive(Debug)]
pub struct SetNodeInputMessage {
    pub node_id: String,
    pub input_index: usize,
    pub value: Value,
}

// when a node's input changes because of a node connection
// this will let the ui know
#[derive(Debug)]
pub struct NodeInputChangedMessage {
    pub node_id: String,
    pub input_index: usize,
    pub value: Value,
}

// a node has run and it's output has changed
#[derive(Debug, Clone)]
pub struct NodeOutputChangedMessage {
    pub node_id: String,
    pub output_index: usize,
    pub value: Value,
    //pub value_type: ValueType,
    pub time: Duration,
    pub thumbnail: Option<ImageBuffer<Rgba<u8>, Vec<u8>>>,
}

#[derive(Debug)]
pub struct AddNodeMessage {
    pub node_id: String,
    pub operation: Operation,
    pub position: Vec2,
}

#[derive(Debug)]
pub struct AddedNodeMessage {
    pub node_id: String,
    pub operation: Operation,
    pub position: Vec2,
}

#[derive(Debug)]
pub struct LoadedNodeMessage {
    pub node: Node,
}

#[derive(Debug)]
pub struct RemoveNodeMessage {
    pub node_id: String,
}

#[derive(Debug)]
pub struct RemovedNodeMessage {
    pub node_id: String,
}

#[derive(Debug)]
pub struct AddConnectionMessage {
    pub input_node_id: String,
    pub input_connection_index: usize,
    pub output_node_id: String,
    pub output_connection_index: usize,
}

#[derive(Debug)]
pub struct AddedConnectionMessage {
    pub input_node_id: String,
    pub input_connection_index: usize,
    pub output_node_id: String,
    pub output_connection_index: usize,
}

#[derive(Debug)]
pub struct RemoveConnectionMessage {
    pub node_id: String,
    pub input_index: usize,
}

#[derive(Debug)]
pub struct RemovedConnectionMessage {
    pub node_id: String,
    pub input_index: usize,
}

#[derive(Debug)]
pub enum GraphMessage {
    SavePath(PathBuf),
    GraphName(String),
}

#[derive(Debug)]
pub struct NodePosition {
    pub node_id: String,
    pub position: glam::f32::Vec2,
}

#[derive(Debug, Clone)]
pub enum OperationListItem {
    Category {
        name: String,
        operation_list_items: Vec<OperationListItem>,
    },
    Operation {
        operation: Operation,
    },
}

#[derive(Debug)]
pub struct NewGraphError(pub String);

#[derive(Serialize, Deserialize, Debug)]
pub struct GraphSaveData {
    pub id: String,
    pub name: String,
    pub nodes: HashMap<String, Node>,
}

lazy_static! {
    pub static ref OPERATION_LIST: Vec<OperationListItem> = vec![
        OperationListItem::Category { name: "Numbers".to_string(), operation_list_items: vec![
            OperationListItem::Operation { operation: Operation::InputInteger },
            OperationListItem::Category { name: "Numbers".to_string(), operation_list_items: vec![
                OperationListItem::Operation { operation: Operation::InputInteger },
                OperationListItem::Operation { operation: Operation::InputInteger },
            ]},
            OperationListItem::Operation { operation: Operation::InputInteger },
        ]},
        OperationListItem::Category { name: "Numbers".to_string(), operation_list_items: vec![
            OperationListItem::Operation { operation: Operation::InputInteger },
            OperationListItem::Operation { operation: Operation::InputInteger },
        ]},
        // OperationCategory {
        //     name: "Images".to_string(),
        //     operations: vec![
        //         OperationDescription { operation: Operation::ImageFromUrl },
        //         OperationDescription { operation: Operation::ImageResize },
        //         OperationDescription { operation: Operation::ImageFromClipboard },
        //     ]
        // },
        // OperationCategory {
        //     name: "Text".to_string(),
        //     operations: vec![
        //         OperationDescription { operation: Operation::TextFromClipboard },
        //     ]
        // }
    ];
}
