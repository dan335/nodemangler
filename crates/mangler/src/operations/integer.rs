use crate::input::Input;
use crate::node_settings::NodeSettings;
use crate::operation::{OperationError, OperationResponse, ConnectionSettings, UiType, OutputResponse};
use crate::output::Output;
use crate::value::{Value, ValueType};
use std::time::{Instant, Duration};
use serde::{Serialize, Deserialize};

lazy_static! {
    pub static ref SETTINGS: NodeSettings = NodeSettings::new("Integer".to_string());
    pub static ref INPUT_SETTINGS: Vec<ConnectionSettings> = vec![ConnectionSettings {
        name: "integer".to_string(),
        default_value: Value::Integer(0),
        valid_types: vec![ValueType::Decimal, ValueType::Integer, ValueType::String],
        ui_type: Some(UiType::DragValue),
    },];
    pub static ref OUTPUT_SETTINGS: Vec<ConnectionSettings> = vec![ConnectionSettings {
        name: "integer".to_string(),
        default_value: Value::Integer(0),
        valid_types: vec![ValueType::Integer],
        ui_type: None,
    },];
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OperationInputInteger {}


impl OperationInputInteger {
    pub fn settings() -> NodeSettings {
        NodeSettings { name: "Integer".to_string() }
    }

    pub fn create_inputs() -> Vec<Input> {
        vec![]
    }

    pub fn create_outputs() -> Vec<Output> {
        vec![]
    }

    pub async fn run(inputs: &[Input]) -> Result<OperationResponse, OperationError> {
        let start_time = Instant::now();

    let value = match &inputs[0].get_value() {
        Value::Integer(a) => Value::Integer(*a),
        Value::Decimal(a) => Value::Integer(*a as i32),
        Value::String(a) => {
            if let Ok(n) = a.parse::<i32>() {
                Value::Integer(n)
            } else {
                OUTPUT_SETTINGS[0].default_value.clone()
            }
        },

        _ => { return Err(OperationError{message:"Not supported".to_string()}); },
    };

    let node_output_message = OperationResponse {
        time: Instant::now().duration_since(start_time),
        outputs: vec![OutputResponse {
            value,
        }],
    };

    Ok(node_output_message) 
    }
}


pub async fn new_integer(inputs: &[Input]) -> Result<OperationResponse, OperationError> {
    let start_time = Instant::now();

    let value = match &inputs[0].get_value() {
        Value::Integer(a) => Value::Integer(*a),
        Value::Decimal(a) => Value::Integer(*a as i32),
        Value::String(a) => {
            if let Ok(n) = a.parse::<i32>() {
                Value::Integer(n)
            } else {
                OUTPUT_SETTINGS[0].default_value.clone()
            }
        },

        _ => { return Err(OperationError{message:"Not supported".to_string()}); },
    };

    let node_output_message = OperationResponse {
        time: Instant::now().duration_since(start_time),
        outputs: vec![OutputResponse {
            value,
        }],
    };

    Ok(node_output_message) 
}