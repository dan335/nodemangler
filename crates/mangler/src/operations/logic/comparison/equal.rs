use crate::input::{Input, InputSettings};
use crate::node_settings::NodeSettings;
use crate::operations::{OperationResponse, OperationError, OutputResponse};
use crate::output::Output;
use crate::value::Value;
use serde::{Deserialize, Serialize};
use std::time::Instant;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpLogicCompareEqual {}

impl OpLogicCompareEqual {
    pub fn settings() -> NodeSettings {
        NodeSettings {
            name: "equal".to_string(),
            description: "Returns true if two values are equal.".to_string(),
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
            (Value::Integer(a), Value::Integer(b)) => Value::Bool(*a == *b),
            (Value::Decimal(a), Value::Decimal(b)) => Value::Bool(*a == *b),
            (Value::Integer(a), Value::Decimal(b)) => Value::Bool((*a as f32) == *b),
            (Value::Decimal(a), Value::Integer(b)) => Value::Bool(*a == (*b as f32)),
            (Value::Bool(a), Value::Bool(b)) => Value::Bool(*a == *b),
            (Value::String(a), Value::String(b)) => Value::Bool(*a == *b),
            _ => { return Err(OperationError {
                input_errors: vec![],
                node_error: Some("Unsupported types for equal comparison.".to_string()),
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
    async fn test_equal_integers_true() {
        let mut inputs = make_inputs(Value::Integer(5), Value::Integer(5));
        let result = OpLogicCompareEqual::run(&mut inputs).await.unwrap();
        assert!(matches!(result.responses[0].value, Value::Bool(true)));
    }

    #[tokio::test]
    async fn test_equal_integers_false() {
        let mut inputs = make_inputs(Value::Integer(5), Value::Integer(10));
        let result = OpLogicCompareEqual::run(&mut inputs).await.unwrap();
        assert!(matches!(result.responses[0].value, Value::Bool(false)));
    }

    #[tokio::test]
    async fn test_equal_decimals_true() {
        let mut inputs = make_inputs(Value::Decimal(3.14), Value::Decimal(3.14));
        let result = OpLogicCompareEqual::run(&mut inputs).await.unwrap();
        assert!(matches!(result.responses[0].value, Value::Bool(true)));
    }

    #[tokio::test]
    async fn test_equal_decimals_false() {
        let mut inputs = make_inputs(Value::Decimal(3.14), Value::Decimal(2.71));
        let result = OpLogicCompareEqual::run(&mut inputs).await.unwrap();
        assert!(matches!(result.responses[0].value, Value::Bool(false)));
    }

    #[tokio::test]
    async fn test_equal_mixed_int_decimal() {
        let mut inputs = make_inputs(Value::Integer(5), Value::Decimal(5.0));
        let result = OpLogicCompareEqual::run(&mut inputs).await.unwrap();
        assert!(matches!(result.responses[0].value, Value::Bool(true)));
    }

    #[tokio::test]
    async fn test_equal_bools() {
        let mut inputs = make_inputs(Value::Bool(true), Value::Bool(true));
        let result = OpLogicCompareEqual::run(&mut inputs).await.unwrap();
        assert!(matches!(result.responses[0].value, Value::Bool(true)));
    }

    #[tokio::test]
    async fn test_equal_strings() {
        let mut inputs = make_inputs(Value::String("hello".to_string()), Value::String("hello".to_string()));
        let result = OpLogicCompareEqual::run(&mut inputs).await.unwrap();
        assert!(matches!(result.responses[0].value, Value::Bool(true)));
    }

    #[tokio::test]
    async fn test_equal_strings_false() {
        let mut inputs = make_inputs(Value::String("hello".to_string()), Value::String("world".to_string()));
        let result = OpLogicCompareEqual::run(&mut inputs).await.unwrap();
        assert!(matches!(result.responses[0].value, Value::Bool(false)));
    }

    #[tokio::test]
    async fn test_equal_settings() {
        let s = OpLogicCompareEqual::settings();
        assert_eq!(s.name, "equal");
        assert_eq!(OpLogicCompareEqual::create_inputs().len(), 2);
        assert_eq!(OpLogicCompareEqual::create_outputs().len(), 1);
    }
}
