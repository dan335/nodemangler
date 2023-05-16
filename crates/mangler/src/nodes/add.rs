use crate::input::Input;
use crate::nodes::node_settings::NodeSettings;
use crate::nodes::operation::{ConnectionSettings, UiType};
use crate::output::Output;
use crate::value::{Value, ValueType};
use core::panic;
use std::time::{Duration, Instant};

lazy_static! {
    pub static ref SETTINGS: NodeSettings = NodeSettings::new("Add".to_string());
    pub static ref INPUT_SETTINGS: Vec<ConnectionSettings> = vec![
        ConnectionSettings {
            name: "a".to_string(),
            default_value: Value::Decimal(0.0),
            valid_types: vec![ValueType::Decimal, ValueType::Integer, ValueType::String],
            ui_type: Some(UiType::DragValue),
        },
        ConnectionSettings {
            name: "b".to_string(),
            default_value: Value::Decimal(0.0),
            valid_types: vec![ValueType::Decimal, ValueType::Integer, ValueType::String],
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

#[derive(Debug, Clone, Default)]
pub struct Add {}

impl Add {
    pub fn run(&mut self, inputs: &[Input], outputs: &mut [Output]) -> Duration {
        let start_time = Instant::now();

        outputs[0].value = match (&inputs[0].value, &inputs[1].value) {
            (Value::Integer(a), Value::Decimal(b)) => Value::Decimal(*a as f32 + *b),

            (Value::Integer(a), Value::Integer(b)) => Value::Integer(*a + *b),

            (Value::Integer(a), Value::String(b)) => Value::String(format!("{} {}", a, *b)),

            (Value::Decimal(a), Value::Decimal(b)) => Value::Decimal(*a + *b),

            (Value::Decimal(a), Value::Integer(b)) => Value::Decimal(*a + *b as f32),

            (Value::Decimal(a), Value::String(b)) => Value::String(format!("{} {}", a, *b)),

            (Value::String(a), Value::Integer(b)) => Value::String(format!("{} {}", *a, b)),

            (Value::String(a), Value::Decimal(b)) => Value::String(format!("{} {}", *a, b)),

            (Value::String(a), Value::String(b)) => Value::String(format!("{} {}", *a, *b)),

            _ => panic!("Unable to add formats."),
        };

        Instant::now().duration_since(start_time)
    }
}
