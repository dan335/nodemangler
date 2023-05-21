use crate::input::Input;
use crate::nodes::node_settings::NodeSettings;
use crate::nodes::operation::{ConnectionSettings, UiType};
use crate::output::Output;
use crate::value::{Value, ValueType};
use std::time::{Duration, Instant};

use super::operation::OperationResponse;

lazy_static! {
    pub static ref SETTINGS: NodeSettings = NodeSettings::new("Subtract".to_string());
    pub static ref INPUT_SETTINGS: Vec<ConnectionSettings> = vec![
        ConnectionSettings {
            name: "a".to_string(),
            default_value: Value::Decimal(0.0),
            valid_types: vec![ValueType::Decimal, ValueType::Integer],
            ui_type: Some(UiType::DragValue),
        },
        ConnectionSettings {
            name: "b".to_string(),
            default_value: Value::Decimal(0.0),
            valid_types: vec![ValueType::Decimal, ValueType::Integer],
            ui_type: Some(UiType::DragValue),
        },
    ];
    pub static ref OUTPUT_SETTINGS: Vec<ConnectionSettings> = vec![ConnectionSettings {
        name: "result".to_string(),
        default_value: Value::Decimal(0.0),
        valid_types: vec![ValueType::Decimal],
        ui_type: None,
    },];
}

pub fn subtract(inputs: &[Input], outputs: &mut [Output]) -> Duration {
    let start_time = Instant::now();

    outputs[0].value = match (&inputs[0].get_value(), &inputs[1].get_value()) {
        (Value::Integer(a), Value::Decimal(b)) => Value::Decimal(*a as f32 - b),

        (Value::Integer(a), Value::Integer(b)) => Value::Integer(a - b),

        (Value::Decimal(a), Value::Decimal(b)) => Value::Decimal(a - b),

        (Value::Decimal(a), Value::Integer(b)) => Value::Decimal(a - *b as f32),

        _ => panic!(),
    };

    Instant::now().duration_since(start_time)
}