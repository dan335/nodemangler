use crate::input::Input;
use crate::node_settings::NodeSettings;
use crate::operations::{OperationResponse, OperationError, OutputResponse, convert_input};
use crate::output::Output;
use crate::value::{Value, ValueType};
use serde::{Deserialize, Serialize};
use std::time::Instant;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpLogicInputBool {}

impl OpLogicInputBool {
    pub fn settings() -> NodeSettings {
        NodeSettings {
            name: "bool".to_string(),
            description: "A boolean input.".to_string(),
        }
    }

    pub fn create_inputs() -> Vec<Input> {
        vec![
            Input::new("input".to_string(), Value::Bool(false), None, None)
        ]
    }

    pub fn create_outputs() -> Vec<Output> {
        vec![
            Output::new("output".to_string(), Value::Bool(false), None)
        ]
    }

    pub async fn run(inputs: &mut Vec<Input>) -> Result<OperationResponse, OperationError> {
        let start_time = Instant::now();
        let mut input_errors: Vec<(usize, String)> = vec![];

        let input_converted = convert_input(inputs, 0, ValueType::Bool, &mut input_errors);

        if input_errors.len() > 0 { return Err(OperationError { input_errors, node_error: None }); }

        let Value::Bool(input) = input_converted.unwrap() else { unreachable!() };

        Ok(OperationResponse {
            time: Instant::now().duration_since(start_time),
            responses: vec![OutputResponse {
                value: Value::Bool(input),
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
    async fn test_bool_input_true() {
        let mut inputs = vec![Input::new("input".to_string(), Value::Bool(true), None, None)];
        let result = OpLogicInputBool::run(&mut inputs).await.unwrap();
        match &result.responses[0].value {
            Value::Bool(v) => assert_eq!(*v, true),
            other => panic!("Expected Bool(true), got {:?}", other),
        }
    }

    #[tokio::test]
    async fn test_bool_input_false() {
        let mut inputs = vec![Input::new("input".to_string(), Value::Bool(false), None, None)];
        let result = OpLogicInputBool::run(&mut inputs).await.unwrap();
        match &result.responses[0].value {
            Value::Bool(v) => assert_eq!(*v, false),
            other => panic!("Expected Bool(false), got {:?}", other),
        }
    }

    #[tokio::test]
    async fn test_bool_input_from_integer_nonzero() {
        let mut inputs = vec![Input::new("input".to_string(), Value::Integer(1), None, None)];
        let result = OpLogicInputBool::run(&mut inputs).await.unwrap();
        match &result.responses[0].value {
            Value::Bool(v) => assert_eq!(*v, true),
            other => panic!("Expected Bool(true), got {:?}", other),
        }
    }

    #[tokio::test]
    async fn test_bool_input_from_integer_zero() {
        let mut inputs = vec![Input::new("input".to_string(), Value::Integer(0), None, None)];
        let result = OpLogicInputBool::run(&mut inputs).await.unwrap();
        match &result.responses[0].value {
            Value::Bool(v) => assert_eq!(*v, false),
            other => panic!("Expected Bool(false), got {:?}", other),
        }
    }

    #[tokio::test]
    async fn test_bool_settings() {
        let s = OpLogicInputBool::settings();
        assert_eq!(s.name, "bool");
        assert_eq!(OpLogicInputBool::create_inputs().len(), 1);
        assert_eq!(OpLogicInputBool::create_outputs().len(), 1);
    }
}
