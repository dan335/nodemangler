use crate::input::{Input, InputSettings};
use crate::node_settings::NodeSettings;
use crate::operations::{OperationResponse, OperationError, OutputResponse};
use crate::output::Output;
use crate::value::{Value, ValueType};
use serde::{Deserialize, Serialize};
use std::time::Instant;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpNumberCastToInteger {}

impl OpNumberCastToInteger {
    pub fn settings() -> NodeSettings {
        NodeSettings {
            name: "to integer".to_string(),
            description: "Converts a number to an integer.".to_string(),
        }
    }

    pub fn create_inputs() -> Vec<Input> {
        vec![
            Input::new("a".to_string(), Value::Integer(i32::default()), Some(InputSettings::DragValue { speed:None, clamp:None }), None),
        ]
    }

    pub fn create_outputs() -> Vec<Output> {
        vec![
            Output::new("output".to_string(), Value::Integer(i32::default()), None)
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

        let Ok(Value::Integer(n)) = inputs[0].value.try_convert_to(ValueType::Integer) else { return Err(OperationError { input_errors: vec![(0, "Unable to convert to integer.".to_string())], node_error: None })};

        Ok(OperationResponse {
            time: Instant::now().duration_since(start_time),
            responses: vec![OutputResponse {
                value: Value::Integer(n),
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
    async fn test_to_integer_settings() {
        let s = OpNumberCastToInteger::settings();
        assert_eq!(s.name, "to integer");
        assert_eq!(OpNumberCastToInteger::create_inputs().len(), 1);
        assert_eq!(OpNumberCastToInteger::create_outputs().len(), 1);
    }

    #[tokio::test]
    async fn test_to_integer_from_decimal() {
        let mut inputs = vec![Input::new("input".to_string(), Value::Decimal(3.7), None, None)];
        let result = OpNumberCastToInteger::run(&mut inputs).await.unwrap();
        match &result.responses[0].value {
            Value::Integer(v) => assert_eq!(*v, 3),
            other => panic!("Expected Integer, got {:?}", other),
        }
    }

    #[tokio::test]
    async fn test_to_integer_passthrough() {
        let mut inputs = vec![Input::new("input".to_string(), Value::Integer(42), None, None)];
        let result = OpNumberCastToInteger::run(&mut inputs).await.unwrap();
        match &result.responses[0].value {
            Value::Integer(v) => assert_eq!(*v, 42),
            other => panic!("Expected Integer, got {:?}", other),
        }
    }

    #[tokio::test]
    async fn test_to_integer_truncates_decimal() {
        // try_convert_to Integer from Decimal truncates
        let mut inputs = vec![Input::new("a".to_string(), Value::Decimal(3.9), None, None)];
        let result = OpNumberCastToInteger::run(&mut inputs).await.unwrap();
        match &result.responses[0].value {
            Value::Integer(v) => assert_eq!(*v, 3),
            other => panic!("Expected Integer, got {:?}", other),
        }
    }

    #[tokio::test]
    async fn test_to_integer_from_negative_decimal() {
        let mut inputs = vec![Input::new("a".to_string(), Value::Decimal(-3.9), None, None)];
        let result = OpNumberCastToInteger::run(&mut inputs).await.unwrap();
        match &result.responses[0].value {
            Value::Integer(v) => assert_eq!(*v, -3),
            other => panic!("Expected Integer, got {:?}", other),
        }
    }

    #[tokio::test]
    async fn test_to_integer_zero() {
        let mut inputs = vec![Input::new("a".to_string(), Value::Decimal(0.0), None, None)];
        let result = OpNumberCastToInteger::run(&mut inputs).await.unwrap();
        match &result.responses[0].value {
            Value::Integer(v) => assert_eq!(*v, 0),
            other => panic!("Expected Integer, got {:?}", other),
        }
    }

    #[tokio::test]
    async fn test_to_integer_negative_integer_passthrough() {
        let mut inputs = vec![Input::new("a".to_string(), Value::Integer(-100), None, None)];
        let result = OpNumberCastToInteger::run(&mut inputs).await.unwrap();
        match &result.responses[0].value {
            Value::Integer(v) => assert_eq!(*v, -100),
            other => panic!("Expected Integer, got {:?}", other),
        }
    }

    #[tokio::test]
    async fn test_to_integer_exactly_integer_decimal() {
        let mut inputs = vec![Input::new("a".to_string(), Value::Decimal(5.0), None, None)];
        let result = OpNumberCastToInteger::run(&mut inputs).await.unwrap();
        match &result.responses[0].value {
            Value::Integer(v) => assert_eq!(*v, 5),
            other => panic!("Expected Integer, got {:?}", other),
        }
    }
}
