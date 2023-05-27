use std::time::Duration;

use image::{ImageBuffer, Rgba};
use nanoid::nanoid;
use node_settings::NodeSettings;
use operation::Operation;
use value::{Value, ValueType};
use crate::operation::ConnectionSettings;

#[macro_use]
extern crate lazy_static;

pub mod input;
pub mod output;
pub mod value;
pub mod graph;
pub mod node_settings;
pub mod operation;
pub mod node;
pub mod operations;


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
    pub value_type: ValueType,
    pub time: Duration, 
    pub thumbnail: Option< ImageBuffer<Rgba<u8>, Vec<u8>>>,
}

#[derive(Debug)]
pub struct AddNodeMessage {
    pub node_id: String,
    pub node_settings: NodeSettings,
    pub input_settings: Vec<ConnectionSettings>,
    pub output_settings: Vec<ConnectionSettings>,
    pub operation: Operation,
    pub position: [f32; 2],
}

#[derive(Debug)]
pub struct AddedNodeMessage {
    pub node_id: String,
    pub node_settings: NodeSettings,
    pub input_settings: Vec<ConnectionSettings>,
    pub output_settings: Vec<ConnectionSettings>,
    pub position: [f32; 2],
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


pub struct OperationCategory {
    pub name: String,
    pub operations: Vec<OperationDescription>,
}


pub struct OperationDescription {
    pub node_settings: NodeSettings,
    pub input_settings: Vec<ConnectionSettings>,
    pub output_settings: Vec<ConnectionSettings>,
    pub operation: Operation,
}


lazy_static! {
    pub static ref OPERATION_LIST: Vec<OperationCategory> = vec![
        OperationCategory {
            name: "Numbers".to_string(),
            operations: vec![
                OperationDescription {
                    node_settings: operations::float::SETTINGS.clone(),
                    input_settings: operations::float::INPUT_SETTINGS.clone(),
                    output_settings: operations::float::OUTPUT_SETTINGS.clone(),
                    operation: Operation::Float,
                },
                OperationDescription {
                    node_settings: operations::integer::SETTINGS.clone(),
                    input_settings: operations::integer::INPUT_SETTINGS.clone(),
                    output_settings: operations::integer::OUTPUT_SETTINGS.clone(),
                    operation: Operation::Integer,
                },
                OperationDescription {
                    node_settings: operations::add::SETTINGS.clone(),
                    input_settings: operations::add::INPUT_SETTINGS.clone(),
                    output_settings: operations::add::OUTPUT_SETTINGS.clone(),
                    operation: Operation::Add,
                },
                OperationDescription {
                    node_settings: operations::subtract::SETTINGS.clone(),
                    input_settings: operations::subtract::INPUT_SETTINGS.clone(),
                    output_settings: operations::subtract::OUTPUT_SETTINGS.clone(),
                    operation: Operation::Subtract,
                },
            ]
        },
        OperationCategory {
            name: "Images".to_string(),
            operations: vec![
                OperationDescription {
                    node_settings: operations::image_from_url::SETTINGS.clone(),
                    input_settings: operations::image_from_url::INPUT_SETTINGS.clone(),
                    output_settings: operations::image_from_url::OUTPUT_SETTINGS.clone(),
                    operation: Operation::ImageFromUrl,
                },
                OperationDescription {
                    node_settings: operations::image_resize::SETTINGS.clone(),
                    input_settings: operations::image_resize::INPUT_SETTINGS.clone(),
                    output_settings: operations::image_resize::OUTPUT_SETTINGS.clone(),
                    operation: Operation::ImageResize,
                },
                OperationDescription {
                    node_settings: operations::image_from_clipboard::SETTINGS.clone(),
                    input_settings: operations::image_from_clipboard::INPUT_SETTINGS.clone(),
                    output_settings: operations::image_from_clipboard::OUTPUT_SETTINGS.clone(),
                    operation: Operation::ImageFromClipboard,
                },
            ]
        },
        OperationCategory {
            name: "Text".to_string(),
            operations: vec![
                OperationDescription {
                    node_settings: operations::text_from_clipboard::SETTINGS.clone(),
                    input_settings: operations::text_from_clipboard::INPUT_SETTINGS.clone(),
                    output_settings: operations::text_from_clipboard::OUTPUT_SETTINGS.clone(),
                    operation: Operation::TextFromClipboard,
                },
            ]
        }
    ];
}