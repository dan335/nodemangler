use crate::input::{Input, InputSettings};
use crate::node_settings::NodeSettings;
use crate::operations::{OperationResponse, OperationError, OutputResponse};
use crate::output::Output;
use crate::value::Value;
use serde::{Deserialize, Serialize};
use std::time::Instant;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpNumberMathIncrement {}

impl OpNumberMathIncrement {
    pub fn settings() -> NodeSettings {
        NodeSettings {
            name: "increment".to_string(),
            description: "Increments a number by 1.".to_string(),
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
            Value::Integer(a) => Value::Integer(*a + 1),

            Value::Decimal(a) => Value::Decimal(*a + 1.0),

            Value::String(a) => Value::String(format!("{} {}", *a, "+1")),

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
    async fn test_increment_settings() {
        let s = OpNumberMathIncrement::settings();
        assert_eq!(s.name, "increment");
        assert_eq!(OpNumberMathIncrement::create_inputs().len(), 1);
        assert_eq!(OpNumberMathIncrement::create_outputs().len(), 1);
    }

    #[tokio::test]
    async fn test_increment_basic() {
        let mut inputs = vec![Input::new("input".to_string(), Value::Decimal(5.0), None, None)];
        let result = OpNumberMathIncrement::run(&mut inputs).await.unwrap();
        match &result.responses[0].value {
            Value::Decimal(v) => assert!((*v - 6.0).abs() < 0.01),
            other => panic!("Expected Decimal, got {:?}", other),
        }
    }

    #[tokio::test]
    async fn test_increment_integer() {
        let mut inputs = vec![Input::new("a".to_string(), Value::Integer(10), None, None)];
        let result = OpNumberMathIncrement::run(&mut inputs).await.unwrap();
        match &result.responses[0].value {
            Value::Integer(v) => assert_eq!(*v, 11),
            other => panic!("Expected Integer, got {:?}", other),
        }
    }

    #[tokio::test]
    async fn test_increment_zero() {
        let mut inputs = vec![Input::new("a".to_string(), Value::Integer(0), None, None)];
        let result = OpNumberMathIncrement::run(&mut inputs).await.unwrap();
        match &result.responses[0].value {
            Value::Integer(v) => assert_eq!(*v, 1),
            other => panic!("Expected Integer, got {:?}", other),
        }
    }

    #[tokio::test]
    async fn test_increment_negative() {
        let mut inputs = vec![Input::new("a".to_string(), Value::Integer(-5), None, None)];
        let result = OpNumberMathIncrement::run(&mut inputs).await.unwrap();
        match &result.responses[0].value {
            Value::Integer(v) => assert_eq!(*v, -4),
            other => panic!("Expected Integer, got {:?}", other),
        }
    }

    #[tokio::test]
    async fn test_increment_negative_decimal() {
        let mut inputs = vec![Input::new("a".to_string(), Value::Decimal(-1.5), None, None)];
        let result = OpNumberMathIncrement::run(&mut inputs).await.unwrap();
        match &result.responses[0].value {
            Value::Decimal(v) => assert!((*v - (-0.5)).abs() < 1e-5),
            other => panic!("Expected Decimal, got {:?}", other),
        }
    }

    #[tokio::test]
    async fn test_increment_string() {
        let mut inputs = vec![Input::new("a".to_string(), Value::String("hello".to_string()), None, None)];
        let result = OpNumberMathIncrement::run(&mut inputs).await.unwrap();
        match &result.responses[0].value {
            Value::String(s) => assert_eq!(s, "hello +1"),
            other => panic!("Expected String, got {:?}", other),
        }
    }

    #[tokio::test]
    async fn test_increment_invalid_type_returns_error() {
        let mut inputs = vec![Input::new("a".to_string(), Value::Bool(true), None, None)];
        let result = OpNumberMathIncrement::run(&mut inputs).await;
        assert!(result.is_err(), "Expected error for Bool input");
    }

    #[tokio::test]
    async fn test_increment_large_integer() {
        let mut inputs = vec![Input::new("a".to_string(), Value::Integer(i32::MAX / 2), None, None)];
        let result = OpNumberMathIncrement::run(&mut inputs).await.unwrap();
        match &result.responses[0].value {
            Value::Integer(v) => assert_eq!(*v, i32::MAX / 2 + 1),
            other => panic!("Expected Integer, got {:?}", other),
        }
    }
}
