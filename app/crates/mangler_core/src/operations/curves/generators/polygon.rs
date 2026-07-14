//! Regular polygon curve generator.
//!
//! Builds a closed, exact-vertex regular polygon (n straight sides), unlike
//! the SDF-rasterized `images/shapes/polygon` node — this one emits the
//! control points directly, so downstream nodes see the true vertices.

use crate::curve::Curve;
use crate::input::{Input, InputSettings};
use crate::node_settings::NodeSettings;
use crate::operations::curves::common::linear_curve;
use crate::operations::{convert_input, OperationError, OperationResponse, OutputResponse};
use crate::output::Output;
use crate::value::{Value, ValueType};
use serde::{Deserialize, Serialize};
use std::time::Instant;

#[cfg(test)]
#[path = "polygon_tests.rs"]
mod tests;

/// Builds the `sides` vertices of a regular polygon inscribed in a circle of
/// `radius` centered at `(cx, cy)`, first vertex straight up (matching
/// `images/shapes/star`'s convention), rotated `rotation_deg` clockwise.
pub(crate) fn polygon_points(cx: f64, cy: f64, radius: f64, sides: usize, rotation_deg: f64) -> Vec<[f32; 2]> {
    let rot = rotation_deg.to_radians();
    let step = std::f64::consts::TAU / sides as f64;
    (0..sides)
        .map(|i| {
            let a = step * i as f64 - std::f64::consts::FRAC_PI_2 + rot;
            [(cx + radius * a.cos()) as f32, (cy + radius * a.sin()) as f32]
        })
        .collect()
}

/// Operation that generates a closed regular polygon curve.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpCurveGeneratorPolygon {}

impl OpCurveGeneratorPolygon {
    /// Returns the node metadata (name, description, help).
    pub fn settings() -> NodeSettings {
        NodeSettings {
            name: "polygon".to_string(),
            description: "Generates a closed regular polygon curve.".to_string(),
            help: "Builds a closed regular polygon (3-64 sides) inscribed in a circle of the given radius, as exact straight-line vertices (Linear interpolation) — no approximation, unlike a rasterized polygon shape. The first vertex points straight up from the center; rotation tilts the whole polygon clockwise.\n\nAll positions are normalized 0-1 curve-space units. Feed the output into rasterize curve for a filled/stroked mask, or into a modifier node (round corners, jitter, ...) for further shaping.".to_string(),
        }
    }

    /// Creates the default inputs: center x/y, sides, radius, rotation.
    pub fn create_inputs() -> Vec<Input> {
        vec![
            Input::new("center x".to_string(), Value::Decimal(0.5), Some(InputSettings::Slider { range: (0.0, 1.0), step_by: None, clamp_to_range: false }), None)
                .with_description("Horizontal center of the polygon in normalized [0,1] curve space."),
            Input::new("center y".to_string(), Value::Decimal(0.5), Some(InputSettings::Slider { range: (0.0, 1.0), step_by: None, clamp_to_range: false }), None)
                .with_description("Vertical center of the polygon in normalized [0,1] curve space."),
            Input::new("radius".to_string(), Value::Decimal(0.3), Some(InputSettings::Slider { range: (0.0, 1.0), step_by: None, clamp_to_range: false }), None)
                .with_description("Circumscribed radius in normalized units (distance from center to each vertex)."),
            Input::new("sides".to_string(), Value::Integer(6), Some(InputSettings::DragValue { clamp: Some((3.0, 64.0)), speed: None }), None)
                .with_description("Number of sides (minimum 3, maximum 64)."),
            Input::new("rotation".to_string(), Value::Decimal(0.0), Some(InputSettings::Slider { range: (0.0, 360.0), step_by: None, clamp_to_range: false }), None)
                .with_description("Rotation about the center, in degrees. 0 points the first vertex straight up."),
        ]
    }

    /// Creates the default output: a single curve output.
    pub fn create_outputs() -> Vec<Output> {
        vec![
            Output::new("output".to_string(), Value::Curve(Curve::default()), None)
                .with_description("The generated closed polygon curve."),
        ]
    }

    /// Generates the polygon curve from the given inputs.
    pub async fn run(inputs: &mut [Input]) -> Result<OperationResponse, OperationError> {
        let start_time = Instant::now();
        let mut input_errors: Vec<(usize, String)> = vec![];

        let cx_converted = convert_input(inputs, 0, ValueType::Decimal, &mut input_errors);
        let cy_converted = convert_input(inputs, 1, ValueType::Decimal, &mut input_errors);
        let radius_converted = convert_input(inputs, 2, ValueType::Decimal, &mut input_errors);
        let sides_converted = convert_input(inputs, 3, ValueType::Integer, &mut input_errors);
        let rotation_converted = convert_input(inputs, 4, ValueType::Decimal, &mut input_errors);

        if !input_errors.is_empty() {
            return Err(OperationError { input_errors, node_error: None });
        }

        let Value::Decimal(cx) = cx_converted.unwrap() else { unreachable!() };
        let Value::Decimal(cy) = cy_converted.unwrap() else { unreachable!() };
        let Value::Decimal(radius) = radius_converted.unwrap() else { unreachable!() };
        let Value::Integer(sides) = sides_converted.unwrap() else { unreachable!() };
        let Value::Decimal(rotation) = rotation_converted.unwrap() else { unreachable!() };

        let radius = (radius as f64).max(0.001);
        let sides = sides.clamp(3, 64) as usize;

        let points = polygon_points(cx as f64, cy as f64, radius, sides, rotation as f64);
        let curve = linear_curve(points, true);

        Ok(OperationResponse {
            time: Instant::now().duration_since(start_time),
            responses: vec![OutputResponse { value: Value::Curve(curve) }],
        })
    }
}
