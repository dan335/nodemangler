//! Ellipse curve generator.
//!
//! Builds a closed ellipse as 4 cubic-Bezier anchors at the rotated cardinal
//! points, with mirrored tangent handles sized so the standard 4-span circle
//! approximation is within ~0.03% of the true radius — hand-editable in the
//! curve overlay afterward, unlike a dense polyline.

use crate::curve::{Curve, CurveInterpolation};
use crate::input::{Input, InputSettings};
use crate::node_settings::NodeSettings;
use crate::operations::{convert_input, OperationError, OperationResponse, OutputResponse};
use crate::output::Output;
use crate::value::{Value, ValueType};
use serde::{Deserialize, Serialize};
use std::time::Instant;

#[cfg(test)]
#[path = "ellipse_tests.rs"]
mod tests;

/// Magic constant for approximating a quarter-circle arc with a single cubic
/// Bezier span: `(4/3) * tan(pi/8)`.
pub(crate) fn bezier_quarter_k() -> f64 {
    (4.0 / 3.0) * (std::f64::consts::FRAC_PI_8).tan()
}

/// Builds a closed 4-anchor Bezier ellipse centered at `(cx, cy)` with radii
/// `(rx, ry)`, rotated `rotation_deg` degrees about its center.
///
/// Anchors sit at the rotated cardinal points (right, bottom, left, top in
/// the curve's y-down frame); each anchor's mirrored tangent handle is the
/// unit tangent at that anchor times the *other* axis's radius times
/// [`bezier_quarter_k`] — the standard construction that makes a circle
/// (`rx == ry`) match the true radius everywhere to within ~0.03%.
pub(crate) fn ellipse_curve(cx: f64, cy: f64, rx: f64, ry: f64, rotation_deg: f64) -> Curve {
    let rot = rotation_deg.to_radians();
    let (s, c) = rot.sin_cos();
    let k = bezier_quarter_k();
    let rotate = |p: [f64; 2]| [p[0] * c - p[1] * s, p[0] * s + p[1] * c];
    // (local anchor point, local forward-tangent handle vector) at angles
    // 0/90/180/270 degrees (right, bottom, left, top in y-down space).
    let locals: [([f64; 2], [f64; 2]); 4] = [
        ([rx, 0.0], [0.0, ry * k]),
        ([0.0, ry], [-rx * k, 0.0]),
        ([-rx, 0.0], [0.0, -ry * k]),
        ([0.0, -ry], [rx * k, 0.0]),
    ];
    let mut points = Vec::with_capacity(4);
    let mut handles = Vec::with_capacity(4);
    for (p, h) in locals {
        let pr = rotate(p);
        points.push([(cx + pr[0]) as f32, (cy + pr[1]) as f32]);
        let hr = rotate(h);
        handles.push([hr[0] as f32, hr[1] as f32]);
    }
    Curve { points, closed: true, interpolation: CurveInterpolation::Bezier, handles }
}

/// Operation that generates a closed ellipse curve.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpCurveGeneratorEllipse {}

impl OpCurveGeneratorEllipse {
    /// Returns the node metadata (name, description, help).
    pub fn settings() -> NodeSettings {
        NodeSettings {
            name: "ellipse".to_string(),
            description: "Generates a closed ellipse curve.".to_string(),
            help: "Builds a closed ellipse (or circle, when radius x equals radius y) as 4 cubic-Bezier anchors at the rotated cardinal points, with mirrored tangent handles sized by the standard 4-span circle approximation — accurate to within about 0.03% of the true radius, and hand-editable afterward in the curve overlay (drag an anchor or handle like any other Bezier curve).\n\nAll positions and radii are normalized 0-1 curve-space units (independent of any raster size); feed the output into rasterize curve to bake it into a mask, or into meander to use it as a seed river shape.".to_string(),
        }
    }

    /// Creates the default inputs: center x/y, radius x/y, rotation.
    pub fn create_inputs() -> Vec<Input> {
        vec![
            Input::new("center x".to_string(), Value::Decimal(0.5), Some(InputSettings::Slider { range: (0.0, 1.0), step_by: None, clamp_to_range: false }), None)
                .with_description("Horizontal center of the ellipse in normalized [0,1] curve space."),
            Input::new("center y".to_string(), Value::Decimal(0.5), Some(InputSettings::Slider { range: (0.0, 1.0), step_by: None, clamp_to_range: false }), None)
                .with_description("Vertical center of the ellipse in normalized [0,1] curve space."),
            Input::new("radius x".to_string(), Value::Decimal(0.3), Some(InputSettings::Slider { range: (0.0, 1.0), step_by: None, clamp_to_range: false }), None)
                .with_description("Horizontal radius in normalized units. Equal to radius y produces a circle."),
            Input::new("radius y".to_string(), Value::Decimal(0.3), Some(InputSettings::Slider { range: (0.0, 1.0), step_by: None, clamp_to_range: false }), None)
                .with_description("Vertical radius in normalized units. Equal to radius x produces a circle."),
            Input::new("rotation".to_string(), Value::Decimal(0.0), Some(InputSettings::Slider { range: (0.0, 360.0), step_by: None, clamp_to_range: false }), None)
                .with_description("Rotation of the ellipse about its center, in degrees."),
        ]
    }

    /// Creates the default output: a single curve output.
    pub fn create_outputs() -> Vec<Output> {
        vec![
            Output::new("output".to_string(), Value::Curve(Curve::default()), None)
                .with_description("The generated closed ellipse curve."),
        ]
    }

    /// Generates the ellipse curve from the given inputs.
    pub async fn run(inputs: &mut [Input]) -> Result<OperationResponse, OperationError> {
        let start_time = Instant::now();
        let mut input_errors: Vec<(usize, String)> = vec![];

        let cx_converted = convert_input(inputs, 0, ValueType::Decimal, &mut input_errors);
        let cy_converted = convert_input(inputs, 1, ValueType::Decimal, &mut input_errors);
        let rx_converted = convert_input(inputs, 2, ValueType::Decimal, &mut input_errors);
        let ry_converted = convert_input(inputs, 3, ValueType::Decimal, &mut input_errors);
        let rotation_converted = convert_input(inputs, 4, ValueType::Decimal, &mut input_errors);

        if !input_errors.is_empty() {
            return Err(OperationError { input_errors, node_error: None });
        }

        let Value::Decimal(cx) = cx_converted.unwrap() else { unreachable!() };
        let Value::Decimal(cy) = cy_converted.unwrap() else { unreachable!() };
        let Value::Decimal(rx) = rx_converted.unwrap() else { unreachable!() };
        let Value::Decimal(ry) = ry_converted.unwrap() else { unreachable!() };
        let Value::Decimal(rotation) = rotation_converted.unwrap() else { unreachable!() };

        // Floor radii so a zero/negative input still produces a valid,
        // vanishingly small (but non-degenerate) ellipse.
        let rx = (rx as f64).max(0.001);
        let ry = (ry as f64).max(0.001);

        let curve = ellipse_curve(cx as f64, cy as f64, rx, ry, rotation as f64);

        Ok(OperationResponse {
            time: Instant::now().duration_since(start_time),
            responses: vec![OutputResponse { value: Value::Curve(curve) }],
        })
    }
}
