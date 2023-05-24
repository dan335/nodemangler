use crate::input::Input;
use crate::node_settings::NodeSettings;
use crate::operation::{OperationError, OperationResponse, ConnectionSettings, UiType};
use crate::value::{Value, ValueType};
use std::time::Instant;

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

pub async fn new_integer(inputs: &[Input]) -> Result<Vec<OperationResponse>, OperationError> {
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
        index: 0,
        value,
        time: Instant::now().duration_since(start_time),
    };

    Ok(vec![node_output_message]) 
}