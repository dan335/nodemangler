//! Text replace operation.
//!
//! Replaces every occurrence of a substring within a `Text` value with a
//! replacement string.

use crate::input::Input;
use crate::node_settings::NodeSettings;
use crate::convert_inputs;
use crate::operations::{OperationResponse, OperationError, OutputResponse};
use crate::output::Output;
use crate::value::Value;
use serde::{Deserialize, Serialize};
use std::time::Instant;

/// A node that replaces all occurrences of one substring with another.
///
/// Every non-overlapping match of `from` in `text` is replaced by `to`. The
/// output is always `Text`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpTextReplace {}

impl OpTextReplace {
    /// Returns the node metadata for this operation.
    pub fn settings() -> NodeSettings {
        NodeSettings {
            name: "replace".to_string(),
            description: "Replaces all occurrences of a substring.".to_string(),
            help: "Replaces every non-overlapping occurrence of from in text with to, scanning left to right. Matching is exact and case-sensitive; to is inserted verbatim, so an empty to deletes the matches.\n\nWhen from is empty the text is returned unchanged rather than inserting to between every character. All inputs are coerced to Text first.".to_string(),
        }
    }

    /// Creates the default inputs: the source text, the search substring, and
    /// the replacement.
    pub fn create_inputs() -> Vec<Input> {
        vec![
            Input::new("text".to_string(), Value::Text(String::new()), None, None)
                .with_description("Source text to search within."),
            Input::new("from".to_string(), Value::Text(String::new()), None, None)
                .with_description("Substring to find; if empty the text is left unchanged."),
            Input::new("to".to_string(), Value::Text(String::new()), None, None)
                .with_description("Replacement inserted in place of each match."),
        ]
    }

    /// Creates the default output: a single `Text` value.
    pub fn create_outputs() -> Vec<Output> {
        vec![
            Output::new("output".to_string(), Value::Text(String::new()), None)
                .with_description("The text with every occurrence of from replaced by to."),
        ]
    }

    /// Converts the inputs to `Text` and replaces every occurrence of `from`.
    pub async fn run(inputs: &mut [Input]) -> Result<OperationResponse, OperationError> {
        let start_time = Instant::now();

        convert_inputs! { inputs;
            Text(text) = 0,
            Text(from) = 1,
            Text(to) = 2,
        }

        let output = if from.is_empty() {
            text
        } else {
            text.replace(&from, &to)
        };

        Ok(OperationResponse {
            time: Instant::now().duration_since(start_time),
            responses: vec![OutputResponse {
                value: Value::Text(output),
            }],
        })
    }
}

#[cfg(test)]
#[path = "replace_tests.rs"]
mod tests;
