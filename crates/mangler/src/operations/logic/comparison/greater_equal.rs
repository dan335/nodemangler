use crate::input::{Input, InputSettings};
use crate::node_settings::NodeSettings;
use crate::operations::{OperationResponse, OperationError, OutputResponse};
use crate::output::Output;
use crate::value::Value;
use serde::{Deserialize, Serialize};
use std::time::Instant;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpLogicCompareGreaterEqual {}

impl OpLogicCompareGreaterEqual {
    pub fn settings() -> NodeSettings {
        NodeSettings {
            name: "greater equal".to_string(),
            description: "Returns true if a is greater than or equal to b.".to_string(),
        }
    }

    pub fn create_inputs() -> Vec<Input> {
        vec![
            Input::new("a".to_string(), Value::Decimal(0.0), Some(InputSettings::DragValue { speed: None, clamp: None }), None),
            Input::new("b".to_string(), Value::Decimal(0.0), Some(InputSettings::DragValue { speed: None, clamp: None }), None),
        ]
    }

    pub fn create_outputs() -> Vec<Output> {
        vec![
            Output::new("output".to_string(), Value::Bool(false), None)
        ]
    }

    pub async fn run(inputs: &mut Vec<Input>) -> Result<OperationResponse, OperationError> {
        let start_time = Instant::now();

        let value = match (&inputs[0].value, &inputs[1].value) {
            (Value::Integer(a), Value::Integer(b)) => Value::Bool(*a >= *b),
            (Value::Decimal(a), Value::Decimal(b)) => Value::Bool(*a >= *b),
            (Value::Integer(a), Value::Decimal(b)) => Value::Bool((*a as f32) >= *b),
            (Value::Decimal(a), Value::Integer(b)) => Value::Bool(*a >= (*b as f32)),
            _ => { return Err(OperationError {
                input_errors: vec![],
                node_error: Some("Unsupported types for greater equal comparison.".to_string()),
            }); }
        };

        Ok(OperationResponse {
            time: Instant::now().duration_since(start_time),
            responses: vec![OutputResponse { value }],
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::input::Input;
    use crate::value::Value;

    fn make_inputs(a: Value, b: Value) -> Vec<Input> {
        vec![
            Input::new("a".to_string(), a, None, None),
            Input::new("b".to_string(), b, None, None),
        ]
    }

    #[tokio::test]
    async fn test_greater_equal_greater() {
        let mut inputs = make_inputs(Value::Integer(10), Value::Integer(5));
        let result = OpLogicCompareGreaterEqual::run(&mut inputs).await.unwrap();
        assert!(matches!(result.responses[0].value, Value::Bool(true)));
    }

    #[tokio::test]
    async fn test_greater_equal_equal() {
        let mut inputs = make_inputs(Value::Integer(5), Value::Integer(5));
        let result = OpLogicCompareGreaterEqual::run(&mut inputs).await.unwrap();
        assert!(matches!(result.responses[0].value, Value::Bool(true)));
    }

    #[tokio::test]
    async fn test_greater_equal_less() {
        let mut inputs = make_inputs(Value::Integer(3), Value::Integer(5));
        let result = OpLogicCompareGreaterEqual::run(&mut inputs).await.unwrap();
        assert!(matches!(result.responses[0].value, Value::Bool(false)));
    }

    #[tokio::test]
    async fn test_greater_equal_decimals() {
        let mut inputs = make_inputs(Value::Decimal(5.0), Value::Decimal(5.0));
        let result = OpLogicCompareGreaterEqual::run(&mut inputs).await.unwrap();
        assert!(matches!(result.responses[0].value, Value::Bool(true)));
    }

    #[tokio::test]
    async fn test_greater_equal_settings() {
        let s = OpLogicCompareGreaterEqual::settings();
        assert_eq!(s.name, "greater equal");
    }
}
