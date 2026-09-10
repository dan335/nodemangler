//! Curve control-point count.
//!
//! Reports the number of *control* points a curve carries — the same count
//! the Preview2D overlay editor shows and lets you drag — not the much larger
//! number of samples a `Smooth`/`Bezier` curve flattens into for rendering.

use crate::curve::Curve;
use crate::input::Input;
use crate::node_settings::NodeSettings;
use crate::convert_inputs;
use crate::operations::{OperationError, OperationResponse, OutputResponse};
use crate::output::Output;
use crate::value::Value;
use serde::{Deserialize, Serialize};
use std::time::Instant;

/// Operation that reports a curve's control-point count.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpNumberCurvePointCount {}

impl OpNumberCurvePointCount {
    /// Returns the node metadata (name, description, help).
    pub fn settings() -> NodeSettings {
        NodeSettings {
            name: "point count".to_string(),
            description: "Reports a curve's control-point count.".to_string(),
            help: "Counts the control points a curve carries — the same points the Preview2D overlay editor lets you drag. This is not the flattened sample count: a Smooth or Bezier curve renders through many more interpolated points than it has control points, so this node reflects the curve's editable structure, not its rendered density.".to_string(),
        }
    }

    /// Creates the input port: a single curve to measure.
    pub fn create_inputs() -> Vec<Input> {
        vec![
            Input::new("curve".to_string(), Value::Curve(Curve::default()), None, None)
                .with_description("Curve whose control points are counted."),
        ]
    }

    /// Creates the output port: control-point count.
    pub fn create_outputs() -> Vec<Output> {
        vec![
            Output::new("points".to_string(), Value::Integer(0), None)
                .with_description("Number of control points."),
        ]
    }

    /// Executes the measurement.
    pub async fn run(inputs: &mut [Input]) -> Result<OperationResponse, OperationError> {
        let start_time = Instant::now();

        convert_inputs! { inputs;
            Curve(curve) = 0,
        }

        Ok(OperationResponse {
            time: Instant::now().duration_since(start_time),
            responses: vec![
                OutputResponse { value: Value::Integer(curve.points.len() as i32) },
            ],
        })
    }
}

#[cfg(test)]
#[path = "point_count_tests.rs"]
mod tests;
