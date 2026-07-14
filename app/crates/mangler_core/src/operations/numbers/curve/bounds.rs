//! Curve axis-aligned bounding box.
//!
//! Reports the bounding box of a curve's flattened polyline, in normalized
//! `[0,1]²` units, so downstream math can react to how much space a curve
//! actually occupies (e.g. deciding a raster size or an offset to re-center
//! it).

use crate::curve::Curve;
use crate::input::Input;
use crate::node_settings::NodeSettings;
use crate::operations::{convert_input, OperationError, OperationResponse, OutputResponse};
use crate::output::Output;
use crate::value::{Value, ValueType};
use serde::{Deserialize, Serialize};
use std::time::Instant;

/// Operation that reports a curve's axis-aligned bounding box.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpNumberCurveBounds {}

impl OpNumberCurveBounds {
    /// Returns the node metadata (name, description, help).
    pub fn settings() -> NodeSettings {
        NodeSettings {
            name: "bounds".to_string(),
            description: "Reports a curve's axis-aligned bounding box.".to_string(),
            help: "Flattens the curve and reports the smallest axis-aligned box that contains it: x/y of the top-left corner and its width/height, all in the curve's own normalized [0,1]² units. An empty curve (no points) reports all zeros.".to_string(),
        }
    }

    /// Creates the input port: a single curve to measure.
    pub fn create_inputs() -> Vec<Input> {
        vec![
            Input::new("curve".to_string(), Value::Curve(Curve::default()), None, None)
                .with_description("Curve whose bounding box is measured."),
        ]
    }

    /// Creates the output ports: x, y, width, height.
    pub fn create_outputs() -> Vec<Output> {
        vec![
            Output::new("x".to_string(), Value::Decimal(0.0), None)
                .with_description("Left edge of the bounding box."),
            Output::new("y".to_string(), Value::Decimal(0.0), None)
                .with_description("Top edge of the bounding box."),
            Output::new("width".to_string(), Value::Decimal(0.0), None)
                .with_description("Bounding box width."),
            Output::new("height".to_string(), Value::Decimal(0.0), None)
                .with_description("Bounding box height."),
        ]
    }

    /// Executes the measurement.
    pub async fn run(inputs: &mut [Input]) -> Result<OperationResponse, OperationError> {
        let start_time = Instant::now();
        let mut input_errors: Vec<(usize, String)> = vec![];

        let curve_converted = convert_input(inputs, 0, ValueType::Curve, &mut input_errors);

        if !input_errors.is_empty() { return Err(OperationError { input_errors, node_error: None }); }

        let Value::Curve(curve) = curve_converted.unwrap() else { unreachable!() };

        let [x, y, w, h] = curve.bounds().unwrap_or([0.0, 0.0, 0.0, 0.0]);

        Ok(OperationResponse {
            time: Instant::now().duration_since(start_time),
            responses: vec![
                OutputResponse { value: Value::Decimal(x) },
                OutputResponse { value: Value::Decimal(y) },
                OutputResponse { value: Value::Decimal(w) },
                OutputResponse { value: Value::Decimal(h) },
            ],
        })
    }
}

#[cfg(test)]
#[path = "bounds_tests.rs"]
mod tests;
