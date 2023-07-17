use std::path::PathBuf;
use crate::{value::Value, get_id, Output};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Input {
    pub id: String,
    pub name: String,
    pub value: Value,
    pub default_value: Value,
    pub settings: Option<InputSettings>,
    pub connection: Option<(String, usize)>, // id of node with output, index of output
    pub is_error: bool,
    pub error_message: Option<String>,
    pub is_exposed: bool,
    // todo: need to link this somehow with exposed input
    // maybe it needs an id?
    // input that this input should pass data to
    // used to pass node's input to subgraph input
    #[serde(skip)]
    pub link: Option<InputLink>,
}

impl PartialEq for Input {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id
    }
}

impl Input {
    pub fn new(name: String, default_value: Value, settings: Option<InputSettings>, link: Option<InputLink>) -> Input {
        Input {
            id: get_id(),
            name,
            value: default_value.clone(),
            default_value,
            settings,
            connection: None,
            is_error: false,
            error_message: None,
            is_exposed: false,
            link,
        }
    }

    pub fn is_valid_connection(&self, output: &Output) -> bool {
        self.value.value_type().valid_conversions().contains(&output.value.value_type())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InputLink {
    pub node_id: String,
    pub input_id: String,
}


// additional settings for intput
// that do not belong in the value
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum InputSettings {
    Path {
        extension_filter: Vec<String>,
        set_directory: Option<PathBuf>,
        set_file_name: Option<String>,
        set_title: Option<String>,
        file_dialog_type: FileDialogType,
    },
    DragValue {
        clamp: Option<(f32, f32)>,
        speed: Option<f32>,
    },
    Slider {
        range: (f32, f32),
        step_by: Option<f32>,
        clamp_to_range: bool,
    },
    SingleLineText,
    MultiLineText,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum FileDialogType {
    PickFile,
    PickFolder,
    SaveFile,
}