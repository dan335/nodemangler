//! Text split operation.
//!
//! Splits a `Text` value on a delimiter and returns one selected field plus the
//! total field count.

use crate::input::{Input, InputSettings};
use crate::node_settings::NodeSettings;
use crate::convert_inputs;
use crate::operations::{OperationResponse, OperationError, OutputResponse};
use crate::output::Output;
use crate::value::Value;
use serde::{Deserialize, Serialize};
use std::time::Instant;

/// A node that splits text on a delimiter and selects one field.
///
/// Outputs the field at the requested index (empty if out of range) and the
/// total number of fields produced by the split.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpTextSplit {}

impl OpTextSplit {
    /// Returns the node metadata for this operation.
    pub fn settings() -> NodeSettings {
        NodeSettings {
            name: "split".to_string(),
            description: "Splits text on a delimiter and selects a field.".to_string(),
            help: "Splits text on every occurrence of the delimiter and returns the field at index (0-based) along with the total field count. An index outside the range yields an empty field.\n\nWhen the delimiter is empty the whole text is treated as a single field. The count output is useful for iterating over or validating the number of parts.".to_string(),
        }
    }

    /// Creates the default inputs: the source text, the delimiter, and the field
    /// index.
    pub fn create_inputs() -> Vec<Input> {
        vec![
            Input::new("text".to_string(), Value::Text(String::new()), None, None)
                .with_description("Source text to split."),
            Input::new("delimiter".to_string(), Value::Text(",".to_string()), None, None)
                .with_description("Separator to split on; empty means treat the text as one field."),
            Input::new("index".to_string(), Value::Integer(0), Some(InputSettings::DragValue { clamp: Some((0.0, 100000.0)), speed: None }), None)
                .with_description("0-based field to output; out of range yields an empty field."),
        ]
    }

    /// Creates the default outputs: the selected field and the total field count.
    pub fn create_outputs() -> Vec<Output> {
        vec![
            Output::new("field".to_string(), Value::Text(String::new()), None)
                .with_description("The field at the requested index (empty if out of range)."),
            Output::new("count".to_string(), Value::Integer(0), None)
                .with_description("Total number of fields produced by the split."),
        ]
    }

    /// Converts the inputs, splits the text, and returns the selected field and
    /// count.
    pub async fn run(inputs: &mut [Input]) -> Result<OperationResponse, OperationError> {
        let start_time = Instant::now();

        convert_inputs! { inputs;
            Text(text) = 0,
            Text(delimiter) = 1,
            Integer(index) = 2,
        }

        let parts: Vec<&str> = if delimiter.is_empty() {
            vec![text.as_str()]
        } else {
            text.split(delimiter.as_str()).collect()
        };
        let count = parts.len() as i32;
        let field = parts.get(index.max(0) as usize).copied().unwrap_or("").to_string();

        Ok(OperationResponse {
            time: Instant::now().duration_since(start_time),
            responses: vec![
                OutputResponse { value: Value::Text(field) },
                OutputResponse { value: Value::Integer(count) },
            ],
        })
    }
}

#[cfg(test)]
#[path = "split_tests.rs"]
mod tests;
