use std::time::Duration;

use image::{ImageBuffer, Rgba};
use nanoid::nanoid;
use nodes::{node_settings::NodeSettings, operation::ConnectionSettings, operation::Operation};
use value::{Value, ValueType};

#[macro_use]
extern crate lazy_static;

pub mod nodes;
pub mod input;
pub mod output;
pub mod value;
pub mod graph;
pub mod node_attributes;


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
}

#[derive(Debug)]
pub struct RemoveNodeMessage {
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
pub struct RemoveConnectionMessage {
    pub node_id: String,
    pub input_index: usize,
}