//! Base64 encode operation.
//!
//! Encodes the UTF-8 bytes of a text value into a standard `=`-padded Base64
//! string using the shared codec in the parent module.

use crate::input::{Input, InputSettings};
use crate::node_settings::NodeSettings;
use crate::operations::{OperationResponse, OperationError, OutputResponse, convert_input};
use crate::output::Output;
use crate::value::{Value, ValueType};
use serde::{Deserialize, Serialize};
use std::time::Instant;

/// A node that Base64-encodes a text value's UTF-8 bytes.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpTextBase64Encode {}

impl OpTextBase64Encode {
    /// Returns the node metadata for this operation.
    pub fn settings() -> NodeSettings {
        NodeSettings {
            name: "base64 encode".to_string(),
            description: "Encodes text as a standard Base64 string.".to_string(),
            help: "Takes the UTF-8 bytes of the input text and encodes them as standard RFC 4648 Base64 with `=` padding (alphabet A–Z a–z 0–9 + /). The output is always ASCII.\n\nUse it to embed arbitrary text in a URL-safe-ish, transport-safe form, or to pair with `base64 decode` for round-tripping. For encoding an image, use the `data uri` node instead.".to_string(),
        }
    }

    /// Creates the default input: a single text value.
    pub fn create_inputs() -> Vec<Input> {
        vec![
            Input::new("input".to_string(), Value::Text(String::new()), Some(InputSettings::MultiLineText), None)
                .with_description("Text whose UTF-8 bytes are Base64-encoded."),
        ]
    }

    /// Creates the default output: a single text value.
    pub fn create_outputs() -> Vec<Output> {
        vec![
            Output::new("output".to_string(), Value::Text(String::new()), None)
                .with_description("Base64-encoded representation of the input."),
        ]
    }

    /// Converts the input to text and Base64-encodes it.
    pub async fn run(inputs: &mut [Input]) -> Result<OperationResponse, OperationError> {
        let start_time = Instant::now();
        let mut input_errors: Vec<(usize, String)> = vec![];

        let input_converted = convert_input(inputs, 0, ValueType::Text, &mut input_errors);

        if !input_errors.is_empty() { return Err(OperationError { input_errors, node_error: None }); }

        let Value::Text(text) = input_converted.unwrap() else { unreachable!() };

        let encoded = super::base64_encode(text.as_bytes());

        Ok(OperationResponse {
            time: Instant::now().duration_since(start_time),
            responses: vec![OutputResponse { value: Value::Text(encoded) }],
        })
    }
}

#[cfg(test)]
#[path = "base64_encode_tests.rs"]
mod tests;
