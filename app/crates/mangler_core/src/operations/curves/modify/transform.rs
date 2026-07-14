//! Curve transform modifier: translate, rotate, and scale.
//!
//! Structure-preserving — operates directly on `points` and `handles` (as
//! vectors) and keeps `interpolation` and `closed` unchanged. Rotation and
//! scale pivot about the arc-length-weighted centroid of the flattened
//! polyline (not the average of the control points, which would be skewed by
//! point density); translation is then applied on top.

use crate::curve::Curve;
use crate::input::{Input, InputSettings};
use crate::node_settings::NodeSettings;
use crate::operations::curves::common::{dist, flatten_f64};
use crate::operations::{convert_input, OperationError, OperationResponse, OutputResponse};
use crate::output::Output;
use crate::value::{Value, ValueType};
use serde::{Deserialize, Serialize};
use std::time::Instant;

#[cfg(test)]
#[path = "transform_tests.rs"]
mod tests;

/// Arc-length-weighted centroid of a polyline: each segment's midpoint
/// weighted by its length. Falls back to the arithmetic mean of the points
/// when the polyline has effectively zero length (all points coincident).
pub(crate) fn arc_length_centroid(poly: &[[f64; 2]]) -> [f64; 2] {
    let mut cx = 0.0;
    let mut cy = 0.0;
    let mut total = 0.0;
    for seg in poly.windows(2) {
        let l = dist(seg[0], seg[1]);
        cx += (seg[0][0] + seg[1][0]) * 0.5 * l;
        cy += (seg[0][1] + seg[1][1]) * 0.5 * l;
        total += l;
    }
    if total <= 1e-12 {
        let n = poly.len().max(1) as f64;
        let (sx, sy) = poly.iter().fold((0.0, 0.0), |(ax, ay), p| (ax + p[0], ay + p[1]));
        return [sx / n, sy / n];
    }
    [cx / total, cy / total]
}

/// Rotates `v` by `theta` radians (positive = clockwise in this y-down
/// space), matching the generator nodes' `[x*c - y*s, x*s + y*c]` convention.
fn rotate(v: [f64; 2], c: f64, s: f64) -> [f64; 2] {
    [v[0] * c - v[1] * s, v[0] * s + v[1] * c]
}

/// Translates/rotates/scales `curve`'s points, and rotates/scales (but does
/// not translate) its handle vectors. Rotation and scale pivot about the
/// arc-length-weighted centroid of the flattened polyline. Fewer than 2
/// points passes through unchanged.
pub(crate) fn transform_curve(curve: &Curve, offset: [f64; 2], rotation_deg: f64, scale: [f64; 2]) -> Curve {
    if curve.points.len() < 2 {
        return curve.clone();
    }
    let poly = flatten_f64(curve, 48);
    let centroid = arc_length_centroid(&poly);
    let theta = rotation_deg.to_radians();
    let (s, c) = theta.sin_cos();

    let points = curve
        .points
        .iter()
        .map(|p| {
            let v = [
                (p[0] as f64 - centroid[0]) * scale[0],
                (p[1] as f64 - centroid[1]) * scale[1],
            ];
            let r = rotate(v, c, s);
            [
                (centroid[0] + r[0] + offset[0]) as f32,
                (centroid[1] + r[1] + offset[1]) as f32,
            ]
        })
        .collect();

    let handles = curve
        .handles
        .iter()
        .map(|h| {
            let v = [h[0] as f64 * scale[0], h[1] as f64 * scale[1]];
            let r = rotate(v, c, s);
            [r[0] as f32, r[1] as f32]
        })
        .collect();

    Curve { points, closed: curve.closed, interpolation: curve.interpolation, handles }
}

/// Operation that translates, rotates, and scales a curve about its centroid.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpCurveModifyTransform {}

impl OpCurveModifyTransform {
    /// Returns the node metadata (name, description, help).
    pub fn settings() -> NodeSettings {
        NodeSettings {
            name: "transform".to_string(),
            description: "Translates, rotates, and scales a curve.".to_string(),
            help: "Structure-preserving: moves the curve's control points (and, in Bezier mode, its tangent handles as vectors) without flattening or resampling — point count, interpolation mode, and open/closed state are all unchanged. Rotation and scale pivot about the arc-length-weighted centroid of the flattened curve (not the raw average of control points, which would skew toward denser regions); translation is applied afterward. Rotation is in degrees, positive = clockwise. Scale x/y of 1 leaves that axis unchanged; non-uniform scale (x != y) also reshapes the tangent handles, so a circle scaled non-uniformly becomes an ellipse rather than staying circular.\n\nOffsets and the centroid math are in normalized 0-1 curve-space units.".to_string(),
        }
    }

    /// Creates the default inputs: curve, offset x/y, rotation, scale x/y.
    pub fn create_inputs() -> Vec<Input> {
        vec![
            Input::new("curve".to_string(), Value::Curve(Curve::default()), None, None)
                .with_description("The curve to transform."),
            Input::new("offset x".to_string(), Value::Decimal(0.0), Some(InputSettings::Slider { range: (-1.0, 1.0), step_by: None, clamp_to_range: false }), None)
                .with_description("Horizontal translation in normalized curve-space units, applied after rotation/scale."),
            Input::new("offset y".to_string(), Value::Decimal(0.0), Some(InputSettings::Slider { range: (-1.0, 1.0), step_by: None, clamp_to_range: false }), None)
                .with_description("Vertical translation in normalized curve-space units, applied after rotation/scale."),
            Input::new("rotation".to_string(), Value::Decimal(0.0), Some(InputSettings::Slider { range: (-360.0, 360.0), step_by: Some(0.1), clamp_to_range: false }), None)
                .with_description("Rotation in degrees about the curve's arc-length-weighted centroid; positive is clockwise."),
            Input::new("scale x".to_string(), Value::Decimal(1.0), Some(InputSettings::DragValue { clamp: Some((0.01, 100.0)), speed: Some(0.01) }), None)
                .with_description("Horizontal scale about the centroid; 1 = unchanged."),
            Input::new("scale y".to_string(), Value::Decimal(1.0), Some(InputSettings::DragValue { clamp: Some((0.01, 100.0)), speed: Some(0.01) }), None)
                .with_description("Vertical scale about the centroid; 1 = unchanged."),
        ]
    }

    /// Creates the default output: a single curve output.
    pub fn create_outputs() -> Vec<Output> {
        vec![
            Output::new("output".to_string(), Value::Curve(Curve::default()), None)
                .with_description("The transformed curve."),
        ]
    }

    /// Transforms the curve from the given inputs.
    pub async fn run(inputs: &mut [Input]) -> Result<OperationResponse, OperationError> {
        let start_time = Instant::now();
        let mut input_errors: Vec<(usize, String)> = vec![];

        let curve_converted = convert_input(inputs, 0, ValueType::Curve, &mut input_errors);
        let ox_converted = convert_input(inputs, 1, ValueType::Decimal, &mut input_errors);
        let oy_converted = convert_input(inputs, 2, ValueType::Decimal, &mut input_errors);
        let rot_converted = convert_input(inputs, 3, ValueType::Decimal, &mut input_errors);
        let sx_converted = convert_input(inputs, 4, ValueType::Decimal, &mut input_errors);
        let sy_converted = convert_input(inputs, 5, ValueType::Decimal, &mut input_errors);

        if !input_errors.is_empty() {
            return Err(OperationError { input_errors, node_error: None });
        }

        let Value::Curve(curve) = curve_converted.unwrap() else { unreachable!() };
        let Value::Decimal(offset_x) = ox_converted.unwrap() else { unreachable!() };
        let Value::Decimal(offset_y) = oy_converted.unwrap() else { unreachable!() };
        let Value::Decimal(rotation) = rot_converted.unwrap() else { unreachable!() };
        let Value::Decimal(scale_x) = sx_converted.unwrap() else { unreachable!() };
        let Value::Decimal(scale_y) = sy_converted.unwrap() else { unreachable!() };

        let out = transform_curve(
            &curve,
            [offset_x as f64, offset_y as f64],
            rotation as f64,
            [scale_x as f64, scale_y as f64],
        );

        Ok(OperationResponse {
            time: Instant::now().duration_since(start_time),
            responses: vec![OutputResponse { value: Value::Curve(out) }],
        })
    }
}
