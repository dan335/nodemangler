//! URL decode operation.
//!
//! Decodes `%XX` percent-escapes back into text, following RFC 3986.

use crate::input::Input;
use crate::node_settings::NodeSettings;
use crate::operations::{OperationResponse, OperationError, OutputResponse, convert_input};
use crate::output::Output;
use crate::value::{Value, ValueType};
use serde::{Deserialize, Serialize};
use std::time::Instant;

/// A node that decodes percent-encoded (`%XX`) text.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpTextUrlDecode {}

impl OpTextUrlDecode {
    /// Returns the node metadata for this operation.
    pub fn settings() -> NodeSettings {
        NodeSettings {
            name: "url decode".to_string(),
            description: "Decodes percent-encoded (%XX) text.".to_string(),
            help: "Replaces each `%XX` escape with the byte it encodes and interprets the resulting bytes as text. Any character that is not part of a valid `%XX` sequence is passed through unchanged.\n\nFollowing RFC 3986, `+` is left literal rather than turned into a space (that is form-encoding, not URL decoding). Decoded bytes that are not valid UTF-8 are replaced (lossy decoding).".to_string(),
        }
    }

    /// Creates the default input: a single text value.
    pub fn create_inputs() -> Vec<Input> {
        vec![
            Input::new("input".to_string(), Value::Text(String::new()), None, None)
                .with_description("Percent-encoded text to decode."),
        ]
    }

    /// Creates the default output: a single text value.
    pub fn create_outputs() -> Vec<Output> {
        vec![
            Output::new("output".to_string(), Value::Text(String::new()), None)
                .with_description("Decoded (lossy-UTF8) text from the percent-encoded input."),
        ]
    }

    /// Converts the input to text and decodes its `%XX` escapes.
    pub async fn run(inputs: &mut [Input]) -> Result<OperationResponse, OperationError> {
        let start_time = Instant::now();
        let mut input_errors: Vec<(usize, String)> = vec![];

        let input_converted = convert_input(inputs, 0, ValueType::Text, &mut input_errors);

        if !input_errors.is_empty() { return Err(OperationError { input_errors, node_error: None }); }

        let Value::Text(input) = input_converted.unwrap() else { unreachable!() };

        let bytes = input.as_bytes();
        let mut out: Vec<u8> = Vec::new();
        let mut i = 0;
        while i < bytes.len() {
            if bytes[i] == b'%' && i + 3 <= bytes.len() {
                // Decode over the raw byte slice rather than `&input[..]`: if
                // '%' is immediately followed by (part of) a multibyte UTF-8
                // character, `i + 1`/`i + 3` can land mid-character and a
                // *string* slice there panics ("byte index is not a char
                // boundary"). `str::from_utf8` on the byte slice instead just
                // fails for a malformed/non-ASCII pair (two valid hex digits
                // are always plain ASCII, so this never rejects a real
                // escape), and we fall through to passing the '%' through.
                if let Ok(hex) = std::str::from_utf8(&bytes[i + 1..i + 3]) {
                    if let Ok(v) = u8::from_str_radix(hex, 16) {
                        out.push(v);
                        i += 3;
                        continue;
                    }
                }
            }
            out.push(bytes[i]);
            i += 1;
        }

        Ok(OperationResponse {
            time: Instant::now().duration_since(start_time),
            responses: vec![OutputResponse {
                value: Value::Text(String::from_utf8_lossy(&out).into_owned()),
            }],
        })
    }
}

#[cfg(test)]
#[path = "url_decode_tests.rs"]
mod tests;
