use std::time::{Instant, Duration};
use crate::input::Input;
use crate::nodes::operation::ConnectionSettings;
use crate::output::Output;
use crate::value::{Value, ValueType};
use crate::nodes::node_settings::NodeSettings;

use super::operation::Operation;

lazy_static! {
    pub static ref SETTINGS: NodeSettings = NodeSettings::new("Add".to_string());

    pub static ref INPUT_SETTINGS: Vec<ConnectionSettings> = vec![
        ConnectionSettings {
            name: "a".to_string(),
            default_value: Value::Decimal { value: 0.0 },
            valid_types: vec![ValueType::Decimal, ValueType::Integer, ValueType::String],
        },
        ConnectionSettings {
            name: "b".to_string(),
            default_value: Value::Decimal { value: 0.0 },
            valid_types: vec![ValueType::Decimal, ValueType::Integer, ValueType::String],
        },
    ];

    pub static ref OUTPUT_SETTINGS: Vec<ConnectionSettings> = vec![
        ConnectionSettings {
            name: "result".to_string(),
            default_value: Value::Decimal { value: 0.0 },
            valid_types: vec![ValueType::Decimal],
        },
    ];
}


#[derive(Debug, Clone, Default)]
pub struct Add {}


impl Operation for Add {
    fn run(&mut self, inputs: &Vec<Input>, outputs: &mut Vec<Output>) -> Duration {
        let start_time = Instant::now();

        outputs[0].value = match (&inputs[0].value, &inputs[1].value) {
            (
                Value::Integer { value: a },
                Value::Decimal { value: b }
            ) => {
                Value::Decimal { value: *a as f32 + *b }
            },

            (
                Value::Integer { value: a },
                Value::Integer { value: b }
            ) => {
                Value::Integer { value: *a + *b }
            },

            (
                Value::Integer { value: a },
                Value::String { value: b }
            ) => {
                Value::String { value: format!("{} {}", a.to_string(), *b) }
            },

            (
                Value::Decimal { value: a },
                Value::Decimal { value: b }
            ) => {
                Value::Decimal { value: *a + *b }
            },

            (
                Value::Decimal { value: a },
                Value::Integer { value: b }
            ) => {
                Value::Decimal { value: *a + *b as f32 }
            },

            (
                Value::Decimal { value: a },
                Value::String { value: b }
            ) => {
                Value::String { value: format!("{} {}", a.to_string(), *b) }
            },

            (
                Value::String { value: a },
                Value::Integer { value: b }
            ) => {
                Value::String { value: format!("{} {}", *a, b.to_string()) }
            },

            (
                Value::String { value: a },
                Value::Decimal { value: b }
            ) => {
                Value::String { value: format!("{} {}", *a, b.to_string()) }
            },

            (
                Value::String { value: a },
                Value::String { value: b }
            ) => {
                Value::String { value: format!("{} {}", *a, *b) }
            },
        };


        Instant::now().duration_since(start_time)
    }
}