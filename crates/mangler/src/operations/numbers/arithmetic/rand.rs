use crate::input::{Input, InputSettings};
use crate::node_settings::NodeSettings;
use crate::operations::{OperationResponse, OperationError, OutputResponse, convert_input};
use crate::output::Output;
use crate::value::{Value, ValueType};
use serde::{Deserialize, Serialize};
use std::time::Instant;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpNumberMathRand {}

impl OpNumberMathRand {
    pub fn settings() -> NodeSettings {
        NodeSettings {
            name: "random".to_string(),
            description: "Generates a random decimal between min and max.".to_string(),
        }
    }

    pub fn create_inputs() -> Vec<Input> {
        vec![
            Input::new("min".to_string(), Value::Decimal(0.0), Some(InputSettings::DragValue { speed: None, clamp: None }), None),
            Input::new("max".to_string(), Value::Decimal(1.0), Some(InputSettings::DragValue { speed: None, clamp: None }), None),
        ]
    }

    pub fn create_outputs() -> Vec<Output> {
        vec![
            Output::new("output".to_string(), Value::Decimal(0.0), None)
        ]
    }

    pub async fn run(inputs: &mut Vec<Input>) -> Result<OperationResponse, OperationError> {
        let start_time = Instant::now();
        let mut input_errors: Vec<(usize, String)> = vec![];

        let min_converted = convert_input(inputs, 0, ValueType::Decimal, &mut input_errors);
        let max_converted = convert_input(inputs, 1, ValueType::Decimal, &mut input_errors);

        if input_errors.len() > 0 { return Err(OperationError { input_errors, node_error: None }); }

        let Value::Decimal(min) = min_converted.unwrap() else { unreachable!() };
        let Value::Decimal(max) = max_converted.unwrap() else { unreachable!() };

        if min >= max {
            return Err(OperationError {
                input_errors: vec![(0, "Min must be less than max.".to_string())], node_error: None,
            });
        }

        let value = min + fastrand::f32() * (max - min);

        Ok(OperationResponse {
            time: Instant::now().duration_since(start_time),
            responses: vec![OutputResponse {
                value: Value::Decimal(value),
            }],
        })
    }
}
