use crate::input::{Input, InputSettings};
use crate::node_settings::NodeSettings;
use crate::operations::{OperationResponse, OperationError, OutputResponse};
use crate::output::Output;
use crate::value::Value;
use serde::{Deserialize, Serialize};
use std::time::Instant;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpNumberMathDivide {}

impl OpNumberMathDivide {
    pub fn settings() -> NodeSettings {
        NodeSettings {
            name: "divide".to_string(),
            description: "Divides two numbers.".to_string(),
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

        // Check for division by zero
        let is_zero = match &inputs[1].value {
            Value::Integer(b) => *b == 0,
            Value::Decimal(b) => *b == 0.0,
            _ => false,
        };
        if is_zero {
            return Err(OperationError {
                input_errors: vec![(1, "Division by zero.".to_string())], node_error: None,
            });
        }

        let value = match (&inputs[0].value, &inputs[1].value) {
            (Value::Integer(a), Value::Decimal(b)) => Value::Decimal(*a as f32 / *b),

            (Value::Integer(a), Value::Integer(b)) => Value::Integer(*a / *b),

            (Value::Decimal(a), Value::Decimal(b)) => Value::Decimal(*a / *b),

            (Value::Decimal(a), Value::Integer(b)) => Value::Decimal(*a / *b as f32),

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
    async fn test_divide_settings() {
        let s = OpNumberMathDivide::settings();
        assert_eq!(s.name, "divide");
        assert_eq!(OpNumberMathDivide::create_inputs().len(), 2);
        assert_eq!(OpNumberMathDivide::create_outputs().len(), 1);
    }

    #[tokio::test]
    async fn test_divide_basic() {
        let mut inputs = vec![
            Input::new("a".to_string(), Value::Decimal(20.0), None, None),
            Input::new("b".to_string(), Value::Decimal(4.0), None, None),
        ];
        let result = OpNumberMathDivide::run(&mut inputs).await.unwrap();
        match &result.responses[0].value {
            Value::Decimal(v) => assert!((*v - 5.0).abs() < 0.01),
            other => panic!("Expected Decimal, got {:?}", other),
        }
    }

    #[tokio::test]
    async fn test_divide_integer_by_zero_errors() {
        let mut inputs = vec![
            Input::new("a".to_string(), Value::Integer(10), None, None),
            Input::new("b".to_string(), Value::Integer(0), None, None),
        ];
        let result = OpNumberMathDivide::run(&mut inputs).await;
        assert!(result.is_err(), "Expected error for integer division by zero");
    }

    #[tokio::test]
    async fn test_divide_decimal_by_zero_errors() {
        let mut inputs = vec![
            Input::new("a".to_string(), Value::Decimal(5.0), None, None),
            Input::new("b".to_string(), Value::Decimal(0.0), None, None),
        ];
        let result = OpNumberMathDivide::run(&mut inputs).await;
        assert!(result.is_err(), "Expected error for decimal division by zero");
    }

    #[tokio::test]
    async fn test_divide_integer_integer() {
        let mut inputs = vec![
            Input::new("a".to_string(), Value::Integer(15), None, None),
            Input::new("b".to_string(), Value::Integer(3), None, None),
        ];
        let result = OpNumberMathDivide::run(&mut inputs).await.unwrap();
        match &result.responses[0].value {
            Value::Integer(v) => assert_eq!(*v, 5),
            other => panic!("Expected Integer, got {:?}", other),
        }
    }

    #[tokio::test]
    async fn test_divide_integer_decimal() {
        let mut inputs = vec![
            Input::new("a".to_string(), Value::Integer(10), None, None),
            Input::new("b".to_string(), Value::Decimal(4.0), None, None),
        ];
        let result = OpNumberMathDivide::run(&mut inputs).await.unwrap();
        match &result.responses[0].value {
            Value::Decimal(v) => assert!((*v - 2.5).abs() < 1e-5),
            other => panic!("Expected Decimal, got {:?}", other),
        }
    }

    #[tokio::test]
    async fn test_divide_decimal_integer() {
        let mut inputs = vec![
            Input::new("a".to_string(), Value::Decimal(7.5), None, None),
            Input::new("b".to_string(), Value::Integer(3), None, None),
        ];
        let result = OpNumberMathDivide::run(&mut inputs).await.unwrap();
        match &result.responses[0].value {
            Value::Decimal(v) => assert!((*v - 2.5).abs() < 1e-5),
            other => panic!("Expected Decimal, got {:?}", other),
        }
    }

    #[tokio::test]
    async fn test_divide_negative_by_positive() {
        let mut inputs = vec![
            Input::new("a".to_string(), Value::Integer(-12), None, None),
            Input::new("b".to_string(), Value::Integer(4), None, None),
        ];
        let result = OpNumberMathDivide::run(&mut inputs).await.unwrap();
        match &result.responses[0].value {
            Value::Integer(v) => assert_eq!(*v, -3),
            other => panic!("Expected Integer, got {:?}", other),
        }
    }

    #[tokio::test]
    async fn test_divide_negative_by_negative() {
        let mut inputs = vec![
            Input::new("a".to_string(), Value::Integer(-12), None, None),
            Input::new("b".to_string(), Value::Integer(-4), None, None),
        ];
        let result = OpNumberMathDivide::run(&mut inputs).await.unwrap();
        match &result.responses[0].value {
            Value::Integer(v) => assert_eq!(*v, 3),
            other => panic!("Expected Integer, got {:?}", other),
        }
    }

    #[tokio::test]
    async fn test_divide_fractional_result() {
        let mut inputs = vec![
            Input::new("a".to_string(), Value::Decimal(1.0), None, None),
            Input::new("b".to_string(), Value::Decimal(3.0), None, None),
        ];
        let result = OpNumberMathDivide::run(&mut inputs).await.unwrap();
        match &result.responses[0].value {
            Value::Decimal(v) => assert!((*v - 0.33333).abs() < 1e-4),
            other => panic!("Expected Decimal, got {:?}", other),
        }
    }

    #[tokio::test]
    async fn test_divide_zero_by_nonzero() {
        let mut inputs = vec![
            Input::new("a".to_string(), Value::Integer(0), None, None),
            Input::new("b".to_string(), Value::Integer(5), None, None),
        ];
        let result = OpNumberMathDivide::run(&mut inputs).await.unwrap();
        match &result.responses[0].value {
            Value::Integer(v) => assert_eq!(*v, 0),
            other => panic!("Expected Integer, got {:?}", other),
        }
    }

    #[tokio::test]
    async fn test_divide_invalid_type_returns_error() {
        let mut inputs = vec![
            Input::new("a".to_string(), Value::Bool(true), None, None),
            Input::new("b".to_string(), Value::Integer(2), None, None),
        ];
        let result = OpNumberMathDivide::run(&mut inputs).await;
        assert!(result.is_err(), "Expected error for unsupported type");
    }
}
