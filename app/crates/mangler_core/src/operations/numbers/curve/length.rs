//! Curve arc length.
//!
//! Reports the total arc length of a curve's flattened polyline, in
//! normalized `[0,1]²` units, so downstream math can key off how long a path
//! is (e.g. deciding a stamp count or spacing for something that walks the
//! curve).

use crate::curve::Curve;
use crate::input::Input;
use crate::node_settings::NodeSettings;
use crate::convert_inputs;
use crate::operations::{OperationError, OperationResponse, OutputResponse};
use crate::output::Output;
use crate::value::Value;
use serde::{Deserialize, Serialize};
use std::time::Instant;

/// Operation that reports a curve's total arc length.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpNumberCurveLength {}

impl OpNumberCurveLength {
    /// Returns the node metadata (name, description, help).
    pub fn settings() -> NodeSettings {
        NodeSettings {
            name: "length".to_string(),
            description: "Reports a curve's total arc length.".to_string(),
            help: "Flattens the curve and sums the distance between consecutive points, in the curve's own normalized [0,1]² units (not pixels — multiply by an image's max dimension to convert). An empty curve (no points) reports 0.".to_string(),
        }
    }

    /// Creates the input port: a single curve to measure.
    pub fn create_inputs() -> Vec<Input> {
        vec![
            Input::new("curve".to_string(), Value::Curve(Curve::default()), None, None)
                .with_description("Curve whose arc length is measured."),
        ]
    }

    /// Creates the output port: total arc length.
    pub fn create_outputs() -> Vec<Output> {
        vec![
            Output::new("length".to_string(), Value::Decimal(0.0), None)
                .with_description("Total arc length in normalized [0,1]² units."),
        ]
    }

    /// Executes the measurement.
    pub async fn run(inputs: &mut [Input]) -> Result<OperationResponse, OperationError> {
        let start_time = Instant::now();

        convert_inputs! { inputs;
            Curve(curve) = 0,
        }

        // `Curve::length` already flattens to an empty polyline for 0 points,
        // summing to 0.0 with no special-casing needed here.
        let length = curve.length();

        Ok(OperationResponse {
            time: Instant::now().duration_since(start_time),
            responses: vec![
                OutputResponse { value: Value::Decimal(length) },
            ],
        })
    }
}

#[cfg(test)]
#[path = "length_tests.rs"]
mod tests;
