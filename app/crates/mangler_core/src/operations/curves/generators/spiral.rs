//! Archimedean spiral curve generator.
//!
//! Radius grows linearly with angle (an Archimedean spiral, unlike a
//! logarithmic one) from an inner to an outer radius; emitted as a dense
//! open polyline since the shape isn't representable with a handful of
//! Bezier anchors.

use crate::curve::Curve;
use crate::input::{Input, InputSettings};
use crate::node_settings::NodeSettings;
use crate::operations::curves::common::linear_curve;
use crate::convert_inputs;
use crate::operations::{OperationError, OperationResponse, OutputResponse};
use crate::output::Output;
use crate::value::Value;
use serde::{Deserialize, Serialize};
use std::time::Instant;

#[cfg(test)]
#[path = "spiral_tests.rs"]
mod tests;

/// Hard cap on emitted points, regardless of `turns * points_per_turn`.
const MAX_SPIRAL_POINTS: usize = 2000;

/// Builds an open Archimedean spiral polyline centered at `(cx, cy)`,
/// sweeping `turns` full revolutions from `inner_r` to `outer_r`, rotated by
/// `rotation_deg`, sampled at `points_per_turn` points per revolution
/// (capped overall at [`MAX_SPIRAL_POINTS`]).
pub(crate) fn spiral_points(
    cx: f64,
    cy: f64,
    turns: f64,
    inner_r: f64,
    outer_r: f64,
    rotation_deg: f64,
    points_per_turn: f64,
) -> Vec<[f32; 2]> {
    let rot = rotation_deg.to_radians();
    let n_samples = ((turns * points_per_turn).round() as usize).clamp(2, MAX_SPIRAL_POINTS);
    (0..=n_samples)
        .map(|i| {
            let t = i as f64 / n_samples as f64;
            let angle = t * turns * std::f64::consts::TAU + rot;
            let r = inner_r + (outer_r - inner_r) * t;
            [(cx + r * angle.cos()) as f32, (cy + r * angle.sin()) as f32]
        })
        .collect()
}

/// Operation that generates an Archimedean spiral curve.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpCurveGeneratorSpiral {}

impl OpCurveGeneratorSpiral {
    /// Returns the node metadata (name, description, help).
    pub fn settings() -> NodeSettings {
        NodeSettings {
            name: "spiral".to_string(),
            description: "Generates an Archimedean spiral curve.".to_string(),
            help: "Builds an open Archimedean spiral: radius grows linearly with angle from inner radius to outer radius over the given number of turns (revolutions), sampled as a dense Linear polyline (capped at 2000 points) since a spiral isn't representable with a handful of Bezier anchors.\n\nAll positions and radii are normalized 0-1 curve-space units. Feed the output into rasterize curve, or into meander/jitter for an organic variant.".to_string(),
        }
    }

    /// Creates the default inputs: center x/y, turns, inner/outer radius, rotation, points/turn.
    pub fn create_inputs() -> Vec<Input> {
        vec![
            Input::new("center x".to_string(), Value::Decimal(0.5), Some(InputSettings::Slider { range: (0.0, 1.0), step_by: None, clamp_to_range: false }), None)
                .with_description("Horizontal center of the spiral in normalized [0,1] curve space."),
            Input::new("center y".to_string(), Value::Decimal(0.5), Some(InputSettings::Slider { range: (0.0, 1.0), step_by: None, clamp_to_range: false }), None)
                .with_description("Vertical center of the spiral in normalized [0,1] curve space."),
            Input::new("turns".to_string(), Value::Decimal(3.0), Some(InputSettings::Slider { range: (0.25, 20.0), step_by: Some(0.05), clamp_to_range: false }), None)
                .with_description("Number of full revolutions from the inner to the outer radius."),
            Input::new("inner radius".to_string(), Value::Decimal(0.02), Some(InputSettings::Slider { range: (0.0, 1.0), step_by: None, clamp_to_range: false }), None)
                .with_description("Radius at the spiral's start (center end), in normalized units."),
            Input::new("outer radius".to_string(), Value::Decimal(0.4), Some(InputSettings::Slider { range: (0.0, 1.0), step_by: None, clamp_to_range: false }), None)
                .with_description("Radius at the spiral's end (outermost coil), in normalized units."),
            Input::new("rotation".to_string(), Value::Decimal(0.0), Some(InputSettings::Slider { range: (0.0, 360.0), step_by: None, clamp_to_range: false }), None)
                .with_description("Rotation of the whole spiral about its center, in degrees."),
            Input::new("points per turn".to_string(), Value::Integer(32), Some(InputSettings::DragValue { clamp: Some((8.0, 128.0)), speed: None }), None)
                .with_description("Sample density per revolution (higher = smoother coils, more points). Total point count is capped at 2000."),
        ]
    }

    /// Creates the default output: a single curve output.
    pub fn create_outputs() -> Vec<Output> {
        vec![
            Output::new("output".to_string(), Value::Curve(Curve::default()), None)
                .with_description("The generated open spiral curve."),
        ]
    }

    /// Generates the spiral curve from the given inputs.
    pub async fn run(inputs: &mut [Input]) -> Result<OperationResponse, OperationError> {
        let start_time = Instant::now();

        convert_inputs! { inputs;
            Decimal(cx) = 0,
            Decimal(cy) = 1,
            Decimal(turns) = 2,
            Decimal(inner_r) = 3,
            Decimal(outer_r) = 4,
            Decimal(rotation) = 5,
            Integer(points_per_turn) = 6,
        }

        let turns = (turns as f64).clamp(0.25, 20.0);
        let inner_r = (inner_r as f64).max(0.0);
        let outer_r = (outer_r as f64).max(0.0);
        let points_per_turn = (points_per_turn.clamp(8, 128)) as f64;

        let points = spiral_points(cx as f64, cy as f64, turns, inner_r, outer_r, rotation as f64, points_per_turn);
        let curve = linear_curve(points, false);

        Ok(OperationResponse {
            time: Instant::now().duration_since(start_time),
            responses: vec![OutputResponse { value: Value::Curve(curve) }],
        })
    }
}
