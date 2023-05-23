use crate::input::Input;
use crate::nodes::node_settings::NodeSettings;
use crate::nodes::operation::{OperationError, OperationResponse, ConnectionSettings, UiType};
use crate::value::{Value, ValueType};
use core::panic;
use std::time::Instant;

lazy_static! {
    pub static ref SETTINGS: NodeSettings = NodeSettings::new("Decimal".to_string());
    pub static ref INPUT_SETTINGS: Vec<ConnectionSettings> = vec![ConnectionSettings {
        name: "decimal".to_string(),
        default_value: Value::Decimal(0.0),
        valid_types: vec![ValueType::Decimal, ValueType::Integer, ValueType::String],
        ui_type: Some(UiType::DragValue),
    },];
    pub static ref OUTPUT_SETTINGS: Vec<ConnectionSettings> = vec![ConnectionSettings {
        name: "decimal".to_string(),
        default_value: Value::Decimal(0.0),
        valid_types: vec![ValueType::Decimal],
        ui_type: None,
    },];
}


pub async fn new_float(inputs: &[Input]) -> Result<Vec<OperationResponse>, OperationError> {
    let start_time = Instant::now();

    let value = match &inputs[0].get_value() {
        Value::Integer(a) => Value::Decimal(*a as f32),
        Value::Decimal(a) => Value::Decimal(*a),
        Value::String(a) => {
            if let Ok(n) = a.parse::<f32>() {
                Value::Decimal(n)
            } else {
                OUTPUT_SETTINGS[0].default_value.clone()
            }
        },

        _ => panic!("Unable to convert formats to float."),
    };

    let node_output_message = OperationResponse {
        index: 0,
        value,
        time: Instant::now().duration_since(start_time),
    };

    Ok(vec![node_output_message])
}