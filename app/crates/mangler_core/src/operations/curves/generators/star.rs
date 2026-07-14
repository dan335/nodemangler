//! Star curve generator.
//!
//! Builds a closed, exact-vertex star polygon (alternating outer/inner
//! radii) — the curve twin of the SDF-rasterized `images/shapes/star` node,
//! whose parameter names it mirrors (`points`, `outer_radius`,
//! `inner_radius`, `rotation`).

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
#[path = "star_tests.rs"]
mod tests;

/// Builds the `2 * points` vertices of a star, alternating between
/// `outer_radius` and `inner_radius`, first (outer) vertex straight up,
/// rotated `rotation_deg` clockwise.
pub(crate) fn star_points(cx: f64, cy: f64, outer: f64, inner: f64, points: usize, rotation_deg: f64) -> Vec<[f32; 2]> {
    let rot = rotation_deg.to_radians();
    let total = points * 2;
    let step = std::f64::consts::TAU / total as f64;
    (0..total)
        .map(|i| {
            let a = step * i as f64 - std::f64::consts::FRAC_PI_2 + rot;
            let r = if i % 2 == 0 { outer } else { inner };
            [(cx + r * a.cos()) as f32, (cy + r * a.sin()) as f32]
        })
        .collect()
}

/// Operation that generates a closed star curve.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpCurveGeneratorStar {}

impl OpCurveGeneratorStar {
    /// Returns the node metadata (name, description, help).
    pub fn settings() -> NodeSettings {
        NodeSettings {
            name: "star".to_string(),
            description: "Generates a closed star curve.".to_string(),
            help: "Builds a closed star polygon (3-64 points) as exact straight-line vertices (Linear interpolation), alternating between outer_radius (spike tips) and inner_radius (valley points) — the curve twin of the rasterized star shape node. The first spike points straight up from the center; rotation tilts the whole star clockwise.\n\nAll positions are normalized 0-1 curve-space units. Feed the output into rasterize curve for a filled/stroked mask, or into round corners for a rounded-spike variant.".to_string(),
        }
    }

    /// Creates the default inputs: center x/y, points, outer/inner radius, rotation.
    pub fn create_inputs() -> Vec<Input> {
        vec![
            Input::new("center x".to_string(), Value::Decimal(0.5), Some(InputSettings::Slider { range: (0.0, 1.0), step_by: None, clamp_to_range: false }), None)
                .with_description("Horizontal center of the star in normalized [0,1] curve space."),
            Input::new("center y".to_string(), Value::Decimal(0.5), Some(InputSettings::Slider { range: (0.0, 1.0), step_by: None, clamp_to_range: false }), None)
                .with_description("Vertical center of the star in normalized [0,1] curve space."),
            Input::new("points".to_string(), Value::Integer(5), Some(InputSettings::DragValue { clamp: Some((3.0, 64.0)), speed: None }), None)
                .with_description("Number of points (spikes) on the star (minimum 3, maximum 64)."),
            Input::new("outer_radius".to_string(), Value::Decimal(0.35), Some(InputSettings::Slider { range: (0.0, 1.0), step_by: None, clamp_to_range: false }), None)
                .with_description("Radius out to the tips of the spikes in normalized units."),
            Input::new("inner_radius".to_string(), Value::Decimal(0.15), Some(InputSettings::Slider { range: (0.0, 1.0), step_by: None, clamp_to_range: false }), None)
                .with_description("Radius out to the valleys between spikes in normalized units."),
            Input::new("rotation".to_string(), Value::Decimal(0.0), Some(InputSettings::Slider { range: (0.0, 360.0), step_by: None, clamp_to_range: false }), None)
                .with_description("Rotation about the center, in degrees. 0 points the first spike straight up."),
        ]
    }

    /// Creates the default output: a single curve output.
    pub fn create_outputs() -> Vec<Output> {
        vec![
            Output::new("output".to_string(), Value::Curve(Curve::default()), None)
                .with_description("The generated closed star curve."),
        ]
    }

    /// Generates the star curve from the given inputs.
    pub async fn run(inputs: &mut [Input]) -> Result<OperationResponse, OperationError> {
        let start_time = Instant::now();
        let mut input_errors: Vec<(usize, String)> = vec![];

        let cx_converted = convert_input(inputs, 0, ValueType::Decimal, &mut input_errors);
        let cy_converted = convert_input(inputs, 1, ValueType::Decimal, &mut input_errors);
        let points_converted = convert_input(inputs, 2, ValueType::Integer, &mut input_errors);
        let outer_converted = convert_input(inputs, 3, ValueType::Decimal, &mut input_errors);
        let inner_converted = convert_input(inputs, 4, ValueType::Decimal, &mut input_errors);
        let rotation_converted = convert_input(inputs, 5, ValueType::Decimal, &mut input_errors);

        if !input_errors.is_empty() {
            return Err(OperationError { input_errors, node_error: None });
        }

        let Value::Decimal(cx) = cx_converted.unwrap() else { unreachable!() };
        let Value::Decimal(cy) = cy_converted.unwrap() else { unreachable!() };
        let Value::Integer(points) = points_converted.unwrap() else { unreachable!() };
        let Value::Decimal(outer) = outer_converted.unwrap() else { unreachable!() };
        let Value::Decimal(inner) = inner_converted.unwrap() else { unreachable!() };
        let Value::Decimal(rotation) = rotation_converted.unwrap() else { unreachable!() };

        let points = points.clamp(3, 64) as usize;
        let outer = (outer as f64).max(0.001);
        let inner = (inner as f64).max(0.001);

        let verts = star_points(cx as f64, cy as f64, outer, inner, points, rotation as f64);
        let curve = linear_curve(verts, true);

        Ok(OperationResponse {
            time: Instant::now().duration_since(start_time),
            responses: vec![OutputResponse { value: Value::Curve(curve) }],
        })
    }
}
