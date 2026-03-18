//! Text to-uppercase operation.
//!
//! Converts all characters in a `Text` value to their uppercase equivalents
//! using Unicode full case-folding rules.

use crate::input::Input;
use crate::node_settings::NodeSettings;
use crate::operations::{OperationResponse, OperationError, OutputResponse, convert_input};
use crate::output::Output;
use crate::value::{Value, ValueType};
use serde::{Deserialize, Serialize};
use std::time::Instant;

/// A node that converts a text value to uppercase.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpTextToUppercase {}

impl OpTextToUppercase {
    /// Returns the node metadata for this operation.
    pub fn settings() -> NodeSettings {
        NodeSettings {
            name: "to uppercase".to_string(),
            description: "Converts text to uppercase.".to_string(),
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

    /// Converts the input text to uppercase.
    pub async fn run(inputs: &mut [Input]) -> Result<OperationResponse, OperationError> {
        let start_time = Instant::now();
        let mut input_errors: Vec<(usize, String)> = vec![];

        let input_converted = convert_input(inputs, 0, ValueType::Text, &mut input_errors);

        if !input_errors.is_empty() { return Err(OperationError { input_errors, node_error: None }); }

        let Value::Text(text) = input_converted.unwrap() else { unreachable!() };

        Ok(OperationResponse {
            time: Instant::now().duration_since(start_time),
            responses: vec![OutputResponse {
                value: Value::Text(text.to_uppercase()),
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
    async fn test_to_uppercase_basic() {
        let mut inputs = vec![Input::new("input".to_string(), Value::Text("hello".to_string()), None, None)];
        let result = OpTextToUppercase::run(&mut inputs).await.unwrap();
        match &result.responses[0].value {
            Value::Text(v) => assert_eq!(v, "HELLO"),
            other => panic!("Expected Text(\"HELLO\"), got {:?}", other),
        }
    }

    #[tokio::test]
    async fn test_to_uppercase_already_upper() {
        let mut inputs = vec![Input::new("input".to_string(), Value::Text("WORLD".to_string()), None, None)];
        let result = OpTextToUppercase::run(&mut inputs).await.unwrap();
        match &result.responses[0].value {
            Value::Text(v) => assert_eq!(v, "WORLD"),
            other => panic!("Expected Text, got {:?}", other),
        }
    }

    #[tokio::test]
    async fn test_to_uppercase_settings() {
        let s = OpTextToUppercase::settings();
        assert_eq!(s.name, "to uppercase");
        assert_eq!(OpTextToUppercase::create_inputs().len(), 1);
        assert_eq!(OpTextToUppercase::create_outputs().len(), 1);
    }
}
