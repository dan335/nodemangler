//! Text template operation.
//!
//! Substitutes up to three text values into `{}` placeholders in a template
//! string, in order.

use crate::input::{Input, InputSettings};
use crate::node_settings::NodeSettings;
use crate::convert_inputs;
use crate::operations::{OperationResponse, OperationError, OutputResponse};
use crate::output::Output;
use crate::value::Value;
use serde::{Deserialize, Serialize};
use std::time::Instant;

/// A node that fills `{}` placeholders in a template with up to three values.
///
/// The `template` input holds the format string; `a`, `b`, and `c` are
/// substituted into successive `{}` placeholders in order. The output is
/// always `Text`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpTextTemplate {}

impl OpTextTemplate {
    /// Returns the node metadata for this operation.
    pub fn settings() -> NodeSettings {
        NodeSettings {
            name: "template".to_string(),
            description: "Substitutes values into a template's placeholders.".to_string(),
            help: "Replaces each `{}` placeholder in the template with the next of a, b, c, in order. The first `{}` gets a, the second gets b, the third gets c. Any leftover `{}` (with no value to fill them) are left in place, and any extra values beyond the available placeholders are ignored.".to_string(),
        }
    }

    /// Creates the default inputs: the template and three substitution values.
    pub fn create_inputs() -> Vec<Input> {
        vec![
            Input::new("template".to_string(), Value::Text("{} {}".to_string()), Some(InputSettings::MultiLineText), None)
                .with_description("Format string; each `{}` is replaced by the next value."),
            Input::new("a".to_string(), Value::Text(String::new()), None, None)
                .with_description("Value substituted into the first `{}` placeholder."),
            Input::new("b".to_string(), Value::Text(String::new()), None, None)
                .with_description("Value substituted into the second `{}` placeholder."),
            Input::new("c".to_string(), Value::Text(String::new()), None, None)
                .with_description("Value substituted into the third `{}` placeholder."),
        ]
    }

    /// Creates the default output: a single `Text` value.
    pub fn create_outputs() -> Vec<Output> {
        vec![
            Output::new("output".to_string(), Value::Text(String::new()), None)
                .with_description("The template with its placeholders filled in."),
        ]
    }

    /// Converts the inputs and fills the template's placeholders in order.
    pub async fn run(inputs: &mut [Input]) -> Result<OperationResponse, OperationError> {
        let start_time = Instant::now();

        convert_inputs! { inputs;
            Text(template) = 0,
            Text(a) = 1,
            Text(b) = 2,
            Text(c) = 3,
        }

        let mut out = template;
        for v in [a, b, c] {
            if let Some(pos) = out.find("{}") {
                out.replace_range(pos..pos + 2, &v);
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
#[path = "template_tests.rs"]
mod tests;
