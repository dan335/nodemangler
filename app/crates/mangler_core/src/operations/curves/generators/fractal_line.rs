//! Fractal (midpoint-displacement) line curve generator.
//!
//! Classic 1D midpoint-displacement fractal: each subdivision level inserts
//! a midpoint between every existing pair of points, displaced
//! perpendicular to that segment by a seeded random amount that halves
//! every level. Endpoints are never touched, so they stay exact.

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
#[path = "fractal_line_tests.rs"]
mod tests;

/// Builds a seeded midpoint-displacement fractal polyline from `start` to
/// `end` with `detail` subdivision levels (>=1), yielding exactly
/// `2^detail + 1` points. Displacement at level 0 is `roughness *
/// dist(start, end) * 0.5`, halving each subsequent level; endpoints are
/// copied through unchanged at every level, so they are always exact.
pub(crate) fn fractal_line_points(seed: i32, start: [f64; 2], end: [f64; 2], detail: u32, roughness: f64) -> Vec<[f32; 2]> {
    let mut rng = fastrand::Rng::with_seed(seed as u64);
    let base_len = {
        let dx = end[0] - start[0];
        let dy = end[1] - start[1];
        (dx * dx + dy * dy).sqrt()
    };
    let mut points: Vec<[f64; 2]> = vec![start, end];
    let mut disp_scale = roughness * base_len * 0.5;

    for _ in 0..detail {
        let mut next = Vec::with_capacity(points.len() * 2 - 1);
        for i in 0..points.len() - 1 {
            let a = points[i];
            let b = points[i + 1];
            next.push(a);
            let dx = b[0] - a[0];
            let dy = b[1] - a[1];
            let len = (dx * dx + dy * dy).sqrt();
            let perp = if len > 1e-12 { [-dy / len, dx / len] } else { [0.0, 1.0] };
            let mid = [(a[0] + b[0]) * 0.5, (a[1] + b[1]) * 0.5];
            let d = (rng.f64() * 2.0 - 1.0) * disp_scale;
            next.push([mid[0] + d * perp[0], mid[1] + d * perp[1]]);
        }
        next.push(*points.last().unwrap());
        points = next;
        disp_scale *= 0.5;
    }

    points.into_iter().map(|p| [p[0] as f32, p[1] as f32]).collect()
}

/// Operation that generates a seeded fractal (midpoint-displacement) line curve.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpCurveGeneratorFractalLine {}

impl OpCurveGeneratorFractalLine {
    /// Returns the node metadata (name, description, help).
    pub fn settings() -> NodeSettings {
        NodeSettings {
            name: "fractal line".to_string(),
            description: "Generates a seeded midpoint-displacement fractal line.".to_string(),
            help: "Builds a jagged, self-similar line between two points using midpoint displacement: each subdivision level (detail) inserts a midpoint between every existing pair of points, displaced perpendicular to that segment by a seeded random amount that halves every level - the classic 1D fractal terrain algorithm. roughness scales the initial displacement (relative to the start-end distance); higher detail adds finer wiggles without changing the overall large-scale shape. Endpoints are never displaced, so they land exactly on start and end. Deterministic for a given seed.\n\nAll positions are normalized 0-1 curve-space units. Feed the output into meander for a wiggly river seed, or into smooth to soften the jaggedness.".to_string(),
        }
    }

    /// Creates the default inputs: seed, start x/y, end x/y, detail, roughness.
    pub fn create_inputs() -> Vec<Input> {
        vec![
            Input::new("seed".to_string(), Value::Integer(1), Some(InputSettings::DragValue { clamp: None, speed: None }), None)
                .with_description("Random seed for the displacement pattern; vary it for a different line between the same endpoints."),
            Input::new("start x".to_string(), Value::Decimal(0.1), Some(InputSettings::Slider { range: (0.0, 1.0), step_by: None, clamp_to_range: false }), None)
                .with_description("Horizontal start point in normalized [0,1] curve space."),
            Input::new("start y".to_string(), Value::Decimal(0.5), Some(InputSettings::Slider { range: (0.0, 1.0), step_by: None, clamp_to_range: false }), None)
                .with_description("Vertical start point in normalized [0,1] curve space."),
            Input::new("end x".to_string(), Value::Decimal(0.9), Some(InputSettings::Slider { range: (0.0, 1.0), step_by: None, clamp_to_range: false }), None)
                .with_description("Horizontal end point in normalized [0,1] curve space."),
            Input::new("end y".to_string(), Value::Decimal(0.5), Some(InputSettings::Slider { range: (0.0, 1.0), step_by: None, clamp_to_range: false }), None)
                .with_description("Vertical end point in normalized [0,1] curve space."),
            Input::new("detail".to_string(), Value::Integer(6), Some(InputSettings::DragValue { clamp: Some((1.0, 10.0)), speed: None }), None)
                .with_description("Number of subdivision levels (1-10); the output has 2^detail + 1 points."),
            Input::new("roughness".to_string(), Value::Decimal(0.5), Some(InputSettings::Slider { range: (0.0, 1.0), step_by: Some(0.01), clamp_to_range: true }), None)
                .with_description("Initial displacement scale, relative to the start-end distance; halves every subdivision level."),
        ]
    }

    /// Creates the default output: a single curve output.
    pub fn create_outputs() -> Vec<Output> {
        vec![
            Output::new("output".to_string(), Value::Curve(Curve::default()), None)
                .with_description("The generated open fractal line curve."),
        ]
    }

    /// Generates the fractal line curve from the given inputs.
    pub async fn run(inputs: &mut [Input]) -> Result<OperationResponse, OperationError> {
        let start_time = Instant::now();
        let mut input_errors: Vec<(usize, String)> = vec![];

        let seed_converted = convert_input(inputs, 0, ValueType::Integer, &mut input_errors);
        let sx_converted = convert_input(inputs, 1, ValueType::Decimal, &mut input_errors);
        let sy_converted = convert_input(inputs, 2, ValueType::Decimal, &mut input_errors);
        let ex_converted = convert_input(inputs, 3, ValueType::Decimal, &mut input_errors);
        let ey_converted = convert_input(inputs, 4, ValueType::Decimal, &mut input_errors);
        let detail_converted = convert_input(inputs, 5, ValueType::Integer, &mut input_errors);
        let roughness_converted = convert_input(inputs, 6, ValueType::Decimal, &mut input_errors);

        if !input_errors.is_empty() {
            return Err(OperationError { input_errors, node_error: None });
        }

        let Value::Integer(seed) = seed_converted.unwrap() else { unreachable!() };
        let Value::Decimal(sx) = sx_converted.unwrap() else { unreachable!() };
        let Value::Decimal(sy) = sy_converted.unwrap() else { unreachable!() };
        let Value::Decimal(ex) = ex_converted.unwrap() else { unreachable!() };
        let Value::Decimal(ey) = ey_converted.unwrap() else { unreachable!() };
        let Value::Integer(detail) = detail_converted.unwrap() else { unreachable!() };
        let Value::Decimal(roughness) = roughness_converted.unwrap() else { unreachable!() };

        let detail = detail.clamp(1, 10) as u32;
        let roughness = (roughness as f64).clamp(0.0, 1.0);

        let points = fractal_line_points(seed, [sx as f64, sy as f64], [ex as f64, ey as f64], detail, roughness);
        let curve = linear_curve(points, false);

        Ok(OperationResponse {
            time: Instant::now().duration_since(start_time),
            responses: vec![OutputResponse { value: Value::Curve(curve) }],
        })
    }
}
