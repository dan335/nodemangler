use glam::f32::Vec2;
use input::Input;
use nanoid::nanoid;
use node::Node;
use node_settings::NodeSettings;
use operations::Operation;
use output::Output;
use serde::{Deserialize, Serialize};
use thumbnail::Thumbnail;
use std::{collections::HashMap, path::PathBuf, time::Duration};
use value::Value;

pub mod graph;
pub mod input;
pub mod node;
pub mod node_settings;
pub mod operations;
pub mod output;
pub mod value;
pub mod node_type;
pub mod thumbnail;
pub mod app;
pub mod dynamic_image_serde;
pub mod color;
mod tests;

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
    InputErrorChanged {
        node_id: String,
        input_index: usize,
        is_error: bool,
        message: Option<String>,
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
    Error {
        node_id: String,
        is_error: bool,
        message: Option<String>,
    },
    InfoChanged {
        node_id: String,
        time: Duration,
    },
    GraphRunCompleted {
        total_time: Duration,
    },
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



#[derive(Debug)]
pub struct NewGraphError(pub String);

#[derive(Serialize, Deserialize, Debug)]
pub struct GraphSaveData {
    pub id: String,
    pub name: String,
    pub nodes: HashMap<String, Node>,
}


