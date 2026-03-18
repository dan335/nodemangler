//! Text pass-through operation (kept for graph file compatibility).
//!
//! This node was previously a `Text` → `String` cast. Now that `String` and `Text`
//! have been merged into a single `Text` type it is a no-op pass-through.
//! It is retained in the `Operation` enum so that saved graphs deserialise correctly,
//! but it no longer appears in the node menu.

use crate::input::Input;
use crate::node_settings::NodeSettings;
use crate::operations::{OperationResponse, OperationError, OutputResponse, convert_input};
use crate::output::Output;
use crate::value::{Value, ValueType};
use serde::{Deserialize, Serialize};
use std::time::Instant;

/// A no-op pass-through node kept only for saved-graph deserialisation compatibility.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpTextToString {}

impl OpTextToString {
    /// Returns the node metadata for this operation.
    pub fn settings() -> NodeSettings {
        NodeSettings {
            name: "to string".to_string(),
            description: "Casts a text value to a string.".to_string(),
        }
    }

    /// Creates the default inputs: a single empty `Text` input.
    pub fn create_inputs() -> Vec<Input> {
        vec![
            Input::new("input".to_string(), Value::Text(String::new()), None, None),
        ]
    }

    /// Creates the default output: a single `Text` value.
    pub fn create_outputs() -> Vec<Output> {
        vec![
            Output::new("output".to_string(), Value::Text(String::new()), None),
        ]
    }

    /// Passes the input `Text` through unchanged.
    pub async fn run(inputs: &mut [Input]) -> Result<OperationResponse, OperationError> {
        let start_time = Instant::now();
        let mut input_errors: Vec<(usize, String)> = vec![];

        let input_converted = convert_input(inputs, 0, ValueType::Text, &mut input_errors);

        if !input_errors.is_empty() { return Err(OperationError { input_errors, node_error: None }); }

        let Value::Text(text) = input_converted.unwrap() else { unreachable!() };

        Ok(OperationResponse {
            time: Instant::now().duration_since(start_time),
            responses: vec![OutputResponse {
                value: Value::Text(text),
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
    async fn test_to_string_basic() {
        let mut inputs = vec![Input::new("input".to_string(), Value::Text("hello".to_string()), None, None)];
        let result = OpTextToString::run(&mut inputs).await.unwrap();
        match &result.responses[0].value {
            Value::Text(v) => assert_eq!(v, "hello"),
            other => panic!("Expected Text(\"hello\"), got {:?}", other),
        }
    }

    #[tokio::test]
    async fn test_to_string_empty() {
        let mut inputs = vec![Input::new("input".to_string(), Value::Text(String::new()), None, None)];
        let result = OpTextToString::run(&mut inputs).await.unwrap();
        match &result.responses[0].value {
            Value::Text(v) => assert_eq!(v, ""),
            other => panic!("Expected Text, got {:?}", other),
        }
    }

    #[tokio::test]
    async fn test_to_string_settings() {
        let s = OpTextToString::settings();
        assert_eq!(s.name, "to string");
        assert_eq!(OpTextToString::create_inputs().len(), 1);
        assert_eq!(OpTextToString::create_outputs().len(), 1);
    }
}
