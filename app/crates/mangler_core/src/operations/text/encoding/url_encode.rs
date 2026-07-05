//! URL encode operation.
//!
//! Percent-encodes text for safe use in URLs, following RFC 3986.

use crate::input::Input;
use crate::node_settings::NodeSettings;
use crate::operations::{OperationResponse, OperationError, OutputResponse, convert_input};
use crate::output::Output;
use crate::value::{Value, ValueType};
use serde::{Deserialize, Serialize};
use std::time::Instant;

/// A node that percent-encodes text for URLs.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpTextUrlEncode {}

impl OpTextUrlEncode {
    /// Returns the node metadata for this operation.
    pub fn settings() -> NodeSettings {
        NodeSettings {
            name: "url encode".to_string(),
            description: "Percent-encodes text for safe use in URLs.".to_string(),
            help: "Percent-encodes the UTF-8 bytes of the input following RFC 3986. The unreserved characters A–Z a–z 0–9 and `- _ . ~` are kept as-is; every other byte is written as `%XX` with uppercase hex digits.\n\nSpaces become `%20` (not `+`), so the result is suitable for URL path and query components.".to_string(),
        }
    }

    /// Creates the default input: a single text value.
    pub fn create_inputs() -> Vec<Input> {
        vec![
            Input::new("input".to_string(), Value::Text(String::new()), None, None)
                .with_description("Text to percent-encode for use in a URL."),
        ]
    }

    /// Creates the default output: a single text value.
    pub fn create_outputs() -> Vec<Output> {
        vec![
            Output::new("output".to_string(), Value::Text(String::new()), None)
                .with_description("Percent-encoded form of the input."),
        ]
    }

    /// Converts the input to text and percent-encodes it.
    pub async fn run(inputs: &mut [Input]) -> Result<OperationResponse, OperationError> {
        let start_time = Instant::now();
        let mut input_errors: Vec<(usize, String)> = vec![];

        let input_converted = convert_input(inputs, 0, ValueType::Text, &mut input_errors);

        if !input_errors.is_empty() { return Err(OperationError { input_errors, node_error: None }); }

        let Value::Text(input) = input_converted.unwrap() else { unreachable!() };

        let mut out = String::new();
        for &b in input.as_bytes() {
            match b {
                b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => out.push(b as char),
                _ => out.push_str(&format!("%{:02X}", b)),
            }
        }

        Ok(OperationResponse {
            time: Instant::now().duration_since(start_time),
            responses: vec![OutputResponse {
                value: Value::Text(out),
            }],
        })
    }
}

#[cfg(test)]
#[path = "url_encode_tests.rs"]
mod tests;
