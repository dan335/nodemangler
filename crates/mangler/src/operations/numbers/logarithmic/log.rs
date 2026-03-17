use crate::input::{Input, InputSettings};
use crate::node_settings::NodeSettings;
use crate::operations::{OperationResponse, OperationError, OutputResponse, convert_input};
use crate::output::Output;
use crate::value::{Value, ValueType};
use serde::{Deserialize, Serialize};
use std::time::Instant;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpNumberMathLog {}

impl OpNumberMathLog {
    pub fn settings() -> NodeSettings {
        NodeSettings {
            name: "log".to_string(),
            description: "Computes logarithm with a given base.".to_string(),
        }
    }

    pub fn create_inputs() -> Vec<Input> {
        vec![
            Input::new("input".to_string(), Value::Decimal(100.0), Some(InputSettings::DragValue { speed: None, clamp: None }), None),
            Input::new("base".to_string(), Value::Decimal(10.0), Some(InputSettings::DragValue { speed: None, clamp: None }), None),
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

        let input_converted = convert_input(inputs, 0, ValueType::Decimal, &mut input_errors);
        let base_converted = convert_input(inputs, 1, ValueType::Decimal, &mut input_errors);

        if input_errors.len() > 0 { return Err(OperationError { input_errors, node_error: None }); }

        let Value::Decimal(input) = input_converted.unwrap() else { unreachable!() };
        let Value::Decimal(base) = base_converted.unwrap() else { unreachable!() };

        if input <= 0.0 {
            return Err(OperationError { input_errors: vec![], node_error: Some("Input must be greater than 0.".to_string()) });
        }
        if base <= 0.0 || base == 1.0 {
            return Err(OperationError { input_errors: vec![], node_error: Some("Base must be greater than 0 and not equal to 1.".to_string()) });
        }

        let result = (input as f64).log(base as f64) as f32;

        Ok(OperationResponse {
            time: Instant::now().duration_since(start_time),
            responses: vec![OutputResponse {
                value: Value::Decimal(result),
            }],
        })
    }
}
