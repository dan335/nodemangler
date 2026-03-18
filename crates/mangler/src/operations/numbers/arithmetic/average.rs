//! Average operation for the node graph.
//!
//! Computes the arithmetic mean of two numbers. Both inputs are converted to
//! decimal before the computation.

use crate::input::{Input, InputSettings};
use crate::node_settings::NodeSettings;
use crate::operations::{OperationResponse, OperationError, OutputResponse, convert_input};
use crate::output::Output;
use crate::value::{Value, ValueType};
use serde::{Deserialize, Serialize};
use std::time::Instant;

/// Node operation that computes the average (mean) of two numbers.
///
/// Both inputs are converted to decimal. The result is `(a + b) / 2.0`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpNumberMathAverage {}

impl OpNumberMathAverage {
    /// Returns the node metadata (name and description).
    pub fn settings() -> NodeSettings {
        NodeSettings {
            name: "average".to_string(),
            description: "Computes the average (mean) of two numbers.".to_string(),
        }
    }

    /// Creates the default input list: two decimal drag-value inputs defaulting to 0.0.
    pub fn create_inputs() -> Vec<Input> {
        vec![
            Input::new("a".to_string(), Value::Decimal(0.0), Some(InputSettings::DragValue { speed: None, clamp: None }), None),
            Input::new("b".to_string(), Value::Decimal(0.0), Some(InputSettings::DragValue { speed: None, clamp: None }), None),
        ]
    }

    /// Creates the default output list: a single decimal output.
    pub fn create_outputs() -> Vec<Output> {
        vec![
            Output::new("output".to_string(), Value::Decimal(f32::default()), None)
        ]
    }

    /// Executes the average operation: computes `(a + b) / 2.0`.
    pub async fn run(inputs: &mut Vec<Input>) -> Result<OperationResponse, OperationError> {
        let start_time = Instant::now();
        let mut input_errors: Vec<(usize, String)> = vec![];

        // convert inputs
        let a_val = convert_input(inputs, 0, ValueType::Decimal, &mut input_errors);
        let b_val = convert_input(inputs, 1, ValueType::Decimal, &mut input_errors);

        // return if error
        if input_errors.len() > 0 { return Err(OperationError { input_errors, node_error: None }); }

        // get values
        let Value::Decimal(a) = a_val.unwrap() else { unreachable!() };
        let Value::Decimal(b) = b_val.unwrap() else { unreachable!() };

        // run node
        let value = Value::Decimal((a + b) / 2.0);

        Ok(OperationResponse {
            time: Instant::now().duration_since(start_time),
            responses: vec![OutputResponse {
                value: value,
            }],
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::input::Input;
    use crate::value::Value;

    #[tokio::test]
    async fn test_average_settings() {
        let s = OpNumberMathAverage::settings();
        assert_eq!(s.name, "average");
        assert_eq!(OpNumberMathAverage::create_inputs().len(), 2);
        assert_eq!(OpNumberMathAverage::create_outputs().len(), 1);
    }

    #[tokio::test]
    async fn test_average_zero_and_ten() {
        let mut inputs = vec![
            Input::new("a".to_string(), Value::Decimal(0.0), None, None),
            Input::new("b".to_string(), Value::Decimal(10.0), None, None),
        ];
        let result = OpNumberMathAverage::run(&mut inputs).await.unwrap();
        match &result.responses[0].value {
            Value::Decimal(v) => assert!((*v - 5.0).abs() < 0.01),
            other => panic!("Expected Decimal, got {:?}", other),
        }
    }

    #[tokio::test]
    async fn test_average_same_values() {
        let mut inputs = vec![
            Input::new("a".to_string(), Value::Decimal(3.0), None, None),
            Input::new("b".to_string(), Value::Decimal(3.0), None, None),
        ];
        let result = OpNumberMathAverage::run(&mut inputs).await.unwrap();
        match &result.responses[0].value {
            Value::Decimal(v) => assert!((*v - 3.0).abs() < 0.01),
            other => panic!("Expected Decimal, got {:?}", other),
        }
    }

    #[tokio::test]
    async fn test_average_negative_and_positive() {
        let mut inputs = vec![
            Input::new("a".to_string(), Value::Decimal(-2.0), None, None),
            Input::new("b".to_string(), Value::Decimal(2.0), None, None),
        ];
        let result = OpNumberMathAverage::run(&mut inputs).await.unwrap();
        match &result.responses[0].value {
            Value::Decimal(v) => assert!((*v).abs() < 0.01),
            other => panic!("Expected Decimal, got {:?}", other),
        }
    }
}
