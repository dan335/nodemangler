//! Least common multiple (LCM) operation for the node graph.
//!
//! Computes `lcm(a, b) = |a * b| / gcd(a, b)`. Returns 0 if either input is 0.
//! Uses i64 intermediate multiplication to avoid overflow for large i32 inputs.

use crate::input::{Input, InputSettings};
use crate::node_settings::NodeSettings;
use crate::operations::{OperationResponse, OperationError, OutputResponse, convert_input};
use crate::output::Output;
use crate::value::{Value, ValueType};
use serde::{Deserialize, Serialize};
use std::time::Instant;

/// Node operation that computes the least common multiple of two integers.
///
/// Both inputs are converted to integers. If either is zero, the result is 0.
/// Internally uses i64 for the product to avoid overflow.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpNumberMathLcm {}

impl OpNumberMathLcm {
    /// Returns the node metadata (name and description).
    pub fn settings() -> NodeSettings {
        NodeSettings {
            name: "lcm".to_string(),
            description: "Computes the least common multiple.".to_string(),
        }
    }

    /// Creates the default input list: two integer drag-value inputs (a=4, b=6).
    pub fn create_inputs() -> Vec<Input> {
        vec![
            Input::new("a".to_string(), Value::Integer(4), Some(InputSettings::DragValue { speed: None, clamp: None }), None),
            Input::new("b".to_string(), Value::Integer(6), Some(InputSettings::DragValue { speed: None, clamp: None }), None),
        ]
    }

    /// Creates the default output list: a single integer output.
    pub fn create_outputs() -> Vec<Output> {
        vec![
            Output::new("output".to_string(), Value::Integer(0), None)
        ]
    }

    /// Computes GCD using the Euclidean algorithm (helper for LCM computation).
    fn gcd(a: i32, b: i32) -> i32 {
        let mut a = a.abs();
        let mut b = b.abs();
        while b != 0 {
            let t = b;
            b = a % b;
            a = t;
        }
        a
    }

    /// Executes the LCM computation: `|a * b| / gcd(a, b)`.
    pub async fn run(inputs: &mut [Input]) -> Result<OperationResponse, OperationError> {
        let start_time = Instant::now();
        let mut input_errors: Vec<(usize, String)> = vec![];

        let a_converted = convert_input(inputs, 0, ValueType::Integer, &mut input_errors);
        let b_converted = convert_input(inputs, 1, ValueType::Integer, &mut input_errors);

        if !input_errors.is_empty() { return Err(OperationError { input_errors, node_error: None }); }

        let Value::Integer(a) = a_converted.unwrap() else { unreachable!() };
        let Value::Integer(b) = b_converted.unwrap() else { unreachable!() };

        let result = if a == 0 || b == 0 {
            0
        } else {
            ((a as i64) * (b as i64)).abs() as i32 / Self::gcd(a, b)
        };

        Ok(OperationResponse {
            time: Instant::now().duration_since(start_time),
            responses: vec![OutputResponse {
                value: Value::Integer(result),
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
    async fn test_lcm_settings() {
        let s = OpNumberMathLcm::settings();
        assert_eq!(s.name, "lcm");
        assert_eq!(OpNumberMathLcm::create_inputs().len(), 2);
        assert_eq!(OpNumberMathLcm::create_outputs().len(), 1);
    }

    #[tokio::test]
    async fn test_lcm_basic() {
        let mut inputs = vec![
            Input::new("a".to_string(), Value::Integer(4), None, None),
            Input::new("b".to_string(), Value::Integer(6), None, None),
        ];
        let result = OpNumberMathLcm::run(&mut inputs).await.unwrap();
        match &result.responses[0].value {
            Value::Integer(v) => assert_eq!(*v, 12),
            other => panic!("Expected Integer, got {:?}", other),
        }
    }

    #[tokio::test]
    async fn test_lcm_with_zero() {
        let mut inputs = vec![
            Input::new("a".to_string(), Value::Integer(5), None, None),
            Input::new("b".to_string(), Value::Integer(0), None, None),
        ];
        let result = OpNumberMathLcm::run(&mut inputs).await.unwrap();
        match &result.responses[0].value {
            Value::Integer(v) => assert_eq!(*v, 0),
            other => panic!("Expected Integer, got {:?}", other),
        }
    }

    #[tokio::test]
    async fn test_lcm_zero_a() {
        let mut inputs = vec![
            Input::new("a".to_string(), Value::Integer(0), None, None),
            Input::new("b".to_string(), Value::Integer(5), None, None),
        ];
        let result = OpNumberMathLcm::run(&mut inputs).await.unwrap();
        match &result.responses[0].value {
            Value::Integer(v) => assert_eq!(*v, 0),
            other => panic!("Expected Integer, got {:?}", other),
        }
    }

    #[tokio::test]
    async fn test_lcm_same_numbers() {
        let mut inputs = vec![
            Input::new("a".to_string(), Value::Integer(7), None, None),
            Input::new("b".to_string(), Value::Integer(7), None, None),
        ];
        let result = OpNumberMathLcm::run(&mut inputs).await.unwrap();
        match &result.responses[0].value {
            Value::Integer(v) => assert_eq!(*v, 7),
            other => panic!("Expected Integer, got {:?}", other),
        }
    }

    #[tokio::test]
    async fn test_lcm_coprime() {
        let mut inputs = vec![
            Input::new("a".to_string(), Value::Integer(7), None, None),
            Input::new("b".to_string(), Value::Integer(13), None, None),
        ];
        let result = OpNumberMathLcm::run(&mut inputs).await.unwrap();
        match &result.responses[0].value {
            Value::Integer(v) => assert_eq!(*v, 91),
            other => panic!("Expected Integer, got {:?}", other),
        }
    }

    #[tokio::test]
    async fn test_lcm_negative_inputs() {
        // lcm(-4, 6) = lcm(4, 6) = 12
        let mut inputs = vec![
            Input::new("a".to_string(), Value::Integer(-4), None, None),
            Input::new("b".to_string(), Value::Integer(6), None, None),
        ];
        let result = OpNumberMathLcm::run(&mut inputs).await.unwrap();
        match &result.responses[0].value {
            Value::Integer(v) => assert_eq!(*v, 12),
            other => panic!("Expected Integer, got {:?}", other),
        }
    }

    #[tokio::test]
    async fn test_lcm_from_decimal() {
        let mut inputs = vec![
            Input::new("a".to_string(), Value::Decimal(4.0), None, None),
            Input::new("b".to_string(), Value::Decimal(6.0), None, None),
        ];
        let result = OpNumberMathLcm::run(&mut inputs).await.unwrap();
        match &result.responses[0].value {
            Value::Integer(v) => assert_eq!(*v, 12),
            other => panic!("Expected Integer, got {:?}", other),
        }
    }

    #[tokio::test]
    async fn test_lcm_one_and_n() {
        let mut inputs = vec![
            Input::new("a".to_string(), Value::Integer(1), None, None),
            Input::new("b".to_string(), Value::Integer(15), None, None),
        ];
        let result = OpNumberMathLcm::run(&mut inputs).await.unwrap();
        match &result.responses[0].value {
            Value::Integer(v) => assert_eq!(*v, 15),
            other => panic!("Expected Integer, got {:?}", other),
        }
    }
}
