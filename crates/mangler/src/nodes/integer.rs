use crate::input::Input;
use crate::nodes::node_settings::NodeSettings;
use crate::nodes::operation::{ConnectionSettings, UiType};
use crate::output::Output;
use crate::value::{Value, ValueType};
use std::time::{Duration, Instant};

use super::operation::OperationResponse;

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

pub fn new_integer(inputs: &[Input], outputs: &mut [Output]) -> OperationResponse {
    let start_time = Instant::now();

    let mut response = OperationResponse::new();

    response.output_values.push(match &inputs[0].get_value() {
        Value::Integer(a) => Value::Integer(*a),
        Value::Decimal(a) => Value::Integer(*a as i32),
        Value::String(a) => {
            if let Ok(n) = a.parse::<i32>() {
                Value::Integer(n)
            } else {
                OUTPUT_SETTINGS[0].default_value.clone()
            }
        },

        _ => panic!("Unable to convert formats to integer."),
    });

    response.time = Instant::now().duration_since(start_time);
    response
}