//! 2D distance operation for the node graph.
//!
//! Computes the Euclidean distance between two points `(x1, y1)` and
//! `(x2, y2)` using the numerically stable `f32::hypot`.

use crate::input::{Input, InputSettings};
use crate::node_settings::NodeSettings;
use crate::operations::{OperationResponse, OperationError, OutputResponse, convert_input};
use crate::output::Output;
use crate::value::{Value, ValueType};
use serde::{Deserialize, Serialize};
use std::time::Instant;

/// Node operation that computes the Euclidean distance between two 2D points.
///
/// All four inputs are converted to decimal and combined with
/// `(x2 - x1).hypot(y2 - y1)`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpNumberMathDistance2d {}

impl OpNumberMathDistance2d {
    /// Returns the node metadata (name and description).
    pub fn settings() -> NodeSettings {
        NodeSettings {
            name: "distance 2d".to_string(),
            description: "Euclidean distance between two 2D points.".to_string(),
            help: "Returns the straight-line distance between the points (x1, y1) and (x2, y2), computed as sqrt((x2 - x1)^2 + (y2 - y1)^2). The points (0, 0) and (3, 4) are 5 apart.\n\nUses f32::hypot internally for numerical stability, so it avoids the overflow or underflow of squaring large or tiny coordinate differences directly.".to_string(),
        }
    }

    /// Creates the default input list: `x1` (0.0), `y1` (0.0), `x2` (3.0), `y2` (4.0).
    pub fn create_inputs() -> Vec<Input> {
        vec![
            Input::new("x1".to_string(), Value::Decimal(0.0), Some(InputSettings::DragValue { speed: None, clamp: None }), None)
                .with_description("X coordinate of the first point."),
            Input::new("y1".to_string(), Value::Decimal(0.0), Some(InputSettings::DragValue { speed: None, clamp: None }), None)
                .with_description("Y coordinate of the first point."),
            Input::new("x2".to_string(), Value::Decimal(3.0), Some(InputSettings::DragValue { speed: None, clamp: None }), None)
                .with_description("X coordinate of the second point."),
            Input::new("y2".to_string(), Value::Decimal(4.0), Some(InputSettings::DragValue { speed: None, clamp: None }), None)
                .with_description("Y coordinate of the second point."),
        ]
    }

    /// Creates the default output list: a single decimal output.
    pub fn create_outputs() -> Vec<Output> {
        vec![
            Output::new("distance".to_string(), Value::Decimal(0.0), None)
                .with_description("Euclidean distance between the two points.")
        ]
    }

    /// Executes the distance operation: computes `(x2 - x1).hypot(y2 - y1)`.
    pub async fn run(inputs: &mut [Input]) -> Result<OperationResponse, OperationError> {
        let start_time = Instant::now();
        let mut input_errors: Vec<(usize, String)> = vec![];

        let x1_converted = convert_input(inputs, 0, ValueType::Decimal, &mut input_errors);
        let y1_converted = convert_input(inputs, 1, ValueType::Decimal, &mut input_errors);
        let x2_converted = convert_input(inputs, 2, ValueType::Decimal, &mut input_errors);
        let y2_converted = convert_input(inputs, 3, ValueType::Decimal, &mut input_errors);

        if !input_errors.is_empty() { return Err(OperationError { input_errors, node_error: None }); }

        let Value::Decimal(x1) = x1_converted.unwrap() else { unreachable!() };
        let Value::Decimal(y1) = y1_converted.unwrap() else { unreachable!() };
        let Value::Decimal(x2) = x2_converted.unwrap() else { unreachable!() };
        let Value::Decimal(y2) = y2_converted.unwrap() else { unreachable!() };

        let distance = (x2 - x1).hypot(y2 - y1);

        Ok(OperationResponse {
            time: Instant::now().duration_since(start_time),
            responses: vec![OutputResponse {
                value: Value::Decimal(distance),
            }],
        })
    }
}

#[cfg(test)]
#[path = "distance_2d_tests.rs"]
mod tests;
