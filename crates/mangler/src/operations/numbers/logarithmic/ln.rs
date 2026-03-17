use crate::input::{Input, InputSettings};
use crate::node_settings::NodeSettings;
use crate::operations::{OperationResponse, OperationError, OutputResponse, convert_input};
use crate::output::Output;
use crate::value::{Value, ValueType};
use serde::{Deserialize, Serialize};
use std::time::Instant;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpNumberMathLn {}

impl OpNumberMathLn {
    pub fn settings() -> NodeSettings {
        NodeSettings {
            name: "ln".to_string(),
            description: "Computes the natural logarithm.".to_string(),
        }
    }

    pub fn create_inputs() -> Vec<Input> {
        vec![
            Input::new("input".to_string(), Value::Decimal(2.718), Some(InputSettings::DragValue { speed: None, clamp: None }), None),
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

        let input_converted = convert_input(inputs, 0, ValueType::Decimal, &mut input_errors);

        if input_errors.len() > 0 { return Err(OperationError { input_errors, node_error: None }); }

        let Value::Decimal(input) = input_converted.unwrap() else { unreachable!() };

        if input <= 0.0 {
            return Err(OperationError { input_errors: vec![], node_error: Some("Input must be greater than 0.".to_string()) });
        }

        let result = (input as f64).ln() as f32;

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
    async fn test_ln_settings() {
        let s = OpNumberMathLn::settings();
        assert_eq!(s.name, "ln");
        assert_eq!(OpNumberMathLn::create_inputs().len(), 1);
        assert_eq!(OpNumberMathLn::create_outputs().len(), 1);
    }

    #[tokio::test]
    async fn test_ln_e() {
        let mut inputs = vec![Input::new("input".to_string(), Value::Decimal(std::f32::consts::E), None, None)];
        let result = OpNumberMathLn::run(&mut inputs).await.unwrap();
        match &result.responses[0].value {
            Value::Decimal(v) => assert!((*v - 1.0).abs() < 0.01),
            other => panic!("Expected Decimal, got {:?}", other),
        }
    }

    #[tokio::test]
    async fn test_ln_1() {
        let mut inputs = vec![Input::new("input".to_string(), Value::Decimal(1.0), None, None)];
        let result = OpNumberMathLn::run(&mut inputs).await.unwrap();
        match &result.responses[0].value {
            Value::Decimal(v) => assert!((*v).abs() < 0.01),
            other => panic!("Expected Decimal, got {:?}", other),
        }
    }

    #[tokio::test]
    async fn test_ln_invalid() {
        let mut inputs = vec![Input::new("input".to_string(), Value::Decimal(-1.0), None, None)];
        let result = OpNumberMathLn::run(&mut inputs).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_ln_zero_errors() {
        let mut inputs = vec![Input::new("input".to_string(), Value::Decimal(0.0), None, None)];
        let result = OpNumberMathLn::run(&mut inputs).await;
        assert!(result.is_err(), "Expected error for ln(0)");
    }

    #[tokio::test]
    async fn test_ln_2() {
        // ln(2) ≈ 0.6931
        let mut inputs = vec![Input::new("input".to_string(), Value::Decimal(2.0), None, None)];
        let result = OpNumberMathLn::run(&mut inputs).await.unwrap();
        match &result.responses[0].value {
            Value::Decimal(v) => assert!((*v - 0.6931).abs() < 1e-3),
            other => panic!("Expected Decimal, got {:?}", other),
        }
    }

    #[tokio::test]
    async fn test_ln_10() {
        // ln(10) ≈ 2.302585
        let mut inputs = vec![Input::new("input".to_string(), Value::Decimal(10.0), None, None)];
        let result = OpNumberMathLn::run(&mut inputs).await.unwrap();
        match &result.responses[0].value {
            Value::Decimal(v) => assert!((*v - 2.302585).abs() < 1e-4),
            other => panic!("Expected Decimal, got {:?}", other),
        }
    }

    #[tokio::test]
    async fn test_ln_from_integer() {
        let mut inputs = vec![Input::new("input".to_string(), Value::Integer(1), None, None)];
        let result = OpNumberMathLn::run(&mut inputs).await.unwrap();
        match &result.responses[0].value {
            Value::Decimal(v) => assert!((*v).abs() < 1e-5),
            other => panic!("Expected Decimal, got {:?}", other),
        }
    }

    #[tokio::test]
    async fn test_ln_small_positive() {
        // ln(0.001) should be large negative
        let mut inputs = vec![Input::new("input".to_string(), Value::Decimal(0.001), None, None)];
        let result = OpNumberMathLn::run(&mut inputs).await.unwrap();
        match &result.responses[0].value {
            Value::Decimal(v) => assert!(*v < 0.0),
            other => panic!("Expected Decimal, got {:?}", other),
        }
    }
}
