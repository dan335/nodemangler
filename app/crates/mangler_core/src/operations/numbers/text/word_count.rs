//! Word count operation.
//!
//! Counts whitespace-separated words in a text value.

use crate::input::Input;
use crate::node_settings::NodeSettings;
use crate::convert_inputs;
use crate::operations::{OperationResponse, OperationError, OutputResponse};
use crate::output::Output;
use crate::value::Value;
use serde::{Deserialize, Serialize};
use std::time::Instant;

/// A node that counts the words in a text value.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpNumberTextWordCount {}

impl OpNumberTextWordCount {
    /// Returns the node metadata for this operation.
    pub fn settings() -> NodeSettings {
        NodeSettings {
            name: "word count".to_string(),
            description: "Counts whitespace-separated words in text.".to_string(),
            help: "Splits the input on any run of Unicode whitespace and counts the resulting non-empty tokens, so leading, trailing, and repeated spaces do not inflate the count. An empty or all-whitespace string yields 0.\n\nThis is a plain whitespace tokenizer, not a linguistic one — hyphenated or punctuation-joined words count as one token. It lives under numbers because it produces a number.".to_string(),
        }
    }

    /// Creates the default input: a single text value.
    pub fn create_inputs() -> Vec<Input> {
        vec![
            Input::new("input".to_string(), Value::Text(String::new()), None, None)
                .with_description("Text whose words are counted."),
        ]
    }

    /// Creates the default output: a single integer.
    pub fn create_outputs() -> Vec<Output> {
        vec![
            Output::new("output".to_string(), Value::Integer(0), None)
                .with_description("Number of whitespace-separated words."),
        ]
    }

    /// Converts the input to text and counts its words.
    pub async fn run(inputs: &mut [Input]) -> Result<OperationResponse, OperationError> {
        let start_time = Instant::now();

        convert_inputs! { inputs;
            Text(text) = 0,
        }

        let count = text.split_whitespace().count() as i32;

        Ok(OperationResponse {
            time: Instant::now().duration_since(start_time),
            responses: vec![OutputResponse { value: Value::Integer(count) }],
        })
    }
}

#[cfg(test)]
#[path = "word_count_tests.rs"]
mod tests;
