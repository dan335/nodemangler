use crate::input::{Input, InputSettings};
use crate::node_settings::NodeSettings;
use crate::operations::{OperationResponse, OperationError, OutputResponse, convert_input};
use crate::output::Output;
use crate::value::{Value, ValueType};
use serde::{Deserialize, Serialize};
use std::time::Instant;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpNumberMathFactorial {}

impl OpNumberMathFactorial {
    pub fn settings() -> NodeSettings {
        NodeSettings {
            name: "factorial".to_string(),
            description: "Computes the factorial of an integer.".to_string(),
        }
    }

    pub fn create_inputs() -> Vec<Input> {
        vec![
            Input::new("input".to_string(), Value::Integer(5), Some(InputSettings::DragValue { speed: None, clamp: Some((0.0, 12.0)) }), None),
        ]
    }

    pub fn create_outputs() -> Vec<Output> {
        vec![
            Output::new("output".to_string(), Value::Integer(0), None)
        ]
    }

    pub async fn run(inputs: &mut Vec<Input>) -> Result<OperationResponse, OperationError> {
        let start_time = Instant::now();
        let mut input_errors: Vec<(usize, String)> = vec![];

        let input_converted = convert_input(inputs, 0, ValueType::Integer, &mut input_errors);

        if input_errors.len() > 0 { return Err(OperationError { input_errors, node_error: None }); }

        let Value::Integer(val) = input_converted.unwrap() else { unreachable!() };

        let val = val.clamp(0, 12); // 12! = 479001600, max that fits in i32

        let mut result: i32 = 1;
        for i in 2..=(val) {
            result *= i;
        }

        Ok(OperationResponse {
            time: Instant::now().duration_since(start_time),
            responses: vec![OutputResponse {
                value: Value::Integer(result),
            }],
        })
    }
}
