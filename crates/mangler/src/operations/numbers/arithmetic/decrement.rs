use crate::input::{Input, InputSettings};
use crate::node_settings::NodeSettings;
use crate::operations::{OperationResponse, OperationError, OutputResponse};
use crate::output::Output;
use crate::value::Value;
use serde::{Deserialize, Serialize};
use std::time::Instant;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpNumberMathDecrement {}

impl OpNumberMathDecrement {
    pub fn settings() -> NodeSettings {
        NodeSettings {
            name: "decrement".to_string(),
            description: "Decrements a number by 1.".to_string(),
        }
    }

    pub fn create_inputs() -> Vec<Input> {
        vec![
            Input::new("a".to_string(), Value::Decimal(0.0), Some(InputSettings::DragValue { speed:None, clamp:None }), None),
        ]
    }

    pub fn create_outputs() -> Vec<Output> {
        vec![
            Output::new("output".to_string(), Value::Decimal(f32::default()), None)
        ]
    }

    pub async fn run(inputs: &Vec<Input>) -> Result<OperationResponse, OperationError> {
        let start_time = Instant::now();

        let value = match &inputs[0].value {
            Value::Integer(a) => Value::Integer(*a - 1),

            Value::Decimal(a) => Value::Decimal(*a - 1.0),

            Value::String(a) => Value::String(format!("{} {}", *a, -1)),

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
