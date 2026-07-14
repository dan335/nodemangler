//! Curve enclosed area.
//!
//! Reports the area a curve encloses, measured via the shoelace formula on
//! its flattened polyline. An open curve is measured as if a segment joined
//! its last point back to its first — the same implicit-close convention
//! [`Curve::signed_area`] documents — so this node never requires a curve to
//! be explicitly closed to get a meaningful area.

use crate::curve::Curve;
use crate::input::Input;
use crate::node_settings::NodeSettings;
use crate::operations::{convert_input, OperationError, OperationResponse, OutputResponse};
use crate::output::Output;
use crate::value::{Value, ValueType};
use serde::{Deserialize, Serialize};
use std::time::Instant;

/// Operation that computes the enclosed area of a curve.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpNumberCurveArea {}

impl OpNumberCurveArea {
    /// Returns the node metadata (name, description, help).
    pub fn settings() -> NodeSettings {
        NodeSettings {
            name: "area".to_string(),
            description: "Reports the area a curve encloses.".to_string(),
            help: "Flattens the curve and computes its enclosed area via the shoelace formula, in normalized [0,1]² units. Open curves are implicitly closed for this measurement — a segment is assumed from the last point back to the first — so you don't need to explicitly close a curve to get a meaningful area.\n\n`area` is always non-negative (absolute value); `signed area` keeps the sign, which encodes winding direction: because coordinates are y-down (image convention), positive means clockwise on screen and negative means counter-clockwise. Fewer than 3 points reports 0 for both.".to_string(),
        }
    }

    /// Creates the input port: a single curve to measure.
    pub fn create_inputs() -> Vec<Input> {
        vec![
            Input::new("curve".to_string(), Value::Curve(Curve::default()), None, None)
                .with_description("Curve whose enclosed area is measured."),
        ]
    }

    /// Creates the output ports: absolute area and signed area.
    pub fn create_outputs() -> Vec<Output> {
        vec![
            Output::new("area".to_string(), Value::Decimal(0.0), None)
                .with_description("Enclosed area (always non-negative)."),
            Output::new("signed area".to_string(), Value::Decimal(0.0), None)
                .with_description("Enclosed area with sign: positive = clockwise on screen, negative = counter-clockwise."),
        ]
    }

    /// Executes the area computation.
    pub async fn run(inputs: &mut [Input]) -> Result<OperationResponse, OperationError> {
        let start_time = Instant::now();
        let mut input_errors: Vec<(usize, String)> = vec![];

        let curve_converted = convert_input(inputs, 0, ValueType::Curve, &mut input_errors);

        if !input_errors.is_empty() { return Err(OperationError { input_errors, node_error: None }); }

        let Value::Curve(curve) = curve_converted.unwrap() else { unreachable!() };

        let signed = curve.signed_area();

        Ok(OperationResponse {
            time: Instant::now().duration_since(start_time),
            responses: vec![
                OutputResponse { value: Value::Decimal(signed.abs()) },
                OutputResponse { value: Value::Decimal(signed) },
            ],
        })
    }
}

#[cfg(test)]
#[path = "area_tests.rs"]
mod tests;
