//! Median image luminance.
//!
//! Reduces an image to the median of every pixel's luminance — a robust center
//! that ignores outliers, unlike the arithmetic mean.

use crate::input::Input;
use crate::node_settings::NodeSettings;
use crate::convert_inputs;
use crate::operations::{OperationResponse, OperationError, OutputResponse, image_input};
use crate::output::Output;
use crate::value::Value;
use serde::{Deserialize, Serialize};
use std::time::Instant;

/// Operation that computes the median luminance of an image.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpNumberImageMedian {}

impl OpNumberImageMedian {
    /// Returns the node metadata (name, description, help).
    pub fn settings() -> NodeSettings {
        NodeSettings {
            name: "median".to_string(),
            description: "Reports the median (middle) pixel luminance.".to_string(),
            help: "Sorts every pixel's Rec. 601 luminance (0.299 R + 0.587 G + 0.114 B) and returns the middle value; for an even pixel count it averages the two central values. The median is a robust measure of central brightness that shrugs off a handful of very dark or very bright outliers.\n\nUse it in place of the mean when specular highlights or dead pixels would skew a plain average. An empty image reports 0.".to_string(),
        }
    }

    /// Creates the input port: a single image to measure.
    pub fn create_inputs() -> Vec<Input> {
        vec![
            image_input("image")
                .with_description("Image whose median luminance is measured."),
        ]
    }

    /// Creates the output port: the median luminance.
    pub fn create_outputs() -> Vec<Output> {
        vec![
            Output::new("median".to_string(), Value::Decimal(0.0), None)
                .with_description("Median pixel luminance."),
        ]
    }

    /// Executes the median reduction.
    pub async fn run(inputs: &mut [Input]) -> Result<OperationResponse, OperationError> {
        let start_time = Instant::now();

        convert_inputs! { inputs;
            Image(data) = 0,
        }

        let mut v = super::luma_values(&data);
        let median = if v.is_empty() {
            0.0f32
        } else {
            // `partial_cmp().unwrap()` panics if any pixel's luminance is NaN
            // (e.g. propagated from a divide-by-zero upstream). `f32::total_cmp`
            // gives NaN a well-defined (if somewhat arbitrary) sort position
            // instead, so a stray NaN pixel can't crash the whole node.
            v.sort_by(f32::total_cmp);
            let n = v.len();
            if n % 2 == 1 {
                v[n / 2]
            } else {
                0.5 * (v[n / 2 - 1] + v[n / 2])
            }
        };

        Ok(OperationResponse {
            time: Instant::now().duration_since(start_time),
            responses: vec![
                OutputResponse { value: Value::Decimal(median) },
            ],
        })
    }
}

#[cfg(test)]
#[path = "median_tests.rs"]
mod tests;
