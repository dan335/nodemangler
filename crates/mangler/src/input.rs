use std::path::PathBuf;
use crate::{value::Value, get_id, Output};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Input {
    pub id: String,
    pub name: String,
    pub value: Value,
    pub settings: InputSettings,
    pub connection: Option<(String, usize)>, // id of node with output, index of output
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
    pub fn new(name: String, value: Value, settings: InputSettings, link: Option<InputLink>) -> Input {
        Input {
            id: get_id(),
            name,
            value,
            settings,
            connection: None,
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
    None,
    Path {
        extension_filter: Vec<String>,
        set_directory: Option<PathBuf>,
        set_file_name: Option<String>,
        set_title: Option<String>,
        file_dialog_type: FileDialogType,
    },
    String(TextInputType),
    Decimal(DecimalInputType),
    Integer(IntegerInputType),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum IntegerInputType {
    DragValue {
        clamp: Option<(i32, i32)>,
    },
    Slider {
        range: (i32, i32),
        step_by: Option<i32>,
        clamp_to_range: bool,
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum DecimalInputType {
    DragValue {
        speed: Option<f32>,
        clamp: Option<(f32, f32)>,
    },
    Slider {
        range: (f32, f32),
        step_by: Option<f32>,
        clamp_to_range: bool,
    }
}


#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TextInputType {
    SingleLine,
    MultiLine,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum FileDialogType {
    PickFile,
    PickFolder,
    SaveFile,
}