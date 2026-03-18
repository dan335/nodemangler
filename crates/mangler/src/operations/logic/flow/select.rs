use crate::input::{Input, InputSettings};
use crate::node_settings::NodeSettings;
use crate::operations::{OperationResponse, OperationError, OutputResponse, convert_input};
use crate::output::Output;
use crate::value::{Value, ValueType};
use serde::{Deserialize, Serialize};
use std::time::Instant;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpLogicFlowSelect {}

impl OpLogicFlowSelect {
    pub fn settings() -> NodeSettings {
        NodeSettings {
            name: "select".to_string(),
            description: "Selects between two values based on a condition.".to_string(),
        }
    }

    pub fn create_inputs() -> Vec<Input> {
        vec![
            Input::new("condition".to_string(), Value::Bool(false), None, None),
            Input::new("if true".to_string(), Value::Decimal(1.0), Some(InputSettings::DragValue { speed: None, clamp: None }), None),
            Input::new("if false".to_string(), Value::Decimal(0.0), Some(InputSettings::DragValue { speed: None, clamp: None }), None),
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

        let condition_converted = convert_input(inputs, 0, ValueType::Bool, &mut input_errors);

        if input_errors.len() > 0 { return Err(OperationError { input_errors, node_error: None }); }

        let Value::Bool(condition) = condition_converted.unwrap() else { unreachable!() };

        let value = if condition {
            inputs[1].value.clone()
        } else {
            inputs[2].value.clone()
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

    fn make_inputs(condition: Value, if_true: Value, if_false: Value) -> Vec<Input> {
        vec![
            Input::new("condition".to_string(), condition, None, None),
            Input::new("if true".to_string(), if_true, None, None),
            Input::new("if false".to_string(), if_false, None, None),
        ]
    }

    #[tokio::test]
    async fn test_select_true() {
        let mut inputs = make_inputs(Value::Bool(true), Value::Decimal(10.0), Value::Decimal(20.0));
        let result = OpLogicFlowSelect::run(&mut inputs).await.unwrap();
        match &result.responses[0].value {
            Value::Decimal(v) => assert!((*v - 10.0).abs() < 1e-6),
            other => panic!("Expected Decimal(10.0), got {:?}", other),
        }
    }

    #[tokio::test]
    async fn test_select_false() {
        let mut inputs = make_inputs(Value::Bool(false), Value::Decimal(10.0), Value::Decimal(20.0));
        let result = OpLogicFlowSelect::run(&mut inputs).await.unwrap();
        match &result.responses[0].value {
            Value::Decimal(v) => assert!((*v - 20.0).abs() < 1e-6),
            other => panic!("Expected Decimal(20.0), got {:?}", other),
        }
    }

    #[tokio::test]
    async fn test_select_integers() {
        let mut inputs = make_inputs(Value::Bool(true), Value::Integer(42), Value::Integer(0));
        let result = OpLogicFlowSelect::run(&mut inputs).await.unwrap();
        match &result.responses[0].value {
            Value::Integer(v) => assert_eq!(*v, 42),
            other => panic!("Expected Integer(42), got {:?}", other),
        }
    }

    #[tokio::test]
    async fn test_select_strings() {
        let mut inputs = make_inputs(
            Value::Bool(false),
            Value::String("yes".to_string()),
            Value::String("no".to_string()),
        );
        let result = OpLogicFlowSelect::run(&mut inputs).await.unwrap();
        match &result.responses[0].value {
            Value::String(v) => assert_eq!(v, "no"),
            other => panic!("Expected String(\"no\"), got {:?}", other),
        }
    }

    #[tokio::test]
    async fn test_select_bools() {
        let mut inputs = make_inputs(Value::Bool(true), Value::Bool(true), Value::Bool(false));
        let result = OpLogicFlowSelect::run(&mut inputs).await.unwrap();
        assert!(matches!(result.responses[0].value, Value::Bool(true)));
    }

    #[tokio::test]
    async fn test_select_condition_from_integer() {
        let mut inputs = make_inputs(Value::Integer(1), Value::Decimal(10.0), Value::Decimal(20.0));
        let result = OpLogicFlowSelect::run(&mut inputs).await.unwrap();
        match &result.responses[0].value {
            Value::Decimal(v) => assert!((*v - 10.0).abs() < 1e-6),
            other => panic!("Expected Decimal(10.0), got {:?}", other),
        }
    }

    #[tokio::test]
    async fn test_select_settings() {
        let s = OpLogicFlowSelect::settings();
        assert_eq!(s.name, "select");
        assert_eq!(OpLogicFlowSelect::create_inputs().len(), 3);
        assert_eq!(OpLogicFlowSelect::create_outputs().len(), 1);
    }
}
