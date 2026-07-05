//! Base64 decode operation.
//!
//! Decodes a standard `=`-padded Base64 string back into text using the shared
//! codec in the parent module.

use crate::input::{Input, InputSettings};
use crate::node_settings::NodeSettings;
use crate::operations::{OperationResponse, OperationError, OutputResponse, convert_input};
use crate::output::Output;
use crate::value::{Value, ValueType};
use serde::{Deserialize, Serialize};
use std::time::Instant;

/// A node that decodes a Base64 string back into text.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpTextBase64Decode {}

impl OpTextBase64Decode {
    /// Returns the node metadata for this operation.
    pub fn settings() -> NodeSettings {
        NodeSettings {
            name: "base64 decode".to_string(),
            description: "Decodes a standard Base64 string back into text.".to_string(),
            help: "Decodes standard RFC 4648 Base64 (alphabet A–Z a–z 0–9 + /) back into bytes and interprets them as text. Whitespace and `=` padding are ignored. Input that contains characters outside the alphabet is rejected with an error.\n\nDecoded bytes that are not valid UTF-8 are replaced with the Unicode replacement character (lossy decoding).".to_string(),
        }
    }

    /// Creates the default input: a single text value.
    pub fn create_inputs() -> Vec<Input> {
        vec![
            Input::new("input".to_string(), Value::Text(String::new()), Some(InputSettings::MultiLineText), None)
                .with_description("Base64 text to decode."),
        ]
    }

    /// Creates the default output: a single text value.
    pub fn create_outputs() -> Vec<Output> {
        vec![
            Output::new("output".to_string(), Value::Text(String::new()), None)
                .with_description("Decoded (lossy-UTF8) text from the Base64 input."),
        ]
    }

    /// Converts the input to text and Base64-decodes it.
    pub async fn run(inputs: &mut [Input]) -> Result<OperationResponse, OperationError> {
        let start_time = Instant::now();
        let mut input_errors: Vec<(usize, String)> = vec![];

        let input_converted = convert_input(inputs, 0, ValueType::Text, &mut input_errors);

        if !input_errors.is_empty() { return Err(OperationError { input_errors, node_error: None }); }

        let Value::Text(input) = input_converted.unwrap() else { unreachable!() };

        let value = match super::base64_decode(&input) {
            Some(bytes) => Value::Text(String::from_utf8_lossy(&bytes).into_owned()),
            None => return Err(OperationError { input_errors: vec![(0, "Input is not valid Base64.".to_string())], node_error: None }),
        };

        Ok(OperationResponse {
            time: Instant::now().duration_since(start_time),
            responses: vec![OutputResponse { value }],
        })
    }
}

#[cfg(test)]
#[path = "base64_decode_tests.rs"]
mod tests;
