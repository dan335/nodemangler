//! Text input operation.
//!
//! Provides a simple pass-through node that accepts a multi-line text body
//! (or a value convertible to `Text`) and outputs it. Useful as an entry
//! point for text data in the node graph.

use crate::input::{Input, InputSettings};
use crate::node_settings::NodeSettings;
use crate::operations::{OperationResponse, OperationError, OutputResponse, convert_input};
use crate::output::Output;
use crate::value::{Value, ValueType};
use serde::{Deserialize, Serialize};
use std::time::Instant;

/// A text input node that passes through a `Text` value.
///
/// Accepts any value convertible to `Text` (including `String`) and outputs
/// the converted text body.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpTextInput {}

impl OpTextInput {
    /// Returns the node metadata (name and description) for this operation.
    pub fn settings() -> NodeSettings {
        NodeSettings {
            name: "text".to_string(),
            description: "A text body input.".to_string(),
        }
    }

    /// Creates the default inputs: a single multi-line text input defaulting to an empty string.
    pub fn create_inputs() -> Vec<Input> {
        vec![
            Input::new("input".to_string(), Value::Text(String::new()), Some(InputSettings::MultiLineText), None),
        ]
    }

    /// Creates the default outputs: a single text output defaulting to an empty string.
    pub fn create_outputs() -> Vec<Output> {
        vec![
            Output::new("output".to_string(), Value::Text(String::new()), None),
        ]
    }

    /// Converts the input to a `Text` value and passes it through as the output.
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
    async fn test_text_input_passthrough() {
        let mut inputs = vec![Input::new("input".to_string(), Value::Text("hello".to_string()), None, None)];
        let result = OpTextInput::run(&mut inputs).await.unwrap();
        match &result.responses[0].value {
            Value::Text(v) => assert_eq!(v, "hello"),
            other => panic!("Expected Text, got {:?}", other),
        }
    }

    #[tokio::test]
    async fn test_text_input_empty() {
        let mut inputs = vec![Input::new("input".to_string(), Value::Text(String::new()), None, None)];
        let result = OpTextInput::run(&mut inputs).await.unwrap();
        match &result.responses[0].value {
            Value::Text(v) => assert_eq!(v, ""),
            other => panic!("Expected Text(\"\"), got {:?}", other),
        }
    }

    #[tokio::test]
    async fn test_text_input_passthrough_text() {
        // Text values pass through unchanged.
        let mut inputs = vec![Input::new("input".to_string(), Value::Text("from text".to_string()), None, None)];
        let result = OpTextInput::run(&mut inputs).await.unwrap();
        match &result.responses[0].value {
            Value::Text(v) => assert_eq!(v, "from text"),
            other => panic!("Expected Text, got {:?}", other),
        }
    }

    #[tokio::test]
    async fn test_text_multiline() {
        let body = "line one\nline two\nline three".to_string();
        let mut inputs = vec![Input::new("input".to_string(), Value::Text(body.clone()), None, None)];
        let result = OpTextInput::run(&mut inputs).await.unwrap();
        match &result.responses[0].value {
            Value::Text(v) => assert_eq!(v, &body),
            other => panic!("Expected Text, got {:?}", other),
        }
    }

    #[tokio::test]
    async fn test_text_settings() {
        let s = OpTextInput::settings();
        assert_eq!(s.name, "text");
        assert_eq!(OpTextInput::create_inputs().len(), 1);
        assert_eq!(OpTextInput::create_outputs().len(), 1);
    }
}
