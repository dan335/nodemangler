//! Fractional part operation for the node graph.
//!
//! Extracts the fractional (non-integer) part of a decimal using `f32::fract()`.
//! For negative numbers, the fractional part has the same sign as the input
//! (e.g., `fract(-1.5) == -0.5`).

use crate::input::{Input, InputSettings};
use crate::node_settings::NodeSettings;
use crate::operations::{OperationResponse, OperationError, OutputResponse, convert_input};
use crate::output::Output;
use crate::value::{Value, ValueType};
use serde::{Deserialize, Serialize};
use std::time::Instant;

/// Node operation that returns the fractional part of a decimal.
///
/// Input is converted to decimal via `convert_input`. The result is computed
/// using `f32::fract()`, which preserves the sign of the input.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpNumberMathFrac {}

impl OpNumberMathFrac {
    /// Returns the node metadata (name and description).
    pub fn settings() -> NodeSettings {
        NodeSettings {
            name: "frac".to_string(),
            description: "Returns the fractional part of a decimal.".to_string(),
        }
    }

    /// Creates the default input list: a single decimal input defaulting to 3.14.
    pub fn create_inputs() -> Vec<Input> {
        vec![
            Input::new("input".to_string(), Value::Decimal(std::f32::consts::PI), Some(InputSettings::DragValue { speed: None, clamp: None }), None),
        ]
    }

    /// Creates the default output list: a single decimal output.
    pub fn create_outputs() -> Vec<Output> {
        vec![
            Output::new("output".to_string(), Value::Decimal(0.0), None)
        ]
    }

    /// Executes the frac operation: returns the fractional part of the input.
    pub async fn run(inputs: &mut [Input]) -> Result<OperationResponse, OperationError> {
        let start_time = Instant::now();
        let mut input_errors: Vec<(usize, String)> = vec![];

        let input_converted = convert_input(inputs, 0, ValueType::Decimal, &mut input_errors);

        if !input_errors.is_empty() { return Err(OperationError { input_errors, node_error: None }); }

        let Value::Decimal(val) = input_converted.unwrap() else { unreachable!() };

        let result = val.fract();

        Ok(OperationResponse {
            time: Instant::now().duration_since(start_time),
            responses: vec![OutputResponse {
                value: Value::Decimal(result),
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
    async fn test_frac_settings() {
        let s = OpNumberMathFrac::settings();
        assert_eq!(s.name, "frac");
        assert_eq!(OpNumberMathFrac::create_inputs().len(), 1);
        assert_eq!(OpNumberMathFrac::create_outputs().len(), 1);
    }

    #[tokio::test]
    async fn test_frac_basic() {
        let mut inputs = vec![Input::new("input".to_string(), Value::Decimal(std::f32::consts::PI), None, None)];
        let result = OpNumberMathFrac::run(&mut inputs).await.unwrap();
        match &result.responses[0].value {
            Value::Decimal(v) => assert!((*v - 0.14).abs() < 0.01),
            other => panic!("Expected Decimal, got {:?}", other),
        }
    }

    #[tokio::test]
    async fn test_frac_whole_number() {
        let mut inputs = vec![Input::new("input".to_string(), Value::Decimal(5.0), None, None)];
        let result = OpNumberMathFrac::run(&mut inputs).await.unwrap();
        match &result.responses[0].value {
            Value::Decimal(v) => assert!((*v).abs() < 0.01),
            other => panic!("Expected Decimal, got {:?}", other),
        }
    }

    #[tokio::test]
    async fn test_frac_zero() {
        let mut inputs = vec![Input::new("input".to_string(), Value::Decimal(0.0), None, None)];
        let result = OpNumberMathFrac::run(&mut inputs).await.unwrap();
        match &result.responses[0].value {
            Value::Decimal(v) => assert!((*v).abs() < 1e-6),
            other => panic!("Expected Decimal, got {:?}", other),
        }
    }

    #[tokio::test]
    async fn test_frac_negative() {
        // fract(-1.5) == -0.5 in Rust
        let mut inputs = vec![Input::new("input".to_string(), Value::Decimal(-1.5), None, None)];
        let result = OpNumberMathFrac::run(&mut inputs).await.unwrap();
        match &result.responses[0].value {
            Value::Decimal(v) => assert!((*v - (-0.5)).abs() < 1e-5),
            other => panic!("Expected Decimal, got {:?}", other),
        }
    }

    #[tokio::test]
    async fn test_frac_from_integer() {
        // Integer is converted to Decimal, frac of whole number is 0
        let mut inputs = vec![Input::new("input".to_string(), Value::Integer(7), None, None)];
        let result = OpNumberMathFrac::run(&mut inputs).await.unwrap();
        match &result.responses[0].value {
            Value::Decimal(v) => assert!((*v).abs() < 1e-5),
            other => panic!("Expected Decimal, got {:?}", other),
        }
    }

    #[tokio::test]
    async fn test_frac_large_number() {
        let mut inputs = vec![Input::new("input".to_string(), Value::Decimal(1234567.89), None, None)];
        let result = OpNumberMathFrac::run(&mut inputs).await.unwrap();
        match &result.responses[0].value {
            // Due to f32 precision, check the frac is between 0 and 1
            Value::Decimal(v) => assert!(*v >= 0.0 && *v < 1.0),
            other => panic!("Expected Decimal, got {:?}", other),
        }
    }

    #[tokio::test]
    async fn test_frac_small_decimal() {
        let mut inputs = vec![Input::new("input".to_string(), Value::Decimal(0.0001), None, None)];
        let result = OpNumberMathFrac::run(&mut inputs).await.unwrap();
        match &result.responses[0].value {
            Value::Decimal(v) => assert!((*v - 0.0001).abs() < 1e-7),
            other => panic!("Expected Decimal, got {:?}", other),
        }
    }
}
