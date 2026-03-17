use crate::input::{Input, InputSettings};
use crate::node_settings::NodeSettings;
use crate::operations::{OperationResponse, OperationError, OutputResponse};
use crate::output::Output;
use crate::value::Value;
use serde::{Deserialize, Serialize};
use std::time::Instant;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpNumberMathAbs {}

impl OpNumberMathAbs {
    pub fn settings() -> NodeSettings {
        NodeSettings {
            name: "absolute value".to_string(),
            description: "Returns the absolute value of a number.".to_string(),
        }
    }

    pub fn create_inputs() -> Vec<Input> {
        vec![
            Input::new("a".to_string(), Value::Decimal(1.0), Some(InputSettings::DragValue { speed:None, clamp:None }), None)
        ]
    }

    pub fn create_outputs() -> Vec<Output> {
        vec![
            Output::new("output".to_string(), Value::Decimal(f32::default()), None)
        ]
    }

    pub async fn run(inputs: &mut Vec<Input>) -> Result<OperationResponse, OperationError> {
        let start_time = Instant::now();
        let input_errors: Vec<(usize, String)> = vec![];

        // convert inputs
        // gather errors

        // return if error
        if input_errors.len() > 0 { return Err(OperationError { input_errors, node_error: None }); }

        // get values
        // run node

        let value = match &inputs[0].value {

            Value::Integer(a) => Value::Integer(a.clone().abs()),

            Value::Decimal(a) => Value::Decimal(a.clone().abs()),

            _ => {return Err(OperationError {
                input_errors: vec![], node_error: Some("Error converting.".to_string()),
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::input::Input;
    use crate::value::Value;

    #[tokio::test]
    async fn test_abs_settings() {
        let s = OpNumberMathAbs::settings();
        assert_eq!(s.name, "absolute value");
        assert_eq!(OpNumberMathAbs::create_inputs().len(), 1);
        assert_eq!(OpNumberMathAbs::create_outputs().len(), 1);
    }

    #[tokio::test]
    async fn test_abs_negative() {
        let mut inputs = vec![Input::new("input".to_string(), Value::Decimal(-5.0), None, None)];
        let result = OpNumberMathAbs::run(&mut inputs).await.unwrap();
        match &result.responses[0].value {
            Value::Decimal(v) => assert!((*v - 5.0).abs() < 0.01),
            other => panic!("Expected Decimal, got {:?}", other),
        }
    }

    #[tokio::test]
    async fn test_abs_positive() {
        let mut inputs = vec![Input::new("input".to_string(), Value::Decimal(5.0), None, None)];
        let result = OpNumberMathAbs::run(&mut inputs).await.unwrap();
        match &result.responses[0].value {
            Value::Decimal(v) => assert!((*v - 5.0).abs() < 0.01),
            other => panic!("Expected Decimal, got {:?}", other),
        }
    }

    #[tokio::test]
    async fn test_abs_zero() {
        let mut inputs = vec![Input::new("input".to_string(), Value::Decimal(0.0), None, None)];
        let result = OpNumberMathAbs::run(&mut inputs).await.unwrap();
        match &result.responses[0].value {
            Value::Decimal(v) => assert!((*v).abs() < 0.01),
            other => panic!("Expected Decimal, got {:?}", other),
        }
    }

    #[tokio::test]
    async fn test_abs_integer_positive() {
        let mut inputs = vec![Input::new("a".to_string(), Value::Integer(42), None, None)];
        let result = OpNumberMathAbs::run(&mut inputs).await.unwrap();
        match &result.responses[0].value {
            Value::Integer(v) => assert_eq!(*v, 42),
            other => panic!("Expected Integer, got {:?}", other),
        }
    }

    #[tokio::test]
    async fn test_abs_integer_negative() {
        let mut inputs = vec![Input::new("a".to_string(), Value::Integer(-42), None, None)];
        let result = OpNumberMathAbs::run(&mut inputs).await.unwrap();
        match &result.responses[0].value {
            Value::Integer(v) => assert_eq!(*v, 42),
            other => panic!("Expected Integer, got {:?}", other),
        }
    }

    #[tokio::test]
    async fn test_abs_integer_zero() {
        let mut inputs = vec![Input::new("a".to_string(), Value::Integer(0), None, None)];
        let result = OpNumberMathAbs::run(&mut inputs).await.unwrap();
        match &result.responses[0].value {
            Value::Integer(v) => assert_eq!(*v, 0),
            other => panic!("Expected Integer, got {:?}", other),
        }
    }

    #[tokio::test]
    async fn test_abs_large_negative_integer() {
        let mut inputs = vec![Input::new("a".to_string(), Value::Integer(-(i32::MAX / 2)), None, None)];
        let result = OpNumberMathAbs::run(&mut inputs).await.unwrap();
        match &result.responses[0].value {
            Value::Integer(v) => assert_eq!(*v, i32::MAX / 2),
            other => panic!("Expected Integer, got {:?}", other),
        }
    }

    #[tokio::test]
    async fn test_abs_small_negative_decimal() {
        let mut inputs = vec![Input::new("a".to_string(), Value::Decimal(-0.0001), None, None)];
        let result = OpNumberMathAbs::run(&mut inputs).await.unwrap();
        match &result.responses[0].value {
            Value::Decimal(v) => assert!((*v - 0.0001).abs() < 1e-7),
            other => panic!("Expected Decimal, got {:?}", other),
        }
    }

    #[tokio::test]
    async fn test_abs_invalid_type_returns_error() {
        let mut inputs = vec![Input::new("a".to_string(), Value::Bool(false), None, None)];
        let result = OpNumberMathAbs::run(&mut inputs).await;
        assert!(result.is_err(), "Expected error for Bool input");
    }
}
