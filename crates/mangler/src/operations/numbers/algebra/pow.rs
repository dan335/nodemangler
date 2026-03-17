use crate::input::{Input, InputSettings};
use crate::node_settings::NodeSettings;
use crate::operations::{OperationResponse, OperationError, OutputResponse, convert_input};
use crate::output::Output;
use crate::value::{Value, ValueType};
use serde::{Deserialize, Serialize};
use std::time::Instant;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpNumberMathPow {}

impl OpNumberMathPow {
    pub fn settings() -> NodeSettings {
        NodeSettings {
            name: "power".to_string(),
            description: "Raises base to an exponent.".to_string(),
        }
    }

    pub fn create_inputs() -> Vec<Input> {
        vec![
            Input::new("base".to_string(), Value::Decimal(2.0), Some(InputSettings::DragValue { speed: None, clamp: None }), None),
            Input::new("exponent".to_string(), Value::Decimal(2.0), Some(InputSettings::DragValue { speed: None, clamp: None }), None),
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

        let base_converted = convert_input(inputs, 0, ValueType::Decimal, &mut input_errors);
        let exponent_converted = convert_input(inputs, 1, ValueType::Decimal, &mut input_errors);

        if input_errors.len() > 0 { return Err(OperationError { input_errors, node_error: None }); }

        let Value::Decimal(base) = base_converted.unwrap() else { unreachable!() };
        let Value::Decimal(exponent) = exponent_converted.unwrap() else { unreachable!() };

        let result = (base as f64).powf(exponent as f64) as f32;

        Ok(OperationResponse {
            time: Instant::now().duration_since(start_time),
            responses: vec![OutputResponse {
                value: Value::Decimal(result),
            }],
        })
    }
}
