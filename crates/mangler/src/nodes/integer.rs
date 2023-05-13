use std::time::{Instant, Duration};
use crate::input::Input;
use crate::nodes::operation::{ConnectionSettings, UiType};
use crate::output::Output;
use crate::value::{Value, ValueType};
use crate::nodes::node_settings::NodeSettings;

use super::operation::Operation;

lazy_static! {
    pub static ref SETTINGS: NodeSettings = NodeSettings::new("Integer".to_string());

    pub static ref INPUT_SETTINGS: Vec<ConnectionSettings> = vec![
        ConnectionSettings {
            name: "integer".to_string(),
            default_value: Value::Integer(0),
            valid_types: vec![ValueType::Decimal, ValueType::Integer, ValueType::String],
            ui_type: Some(UiType::DragValue),
        },
    ];

    pub static ref OUTPUT_SETTINGS: Vec<ConnectionSettings> = vec![
        ConnectionSettings {
            name: "integer".to_string(),
            default_value: Value::Integer(0),
            valid_types: vec![ValueType::Integer],
            ui_type: None,
        },
    ];
}


#[derive(Debug, Clone, Default)]
pub struct Integer {}


impl Operation for Integer {
    fn run(&mut self, inputs: &Vec<Input>, outputs: &mut Vec<Output>) -> Duration {
        let start_time = Instant::now();

        outputs[0].value = match &inputs[0].value {
            Value::Integer(a) => Value::Integer(*a),
            Value::Decimal(a) => Value::Integer(*a as usize),
            Value::String(a) => {
                if let Ok(n) = a.parse::<usize>() {
                    Value::Integer(n)
                } else {
                    OUTPUT_SETTINGS[0].default_value.clone()
                }
            },
        };

        Instant::now().duration_since(start_time)
    }
}