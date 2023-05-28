use crate::input::Input;
use crate::node_settings::NodeSettings;
use crate::operation::{OperationError, OperationResponse, ConnectionSettings, UiType, OutputResponse};
use crate::value::{Value, ValueType};
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


pub async fn new_float(inputs: &[Input]) -> Result<OperationResponse, OperationError> {
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

        _ => {return Err(OperationError{message:"Unable to convert to float.".to_string()});}
    };

    Ok(OperationResponse {
        time: Instant::now().duration_since(start_time),
        outputs: vec![OutputResponse {
            value,
        }],
    })
}