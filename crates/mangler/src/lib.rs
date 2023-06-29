use glam::f32::Vec2;
use input::Input;
use nanoid::nanoid;
use node::Node;
use node_settings::NodeSettings;
use operation::Operation;
use output::Output;
use serde::{Deserialize, Serialize};
use thumbnail::Thumbnail;
use std::{collections::HashMap, path::PathBuf, time::Duration};
use value::Value;

pub mod graph;
pub mod input;
pub mod node;
pub mod node_settings;
pub mod operation;
pub mod operations;
pub mod output;
pub mod value;
pub mod node_type;
pub mod thumbnail;
pub mod app;
pub mod color;
mod graph_tests;

pub fn get_id() -> String {
    nanoid!()
}



#[derive(Debug)]
pub enum ChangeNodeMessage {
    SetInput {
        node_id: String,
        input_index: usize,
        value: Value,
    },
    SetPosition {
        node_id: String,
        position: glam::f32::Vec2,
    },
    SetExposeInput {
        node_id: String,
        input_index: usize,
        set_to: bool,
    },
    SetExposeOutput {
        node_id: String,
        output_index: usize,
        set_to: bool,
    }
}

#[derive(Debug)]
pub enum NodeChangedMessage {
    InputChanged {
        node_id: String,
        input_index: usize,
        value: Value,
    },
    OutputChanged {
        node_id: String,
        output_index: usize,
        value: Value,
        thumbnail: Option<Thumbnail>,
    },
    ExposeInputChanged {
        node_id: String,
        input_index: usize,
        set_to: bool,
    },
    ExposeOutputChanged {
        node_id: String,
        output_index: usize,
        set_to: bool,
    },
    SubgraphLoaded {
        node_id: String,
        settings: NodeSettings,
        inputs: Vec<Input>,
        outputs: Vec<Output>,
    },
    Busy {
        node_id: String,
        is_busy: bool,
    },
    InfoChanged {
        node_id: String,
        time: Duration,
    }
}

#[derive(Debug)]
pub enum ChangeGraphMessage {
    AddNode {
        node_id: String,
        node_type: AddNodeType,
        position: Vec2,
    },
    RemoveNode {
        node_id: String,
    },
    AddConnection {
        input_node_id: String,
        input_connection_index: usize,
        output_node_id: String,
        output_connection_index: usize,
    },
    RemoveConnection {
        node_id: String,
        input_index: usize,
    },
    SetSavePath(PathBuf),
    SetGraphName(String),
}

#[derive(Debug)]
pub enum GraphChangedMessage {
    AddedNode {
        node_id: String,
        settings: NodeSettings,
        inputs: Vec<Input>,
        outputs: Vec<Output>,
        position: Vec2,
        is_subgraph: bool,
    },
    LoadedNode {
        node: Node,
    },
    RemovedNode {
        node_id: String,
    },
    AddedConnection {
        input_node_id: String,
        input_connection_index: usize,
        output_node_id: String,
        output_connection_index: usize,
    },
    RemovedConnection {
        node_id: String,
        input_index: usize,
    },
}

#[derive(Debug, Clone)]
pub enum AddNodeType {
    Operation(Operation),
    Subgraph
}

// #[derive(Debug)]
// pub struct NodePosition {
//     pub node_id: String,
//     pub position: glam::f32::Vec2,
// }

#[derive(Debug, Clone)]
pub enum OperationListItem {
    Category {
        name: String,
        operation_list_items: Vec<OperationListItem>,
    },
    Operation {
        operation: Operation,
    },
    Subgraph
}

#[derive(Debug)]
pub struct NewGraphError(pub String);

#[derive(Serialize, Deserialize, Debug)]
pub struct GraphSaveData {
    pub id: String,
    pub name: String,
    pub nodes: HashMap<String, Node>,
}

pub fn operation_list() -> Vec<OperationListItem> {
    vec![
        OperationListItem::Category { name: "numbers".to_string(), operation_list_items: vec![
            OperationListItem::Category { name: "input".to_string(), operation_list_items: vec![
                OperationListItem::Operation { operation: Operation::NumberInputDecimal },
                OperationListItem::Operation { operation: Operation::NumberInputInteger },
            ]},
            OperationListItem::Category { name: "arithmetic".to_string(), operation_list_items: vec![
                OperationListItem::Operation { operation: Operation::NumberMathAdd },
                OperationListItem::Operation { operation: Operation::NumberMathSubtract },
                OperationListItem::Operation { operation: Operation::NumberMathMultiply },
                OperationListItem::Operation { operation: Operation::NumberMathDivide },
            ]},
            OperationListItem::Category { name: "cast".to_string(), operation_list_items: vec![
                OperationListItem::Operation { operation: Operation::NumberCastToInteger },
                OperationListItem::Operation { operation: Operation::NumberCastToDecimal },
            ]},
        ]},
        OperationListItem::Category { name: "colors".to_string(), operation_list_items: vec![
            OperationListItem::Category { name: "input".to_string(), operation_list_items: vec![
                OperationListItem::Operation { operation: Operation::ColorInputRgba },
            ]},
        ]},
        OperationListItem::Category { name: "images".to_string(), operation_list_items: vec![
            OperationListItem::Category { name: "input".to_string(), operation_list_items: vec![
                OperationListItem::Operation { operation: Operation::ImageInputFile },
                OperationListItem::Operation { operation: Operation::ImageInputUrl },
                OperationListItem::Operation { operation: Operation::ImageInputClipboard },
                OperationListItem::Operation { operation: Operation::ImageInputColor },
            ]},
            OperationListItem::Category { name: "output".to_string(), operation_list_items: vec![
                OperationListItem::Operation { operation: Operation::ImageOutputFile },
                OperationListItem::Operation { operation: Operation::ImageOutputClipboard },
            ]},
            OperationListItem::Category { name: "transform".to_string(), operation_list_items: vec![
                OperationListItem::Operation { operation: Operation::ImageTransformResize },
                OperationListItem::Operation { operation: Operation::ImageTransformResizeExact },
                OperationListItem::Operation { operation: Operation::ImageTransformResizeFill },
            ]},
            OperationListItem::Category { name: "adjustments".to_string(), operation_list_items: vec![
                OperationListItem::Operation { operation: Operation::ImageAdjustmentBlur },
                OperationListItem::Operation { operation: Operation::ImageAdjustmentContrast },
                OperationListItem::Operation { operation: Operation::IMageAdjustmentGrayscale }
            ]},
        ]}, 
        //OperationListItem::Subgraph,
    ]
}
