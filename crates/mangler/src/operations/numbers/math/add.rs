use crate::input::Input;
use crate::node_settings::NodeSettings;
use crate::operation::{OperationError, OperationResponse, OutputResponse};
use crate::output::Output;
use crate::value::Value;
use serde::{Deserialize, Serialize};
use std::time::Instant;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OperationNumberMathAdd {}

impl OperationNumberMathAdd {
    pub fn settings() -> NodeSettings {
        NodeSettings {
            name: "Add".to_string(),
        }
    }

    pub fn create_inputs() -> Vec<Input> {
        vec![
            Input {
                name: "a".to_string(),
                value: Value::Integer(i32::default()),
                connection: None,
                valid_types: vec![],
            },
            Input {
                name: "b".to_string(),
                value: Value::Integer(i32::default()),
                connection: None,
                valid_types: vec![],
            },
        ]
    }

    pub fn create_outputs() -> Vec<Output> {
        vec![Output {
            name: "Result".to_string(),
            value: Value::Integer(i32::default()),
            connection: None,
        }]
    }

    pub async fn run(inputs: &[Input]) -> Result<OperationResponse, OperationError> {
        let start_time = Instant::now();

        let value = match (&inputs[0].value, &inputs[1].value) {
            (Value::Integer(a), Value::Decimal(b)) => Value::Decimal(*a as f32 + *b),

            (Value::Integer(a), Value::Integer(b)) => Value::Integer(*a + *b),

            (Value::Integer(a), Value::String(b)) => Value::String(format!("{} {}", a, *b)),

            (Value::Decimal(a), Value::Decimal(b)) => Value::Decimal(*a + *b),

            (Value::Decimal(a), Value::Integer(b)) => Value::Decimal(*a + *b as f32),

            (Value::Decimal(a), Value::String(b)) => Value::String(format!("{} {}", a, *b)),

            (Value::String(a), Value::Integer(b)) => Value::String(format!("{} {}", *a, b)),

            (Value::String(a), Value::Decimal(b)) => Value::String(format!("{} {}", *a, b)),

            (Value::String(a), Value::String(b)) => Value::String(format!("{} {}", *a, *b)),

            _ => {return Err(OperationError {
                message: "Error converting. {:?}".to_string(),
            });}
        };

        Ok(OperationResponse {
            time: Instant::now().duration_since(start_time),
            responses: vec![OutputResponse {
                value: value,
            }],
        })
    }
}
