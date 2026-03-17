use crate::input::{Input, InputSettings};
use crate::node_settings::NodeSettings;
use crate::operations::{OperationResponse, OperationError, OutputResponse};
use crate::output::Output;
use crate::value::{Value, ValueType};
use serde::{Deserialize, Serialize};
use std::time::Instant;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpNumberMathNthRt {}

impl OpNumberMathNthRt {
    pub fn settings() -> NodeSettings {
        NodeSettings {
            name: "nth root".to_string(),
            description: "Returns the nth root of a number.".to_string(),
        }
    }

    pub fn create_inputs() -> Vec<Input> {
        vec![
            Input::new("a".to_string(), Value::Decimal(1.0), Some(InputSettings::DragValue { speed:None, clamp:None }), None),
            Input::new("n".to_string(), Value::Decimal(1.0), Some(InputSettings::DragValue { speed:None, clamp:None }), None)
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

        let Ok(Value::Decimal(n)) = inputs[1].value.try_convert_to(ValueType::Decimal) else { return Err(OperationError { input_errors: vec![(1, "Unable to convert 'n' to Decimal.".to_string())], node_error: None })};

        if n == 0.0 {
            return Err(OperationError {
                input_errors: vec![(1, "Root degree cannot be zero.".to_string())], node_error: None,
            });
        }

        let num = match &inputs[0].value {
            Value::Integer(a) => Some(*a as f32),
            Value::Decimal(a) => Some(a.clone()),
            _ => None,
        };

        if let Some(mut num) = num {
            num = num.max(0.0);

            let nth_root = num.powf(1.0 / n);

            Ok(OperationResponse {
                time: Instant::now().duration_since(start_time),
                responses: vec![OutputResponse {
                    value: Value::Decimal(nth_root),
                }],
            })
        } else {
            return Err(OperationError {
                input_errors: vec![(0, "Unable to convert to a number.".to_string())],
                node_error: None,
            });
        }

        
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::input::Input;
    use crate::value::Value;

    #[tokio::test]
    async fn test_nth_root_settings() {
        let s = OpNumberMathNthRt::settings();
        assert_eq!(s.name, "nth root");
        assert_eq!(OpNumberMathNthRt::create_inputs().len(), 2);
        assert_eq!(OpNumberMathNthRt::create_outputs().len(), 1);
    }

    #[tokio::test]
    async fn test_nth_root_square() {
        let mut inputs = vec![
            Input::new("input".to_string(), Value::Decimal(16.0), None, None),
            Input::new("n".to_string(), Value::Decimal(2.0), None, None),
        ];
        let result = OpNumberMathNthRt::run(&mut inputs).await.unwrap();
        match &result.responses[0].value {
            Value::Decimal(v) => assert!((*v - 4.0).abs() < 0.01),
            other => panic!("Expected Decimal, got {:?}", other),
        }
    }

    #[tokio::test]
    async fn test_nth_root_cube() {
        let mut inputs = vec![
            Input::new("input".to_string(), Value::Decimal(8.0), None, None),
            Input::new("n".to_string(), Value::Decimal(3.0), None, None),
        ];
        let result = OpNumberMathNthRt::run(&mut inputs).await.unwrap();
        match &result.responses[0].value {
            Value::Decimal(v) => assert!((*v - 2.0).abs() < 0.01),
            other => panic!("Expected Decimal, got {:?}", other),
        }
    }

    #[tokio::test]
    async fn test_nth_root_zero_n_errors() {
        let mut inputs = vec![
            Input::new("a".to_string(), Value::Decimal(8.0), None, None),
            Input::new("n".to_string(), Value::Decimal(0.0), None, None),
        ];
        let result = OpNumberMathNthRt::run(&mut inputs).await;
        assert!(result.is_err(), "Expected error for root degree 0");
    }

    #[tokio::test]
    async fn test_nth_root_of_zero() {
        let mut inputs = vec![
            Input::new("a".to_string(), Value::Decimal(0.0), None, None),
            Input::new("n".to_string(), Value::Decimal(2.0), None, None),
        ];
        let result = OpNumberMathNthRt::run(&mut inputs).await.unwrap();
        match &result.responses[0].value {
            Value::Decimal(v) => assert!((*v).abs() < 1e-5),
            other => panic!("Expected Decimal, got {:?}", other),
        }
    }

    #[tokio::test]
    async fn test_nth_root_n_one() {
        // n=1 root of any number is the number itself
        let mut inputs = vec![
            Input::new("a".to_string(), Value::Decimal(42.0), None, None),
            Input::new("n".to_string(), Value::Decimal(1.0), None, None),
        ];
        let result = OpNumberMathNthRt::run(&mut inputs).await.unwrap();
        match &result.responses[0].value {
            Value::Decimal(v) => assert!((*v - 42.0).abs() < 1e-3),
            other => panic!("Expected Decimal, got {:?}", other),
        }
    }

    #[tokio::test]
    async fn test_nth_root_of_one() {
        // Any root of 1 is 1
        let mut inputs = vec![
            Input::new("a".to_string(), Value::Decimal(1.0), None, None),
            Input::new("n".to_string(), Value::Decimal(5.0), None, None),
        ];
        let result = OpNumberMathNthRt::run(&mut inputs).await.unwrap();
        match &result.responses[0].value {
            Value::Decimal(v) => assert!((*v - 1.0).abs() < 1e-5),
            other => panic!("Expected Decimal, got {:?}", other),
        }
    }

    #[tokio::test]
    async fn test_nth_root_negative_input_clamped_to_zero() {
        // Implementation clamps negative inputs to 0.0 with num.max(0.0)
        let mut inputs = vec![
            Input::new("a".to_string(), Value::Decimal(-8.0), None, None),
            Input::new("n".to_string(), Value::Decimal(3.0), None, None),
        ];
        let result = OpNumberMathNthRt::run(&mut inputs).await.unwrap();
        match &result.responses[0].value {
            Value::Decimal(v) => assert!((*v).abs() < 1e-5, "Negative input clamped to 0, so result should be 0"),
            other => panic!("Expected Decimal, got {:?}", other),
        }
    }

    #[tokio::test]
    async fn test_nth_root_integer_input() {
        let mut inputs = vec![
            Input::new("a".to_string(), Value::Integer(27), None, None),
            Input::new("n".to_string(), Value::Decimal(3.0), None, None),
        ];
        let result = OpNumberMathNthRt::run(&mut inputs).await.unwrap();
        match &result.responses[0].value {
            Value::Decimal(v) => assert!((*v - 3.0).abs() < 1e-4),
            other => panic!("Expected Decimal, got {:?}", other),
        }
    }

    #[tokio::test]
    async fn test_nth_root_invalid_type_returns_error() {
        let mut inputs = vec![
            Input::new("a".to_string(), Value::Bool(true), None, None),
            Input::new("n".to_string(), Value::Decimal(2.0), None, None),
        ];
        let result = OpNumberMathNthRt::run(&mut inputs).await;
        assert!(result.is_err(), "Expected error for Bool input");
    }
}
