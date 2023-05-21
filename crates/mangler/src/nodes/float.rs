use tokio::sync::mpsc::Sender;

use crate::NodeOutputChangedMessage;
use crate::input::Input;
use crate::nodes::node_settings::NodeSettings;
use crate::nodes::operation::{ConnectionSettings, UiType};
use crate::output::Output;
use crate::value::{Value, ValueType};
use std::time::{Duration, Instant};

use super::operation::OperationResponse;

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


pub async fn new_float(node_id: &String, inputs: &[Input], outputs: &mut [Output], tx_output: Sender<NodeOutputChangedMessage>) -> Duration {
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

    let time = Instant::now().duration_since(start_time);

    let node_output_message = NodeOutputChangedMessage {
        node_id: node_id.clone(),
        output_index: 0,
        value: value.clone(),
        time,
    };

    match tx_output.try_send(node_output_message) {
        Ok(_) => {
            outputs[0].value = value;
        },
        Err(err) => {
            println!("Error: {:?}", err);
        },
    }

    time  
}