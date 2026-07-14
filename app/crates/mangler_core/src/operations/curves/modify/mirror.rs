//! Curve mirror modifier: reflects across an axis through a pivot.
//!
//! Structure-preserving — reflects `points` and `handles` (as vectors)
//! directly, keeping `interpolation` and `closed` unchanged. The axis is a
//! line through `(pivot x, pivot y)` at a given angle: `vertical` = 90
//! degrees (mirrors left-right), `horizontal` = 0 degrees (mirrors
//! top-bottom), `custom` uses the `angle` input directly.

use crate::curve::Curve;
use crate::input::{Input, InputSettings};
use crate::node_settings::NodeSettings;
use crate::operations::{convert_input, OperationError, OperationResponse, OutputResponse};
use crate::output::Output;
use crate::value::{Value, ValueType};
use serde::{Deserialize, Serialize};
use std::time::Instant;

#[cfg(test)]
#[path = "mirror_tests.rs"]
mod tests;

/// Reflects point `p` across the line through `pivot` with unit direction
/// `d`: `v' = 2*(v . d)*d - v`, where `v = p - pivot`.
fn reflect_point(p: [f64; 2], pivot: [f64; 2], d: [f64; 2]) -> [f64; 2] {
    let v = [p[0] - pivot[0], p[1] - pivot[1]];
    let comp = v[0] * d[0] + v[1] * d[1];
    let vr = [2.0 * comp * d[0] - v[0], 2.0 * comp * d[1] - v[1]];
    [pivot[0] + vr[0], pivot[1] + vr[1]]
}

/// Reflects a vector (e.g. a Bezier handle offset) across a line direction
/// `d`, with no pivot translation.
fn reflect_vector(h: [f64; 2], d: [f64; 2]) -> [f64; 2] {
    let comp = h[0] * d[0] + h[1] * d[1];
    [2.0 * comp * d[0] - h[0], 2.0 * comp * d[1] - h[1]]
}

/// Resolves the mirror axis's direction unit vector for the given `axis`
/// selector (`"horizontal"` = 0 degrees, `"vertical"` = 90 degrees,
/// otherwise `custom_angle_deg`).
fn axis_direction(axis: &str, custom_angle_deg: f64) -> [f64; 2] {
    let angle_deg = match axis {
        "horizontal" => 0.0,
        "vertical" => 90.0,
        _ => custom_angle_deg,
    };
    let theta = angle_deg.to_radians();
    [theta.cos(), theta.sin()]
}

/// Mirrors `curve`'s points and handle vectors across the axis through
/// `pivot` resolved by [`axis_direction`]. Fewer than 2 points passes through
/// unchanged.
pub(crate) fn mirror_curve(curve: &Curve, axis: &str, custom_angle_deg: f64, pivot: [f64; 2]) -> Curve {
    if curve.points.len() < 2 {
        return curve.clone();
    }
    let d = axis_direction(axis, custom_angle_deg);

    let points = curve
        .points
        .iter()
        .map(|p| {
            let r = reflect_point([p[0] as f64, p[1] as f64], pivot, d);
            [r[0] as f32, r[1] as f32]
        })
        .collect();

    let handles = curve
        .handles
        .iter()
        .map(|h| {
            let r = reflect_vector([h[0] as f64, h[1] as f64], d);
            [r[0] as f32, r[1] as f32]
        })
        .collect();

    Curve { points, closed: curve.closed, interpolation: curve.interpolation, handles }
}

/// Operation that mirrors a curve across an axis through a pivot point.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpCurveModifyMirror {}

impl OpCurveModifyMirror {
    /// Returns the node metadata (name, description, help).
    pub fn settings() -> NodeSettings {
        NodeSettings {
            name: "mirror".to_string(),
            description: "Reflects a curve across an axis through a pivot point.".to_string(),
            help: "Structure-preserving: reflects the curve's control points (and, in Bezier mode, its tangent handles as vectors) across a line through (pivot x, pivot y) - vertical mirrors left-right, horizontal mirrors top-bottom, custom uses the angle input as the axis line's angle in degrees (0 = horizontal, 90 = vertical, matching the vertical/horizontal presets). Point count, interpolation mode, and open/closed state are all unchanged; applying mirror twice with the same settings returns the original curve.\n\nPivot position is in normalized 0-1 curve-space units.".to_string(),
        }
    }

    /// Creates the default inputs: curve, axis, angle, pivot x/y.
    pub fn create_inputs() -> Vec<Input> {
        vec![
            Input::new("curve".to_string(), Value::Curve(Curve::default()), None, None)
                .with_description("The curve to mirror."),
            Input::new("axis".to_string(), Value::Text("vertical".to_string()), Some(InputSettings::Dropdown {
                options: vec!["vertical".to_string(), "horizontal".to_string(), "custom".to_string()],
            }), None)
                .with_description("Mirror axis: vertical (left-right), horizontal (top-bottom), or custom (uses the angle input)."),
            Input::new("angle".to_string(), Value::Decimal(0.0), Some(InputSettings::Slider { range: (0.0, 360.0), step_by: None, clamp_to_range: false }), None)
                .with_description("Axis angle in degrees (0 = horizontal, 90 = vertical). Used only when axis = custom."),
            Input::new("pivot x".to_string(), Value::Decimal(0.5), Some(InputSettings::Slider { range: (0.0, 1.0), step_by: None, clamp_to_range: false }), None)
                .with_description("Horizontal position the mirror axis passes through, in normalized curve-space units."),
            Input::new("pivot y".to_string(), Value::Decimal(0.5), Some(InputSettings::Slider { range: (0.0, 1.0), step_by: None, clamp_to_range: false }), None)
                .with_description("Vertical position the mirror axis passes through, in normalized curve-space units."),
        ]
    }

    /// Creates the default output: a single curve output.
    pub fn create_outputs() -> Vec<Output> {
        vec![
            Output::new("output".to_string(), Value::Curve(Curve::default()), None)
                .with_description("The mirrored curve."),
        ]
    }

    /// Mirrors the curve from the given inputs.
    pub async fn run(inputs: &mut [Input]) -> Result<OperationResponse, OperationError> {
        let start_time = Instant::now();
        let mut input_errors: Vec<(usize, String)> = vec![];

        let curve_converted = convert_input(inputs, 0, ValueType::Curve, &mut input_errors);
        let axis_converted = convert_input(inputs, 1, ValueType::Text, &mut input_errors);
        let angle_converted = convert_input(inputs, 2, ValueType::Decimal, &mut input_errors);
        let px_converted = convert_input(inputs, 3, ValueType::Decimal, &mut input_errors);
        let py_converted = convert_input(inputs, 4, ValueType::Decimal, &mut input_errors);

        if !input_errors.is_empty() {
            return Err(OperationError { input_errors, node_error: None });
        }

        let Value::Curve(curve) = curve_converted.unwrap() else { unreachable!() };
        let Value::Text(axis) = axis_converted.unwrap() else { unreachable!() };
        let Value::Decimal(angle) = angle_converted.unwrap() else { unreachable!() };
        let Value::Decimal(pivot_x) = px_converted.unwrap() else { unreachable!() };
        let Value::Decimal(pivot_y) = py_converted.unwrap() else { unreachable!() };

        let out = mirror_curve(&curve, &axis, angle as f64, [pivot_x as f64, pivot_y as f64]);

        Ok(OperationResponse {
            time: Instant::now().duration_since(start_time),
            responses: vec![OutputResponse { value: Value::Curve(out) }],
        })
    }
}
