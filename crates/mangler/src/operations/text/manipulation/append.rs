//! Text append operation.
//!
//! Concatenates two `Text` (or `String`) values into a single `Text` output.

use crate::input::Input;
use crate::node_settings::NodeSettings;
use crate::operations::{OperationResponse, OperationError, OutputResponse, convert_input};
use crate::output::Output;
use crate::value::{Value, ValueType};
use serde::{Deserialize, Serialize};
use std::time::Instant;

/// A node that concatenates two text values.
///
/// Both `a` and `b` inputs accept `Text` or `String` values. The output
/// is always `Text`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpTextAppend {}

impl OpTextAppend {
    /// Returns the node metadata for this operation.
    pub fn settings() -> NodeSettings {
        NodeSettings {
            name: "append".to_string(),
            description: "Concatenates two text values.".to_string(),
        }
    }

    /// Creates the default inputs: `a` and `b`, both empty `Text` values.
    pub fn create_inputs() -> Vec<Input> {
        vec![
            Input::new("a".to_string(), Value::Text(String::new()), None, None),
            Input::new("b".to_string(), Value::Text(String::new()), None, None),
        ]
    }

    /// Creates the default output: a single `Text` value.
    pub fn create_outputs() -> Vec<Output> {
        vec![
            Output::new("output".to_string(), Value::Text(String::new()), None),
        ]
    }

    /// Converts both inputs to `Text` and returns their concatenation.
    pub async fn run(inputs: &mut [Input]) -> Result<OperationResponse, OperationError> {
        let start_time = Instant::now();
        let mut input_errors: Vec<(usize, String)> = vec![];

        let a_converted = convert_input(inputs, 0, ValueType::Text, &mut input_errors);
        let b_converted = convert_input(inputs, 1, ValueType::Text, &mut input_errors);

        if !input_errors.is_empty() { return Err(OperationError { input_errors, node_error: None }); }

        let Value::Text(a) = a_converted.unwrap() else { unreachable!() };
        let Value::Text(b) = b_converted.unwrap() else { unreachable!() };

        Ok(OperationResponse {
            time: Instant::now().duration_since(start_time),
            responses: vec![OutputResponse {
                value: Value::Text(format!("{}{}", a, b)),
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
    async fn test_append_basic() {
        let mut inputs = vec![
            Input::new("a".to_string(), Value::Text("hello ".to_string()), None, None),
            Input::new("b".to_string(), Value::Text("world".to_string()), None, None),
        ];
        let result = OpTextAppend::run(&mut inputs).await.unwrap();
        match &result.responses[0].value {
            Value::Text(v) => assert_eq!(v, "hello world"),
            other => panic!("Expected Text, got {:?}", other),
        }
    }

    #[tokio::test]
    async fn test_append_empty() {
        let mut inputs = vec![
            Input::new("a".to_string(), Value::Text(String::new()), None, None),
            Input::new("b".to_string(), Value::Text(String::new()), None, None),
        ];
        let result = OpTextAppend::run(&mut inputs).await.unwrap();
        match &result.responses[0].value {
            Value::Text(v) => assert_eq!(v, ""),
            other => panic!("Expected Text, got {:?}", other),
        }
    }

    #[tokio::test]
    async fn test_append_settings() {
        let s = OpTextAppend::settings();
        assert_eq!(s.name, "append");
        assert_eq!(OpTextAppend::create_inputs().len(), 2);
        assert_eq!(OpTextAppend::create_outputs().len(), 1);
    }
}
