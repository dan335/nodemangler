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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::input::Input;
    use crate::value::Value;

    #[tokio::test]
    async fn test_rand_settings() {
        let s = OpNumberMathRand::settings();
        assert_eq!(s.name, "random");
        assert_eq!(OpNumberMathRand::create_inputs().len(), 2);
        assert_eq!(OpNumberMathRand::create_outputs().len(), 1);
    }

    #[tokio::test]
    async fn test_rand_returns_decimal() {
        let mut inputs = vec![
            Input::new("min".to_string(), Value::Decimal(0.0), None, None),
            Input::new("max".to_string(), Value::Decimal(1.0), None, None),
        ];
        let result = OpNumberMathRand::run(&mut inputs).await.unwrap();
        match &result.responses[0].value {
            Value::Decimal(v) => assert!(*v >= 0.0 && *v <= 1.0, "Got {}", v),
            other => panic!("Expected Decimal, got {:?}", other),
        }
    }

    #[tokio::test]
    async fn test_rand_within_large_range() {
        let mut inputs = vec![
            Input::new("min".to_string(), Value::Decimal(-1000.0), None, None),
            Input::new("max".to_string(), Value::Decimal(1000.0), None, None),
        ];
        let result = OpNumberMathRand::run(&mut inputs).await.unwrap();
        match &result.responses[0].value {
            Value::Decimal(v) => assert!(*v >= -1000.0 && *v <= 1000.0, "Got {}", v),
            other => panic!("Expected Decimal, got {:?}", other),
        }
    }

    #[tokio::test]
    async fn test_rand_min_equals_max_errors() {
        let mut inputs = vec![
            Input::new("min".to_string(), Value::Decimal(5.0), None, None),
            Input::new("max".to_string(), Value::Decimal(5.0), None, None),
        ];
        let result = OpNumberMathRand::run(&mut inputs).await;
        assert!(result.is_err(), "Expected error when min == max");
    }

    #[tokio::test]
    async fn test_rand_min_greater_than_max_errors() {
        let mut inputs = vec![
            Input::new("min".to_string(), Value::Decimal(10.0), None, None),
            Input::new("max".to_string(), Value::Decimal(5.0), None, None),
        ];
        let result = OpNumberMathRand::run(&mut inputs).await;
        assert!(result.is_err(), "Expected error when min > max");
    }

    #[tokio::test]
    async fn test_rand_accepts_integer_inputs() {
        let mut inputs = vec![
            Input::new("min".to_string(), Value::Integer(0), None, None),
            Input::new("max".to_string(), Value::Integer(100), None, None),
        ];
        let result = OpNumberMathRand::run(&mut inputs).await.unwrap();
        match &result.responses[0].value {
            Value::Decimal(v) => assert!(*v >= 0.0 && *v <= 100.0, "Got {}", v),
            other => panic!("Expected Decimal, got {:?}", other),
        }
    }

    #[tokio::test]
    async fn test_rand_multiple_calls_in_range() {
        // Run many times to confirm we always stay in range
        for _ in 0..20 {
            let mut inputs = vec![
                Input::new("min".to_string(), Value::Decimal(0.0), None, None),
                Input::new("max".to_string(), Value::Decimal(1.0), None, None),
            ];
            let result = OpNumberMathRand::run(&mut inputs).await.unwrap();
            match &result.responses[0].value {
                Value::Decimal(v) => assert!(*v >= 0.0 && *v <= 1.0, "Got out-of-range value: {}", v),
                other => panic!("Expected Decimal, got {:?}", other),
            }
        }
    }
}
