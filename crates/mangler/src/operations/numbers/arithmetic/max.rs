use crate::input::{Input, InputSettings};
use crate::node_settings::NodeSettings;
use crate::operations::{OperationResponse, OperationError, OutputResponse};
use crate::output::Output;
use crate::value::Value;
use serde::{Deserialize, Serialize};
use std::time::Instant;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpNumberMathMax {}

impl OpNumberMathMax {
    pub fn settings() -> NodeSettings {
        NodeSettings {
            name: "max".to_string(),
            description: "Returns the largest of two numbers.".to_string(),
        }
    }

    pub fn create_inputs() -> Vec<Input> {
        vec![
            Input::new("a".to_string(), Value::Decimal(1.0), Some(InputSettings::DragValue { speed:None, clamp:None }), None),
            Input::new("b".to_string(), Value::Decimal(1.0), Some(InputSettings::DragValue { speed:None, clamp:None }), None)
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

        let value = match (&inputs[0].value, &inputs[1].value) {
            (Value::Integer(a), Value::Decimal(b)) => Value::Decimal((*a as f32).max(*b)),

            (Value::Integer(a), Value::Integer(b)) => Value::Integer(*a.max(b)),

            (Value::Decimal(a), Value::Decimal(b)) => Value::Decimal(a.max(*b)),

            (Value::Decimal(a), Value::Integer(b)) => Value::Decimal(a.max(*b as f32)),

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
    async fn test_max_settings() {
        let s = OpNumberMathMax::settings();
        assert_eq!(s.name, "max");
        assert_eq!(OpNumberMathMax::create_inputs().len(), 2);
        assert_eq!(OpNumberMathMax::create_outputs().len(), 1);
    }

    #[tokio::test]
    async fn test_max_basic() {
        let mut inputs = vec![
            Input::new("a".to_string(), Value::Decimal(3.0), None, None),
            Input::new("b".to_string(), Value::Decimal(7.0), None, None),
        ];
        let result = OpNumberMathMax::run(&mut inputs).await.unwrap();
        match &result.responses[0].value {
            Value::Decimal(v) => assert!((*v - 7.0).abs() < 0.01),
            other => panic!("Expected Decimal, got {:?}", other),
        }
    }

    #[tokio::test]
    async fn test_max_equal() {
        let mut inputs = vec![
            Input::new("a".to_string(), Value::Decimal(5.0), None, None),
            Input::new("b".to_string(), Value::Decimal(5.0), None, None),
        ];
        let result = OpNumberMathMax::run(&mut inputs).await.unwrap();
        match &result.responses[0].value {
            Value::Decimal(v) => assert!((*v - 5.0).abs() < 0.01),
            other => panic!("Expected Decimal, got {:?}", other),
        }
    }

    #[tokio::test]
    async fn test_max_integer_integer() {
        let mut inputs = vec![
            Input::new("a".to_string(), Value::Integer(10), None, None),
            Input::new("b".to_string(), Value::Integer(20), None, None),
        ];
        let result = OpNumberMathMax::run(&mut inputs).await.unwrap();
        match &result.responses[0].value {
            Value::Integer(v) => assert_eq!(*v, 20),
            other => panic!("Expected Integer, got {:?}", other),
        }
    }

    #[tokio::test]
    async fn test_max_integer_decimal() {
        let mut inputs = vec![
            Input::new("a".to_string(), Value::Integer(5), None, None),
            Input::new("b".to_string(), Value::Decimal(3.0), None, None),
        ];
        let result = OpNumberMathMax::run(&mut inputs).await.unwrap();
        match &result.responses[0].value {
            Value::Decimal(v) => assert!((*v - 5.0).abs() < 1e-5),
            other => panic!("Expected Decimal, got {:?}", other),
        }
    }

    #[tokio::test]
    async fn test_max_decimal_integer() {
        let mut inputs = vec![
            Input::new("a".to_string(), Value::Decimal(3.0), None, None),
            Input::new("b".to_string(), Value::Integer(5), None, None),
        ];
        let result = OpNumberMathMax::run(&mut inputs).await.unwrap();
        match &result.responses[0].value {
            Value::Decimal(v) => assert!((*v - 5.0).abs() < 1e-5),
            other => panic!("Expected Decimal, got {:?}", other),
        }
    }

    #[tokio::test]
    async fn test_max_both_negative() {
        let mut inputs = vec![
            Input::new("a".to_string(), Value::Integer(-3), None, None),
            Input::new("b".to_string(), Value::Integer(-10), None, None),
        ];
        let result = OpNumberMathMax::run(&mut inputs).await.unwrap();
        match &result.responses[0].value {
            Value::Integer(v) => assert_eq!(*v, -3),
            other => panic!("Expected Integer, got {:?}", other),
        }
    }

    #[tokio::test]
    async fn test_max_mixed_sign() {
        let mut inputs = vec![
            Input::new("a".to_string(), Value::Integer(-5), None, None),
            Input::new("b".to_string(), Value::Integer(5), None, None),
        ];
        let result = OpNumberMathMax::run(&mut inputs).await.unwrap();
        match &result.responses[0].value {
            Value::Integer(v) => assert_eq!(*v, 5),
            other => panic!("Expected Integer, got {:?}", other),
        }
    }

    #[tokio::test]
    async fn test_max_with_zero() {
        let mut inputs = vec![
            Input::new("a".to_string(), Value::Integer(0), None, None),
            Input::new("b".to_string(), Value::Integer(-1), None, None),
        ];
        let result = OpNumberMathMax::run(&mut inputs).await.unwrap();
        match &result.responses[0].value {
            Value::Integer(v) => assert_eq!(*v, 0),
            other => panic!("Expected Integer, got {:?}", other),
        }
    }

    #[tokio::test]
    async fn test_max_invalid_type_returns_error() {
        let mut inputs = vec![
            Input::new("a".to_string(), Value::Bool(true), None, None),
            Input::new("b".to_string(), Value::Integer(5), None, None),
        ];
        let result = OpNumberMathMax::run(&mut inputs).await;
        assert!(result.is_err(), "Expected error for unsupported type");
    }
}
