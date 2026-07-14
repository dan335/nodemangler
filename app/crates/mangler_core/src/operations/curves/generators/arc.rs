//! Arc curve generator.
//!
//! Builds an open circular arc as one cubic-Bezier span per <=90-degree
//! sweep segment, using the standard per-span handle formula
//! `k = (4/3) * tan(delta/4)`. A sweep of +-360 degrees or more closes into a
//! full circle (the same 4-span construction as the ellipse generator).

use crate::curve::{Curve, CurveInterpolation};
use crate::input::{Input, InputSettings};
use crate::node_settings::NodeSettings;
use crate::operations::{convert_input, OperationError, OperationResponse, OutputResponse};
use crate::output::Output;
use crate::value::{Value, ValueType};
use serde::{Deserialize, Serialize};
use std::time::Instant;

#[cfg(test)]
#[path = "arc_tests.rs"]
mod tests;

/// Builds an arc curve centered at `(cx, cy)` with the given `radius`,
/// starting at `start_deg` and sweeping `sweep_deg` (signed; negative sweeps
/// counterclockwise). `|sweep_deg| >= 360` closes into a full circle (open
/// otherwise). Each anchor's mirrored tangent handle is the unit tangent at
/// that anchor's angle times `radius` times `(4/3) * tan(delta/4)`, where
/// `delta` is the (equal) per-span sweep in radians — every span is kept
/// `<=90` degrees by splitting the sweep into `ceil(|sweep|/90)` equal spans.
pub(crate) fn arc_curve(cx: f64, cy: f64, radius: f64, start_deg: f64, sweep_deg: f64) -> Curve {
    // Floor the sweep magnitude at 1 degree and cap it at a full turn -
    // anything beyond 360 degrees retraces the same circle.
    let sign = if sweep_deg < 0.0 { -1.0 } else { 1.0 };
    let mag = sweep_deg.abs().clamp(1.0, 360.0);
    let closed = mag >= 360.0;
    let effective_sweep = sign * mag;

    let num_spans = ((mag / 90.0).ceil() as usize).max(1);
    let delta_deg = effective_sweep / num_spans as f64;
    let delta = delta_deg.to_radians();
    let k = (4.0 / 3.0) * (delta / 4.0).tan();
    let start_rad = start_deg.to_radians();

    // A full circle only needs 4 distinct anchors (the last coincides with
    // the first); an open arc keeps every span endpoint.
    let anchor_count = if closed { num_spans } else { num_spans + 1 };
    let mut points = Vec::with_capacity(anchor_count);
    let mut handles = Vec::with_capacity(anchor_count);
    for i in 0..anchor_count {
        let theta = start_rad + i as f64 * delta;
        let (s, c) = theta.sin_cos();
        points.push([(cx + radius * c) as f32, (cy + radius * s) as f32]);
        handles.push([(-radius * k * s) as f32, (radius * k * c) as f32]);
    }

    Curve { points, closed, interpolation: CurveInterpolation::Bezier, handles }
}

/// Operation that generates an open (or, at a full-turn sweep, closed)
/// circular arc curve.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpCurveGeneratorArc {}

impl OpCurveGeneratorArc {
    /// Returns the node metadata (name, description, help).
    pub fn settings() -> NodeSettings {
        NodeSettings {
            name: "arc".to_string(),
            description: "Generates a circular arc curve.".to_string(),
            help: "Builds a circular arc as one cubic-Bezier span per <=90-degree sweep segment (accurate to a fraction of a percent of the radius), hand-editable afterward in the curve overlay. Start angle is measured from the +x axis (0 = right of center), increasing clockwise in this y-down space; sweep is signed (negative sweeps counterclockwise) and is floored at 1 degree. A sweep of 360 degrees or more closes the arc into a full circle instead of leaving a duplicate seam point.\n\nAll positions and the radius are normalized 0-1 curve-space units. Feed the output into rasterize curve, or chain several arcs with join for compound shapes.".to_string(),
        }
    }

    /// Creates the default inputs: center x/y, radius, start angle, sweep.
    pub fn create_inputs() -> Vec<Input> {
        vec![
            Input::new("center x".to_string(), Value::Decimal(0.5), Some(InputSettings::Slider { range: (0.0, 1.0), step_by: None, clamp_to_range: false }), None)
                .with_description("Horizontal center of the arc's circle in normalized [0,1] curve space."),
            Input::new("center y".to_string(), Value::Decimal(0.5), Some(InputSettings::Slider { range: (0.0, 1.0), step_by: None, clamp_to_range: false }), None)
                .with_description("Vertical center of the arc's circle in normalized [0,1] curve space."),
            Input::new("radius".to_string(), Value::Decimal(0.3), Some(InputSettings::Slider { range: (0.0, 1.0), step_by: None, clamp_to_range: false }), None)
                .with_description("Radius of the arc's circle in normalized units."),
            Input::new("start angle".to_string(), Value::Decimal(0.0), Some(InputSettings::Slider { range: (0.0, 360.0), step_by: None, clamp_to_range: false }), None)
                .with_description("Starting angle in degrees, measured from the +x axis, increasing clockwise."),
            Input::new("sweep".to_string(), Value::Decimal(90.0), Some(InputSettings::Slider { range: (-360.0, 360.0), step_by: None, clamp_to_range: false }), None)
                .with_description("Signed sweep angle in degrees (negative = counterclockwise). +-360 or beyond closes into a full circle. Floored at 1 degree of magnitude."),
        ]
    }

    /// Creates the default output: a single curve output.
    pub fn create_outputs() -> Vec<Output> {
        vec![
            Output::new("output".to_string(), Value::Curve(Curve::default()), None)
                .with_description("The generated arc curve (open, unless the sweep closes it into a full circle)."),
        ]
    }

    /// Generates the arc curve from the given inputs.
    pub async fn run(inputs: &mut [Input]) -> Result<OperationResponse, OperationError> {
        let start_time = Instant::now();
        let mut input_errors: Vec<(usize, String)> = vec![];

        let cx_converted = convert_input(inputs, 0, ValueType::Decimal, &mut input_errors);
        let cy_converted = convert_input(inputs, 1, ValueType::Decimal, &mut input_errors);
        let radius_converted = convert_input(inputs, 2, ValueType::Decimal, &mut input_errors);
        let start_angle_converted = convert_input(inputs, 3, ValueType::Decimal, &mut input_errors);
        let sweep_converted = convert_input(inputs, 4, ValueType::Decimal, &mut input_errors);

        if !input_errors.is_empty() {
            return Err(OperationError { input_errors, node_error: None });
        }

        let Value::Decimal(cx) = cx_converted.unwrap() else { unreachable!() };
        let Value::Decimal(cy) = cy_converted.unwrap() else { unreachable!() };
        let Value::Decimal(radius) = radius_converted.unwrap() else { unreachable!() };
        let Value::Decimal(start_angle) = start_angle_converted.unwrap() else { unreachable!() };
        let Value::Decimal(sweep) = sweep_converted.unwrap() else { unreachable!() };

        let radius = (radius as f64).max(0.001);

        let curve = arc_curve(cx as f64, cy as f64, radius, start_angle as f64, sweep as f64);

        Ok(OperationResponse {
            time: Instant::now().duration_since(start_time),
            responses: vec![OutputResponse { value: Value::Curve(curve) }],
        })
    }
}
