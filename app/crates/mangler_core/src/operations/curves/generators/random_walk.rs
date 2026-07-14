//! Random walk curve generator.
//!
//! Seeded, heading-based random walk: each step turns the heading by a
//! random amount scaled by `wander` and advances by `step size`, clamped to
//! stay inside the unit square.

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
#[path = "random_walk_tests.rs"]
mod tests;

/// Builds a seeded random-walk polyline of `steps` points starting at
/// `start`. Each step turns the current heading by a uniform random amount
/// in `+-wander * pi` radians, then advances `step_size` along it; every
/// resulting point is clamped to `[0,1]^2`. Deterministic for a given seed;
/// `steps < 1` still returns the single start point (never empty).
pub(crate) fn random_walk_points(seed: i32, start: [f64; 2], steps: usize, step_size: f64, wander: f64) -> Vec<[f32; 2]> {
    // Bit-cast so every integer seed (including 0 and negatives) is a
    // distinct RNG stream, matching meander's convention.
    let mut rng = fastrand::Rng::with_seed(seed as u64);
    let mut points = Vec::with_capacity(steps.max(1));
    points.push(start);
    let mut heading = rng.f64() * std::f64::consts::TAU;
    let turn_scale = wander * std::f64::consts::PI;
    for _ in 1..steps {
        heading += (rng.f64() * 2.0 - 1.0) * turn_scale;
        let last = *points.last().unwrap();
        let next = [
            (last[0] + step_size * heading.cos()).clamp(0.0, 1.0),
            (last[1] + step_size * heading.sin()).clamp(0.0, 1.0),
        ];
        points.push(next);
    }
    points.into_iter().map(|p| [p[0] as f32, p[1] as f32]).collect()
}

/// Operation that generates a seeded random-walk curve.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpCurveGeneratorRandomWalk {}

impl OpCurveGeneratorRandomWalk {
    /// Returns the node metadata (name, description, help).
    pub fn settings() -> NodeSettings {
        NodeSettings {
            name: "random walk".to_string(),
            description: "Generates a seeded random-walk curve.".to_string(),
            help: "Builds an open curve by taking a random walk from a starting point: each step turns the current heading by a random amount (scaled by wander) and advances by step size, clamped to stay inside the [0,1] curve canvas. wander = 0 walks in a fixed random direction (a straight line); higher wander turns more erratically. Deterministic for a given seed - vary the seed for a different walk.\n\nAll positions and the step size are normalized 0-1 curve-space units. Feed the output into smooth or jitter for a less jagged look, or straight into meander for an organic seed shape.".to_string(),
        }
    }

    /// Creates the default inputs: seed, start x/y, steps, step size, wander.
    pub fn create_inputs() -> Vec<Input> {
        vec![
            Input::new("seed".to_string(), Value::Integer(1), Some(InputSettings::DragValue { clamp: None, speed: None }), None)
                .with_description("Random seed for the walk; vary it for a different path from the same start."),
            Input::new("start x".to_string(), Value::Decimal(0.5), Some(InputSettings::Slider { range: (0.0, 1.0), step_by: None, clamp_to_range: false }), None)
                .with_description("Horizontal starting point in normalized [0,1] curve space."),
            Input::new("start y".to_string(), Value::Decimal(0.5), Some(InputSettings::Slider { range: (0.0, 1.0), step_by: None, clamp_to_range: false }), None)
                .with_description("Vertical starting point in normalized [0,1] curve space."),
            Input::new("steps".to_string(), Value::Integer(200), Some(InputSettings::DragValue { clamp: Some((2.0, 2000.0)), speed: None }), None)
                .with_description("Number of points in the walk (minimum 2)."),
            Input::new("step size".to_string(), Value::Decimal(0.01), Some(InputSettings::Slider { range: (0.001, 0.1), step_by: None, clamp_to_range: false }), None)
                .with_description("Distance advanced per step, in normalized units."),
            Input::new("wander".to_string(), Value::Decimal(0.3), Some(InputSettings::Slider { range: (0.0, 1.0), step_by: Some(0.01), clamp_to_range: true }), None)
                .with_description("How erratically the heading turns each step: 0 = straight line, 1 = fully random turns."),
        ]
    }

    /// Creates the default output: a single curve output.
    pub fn create_outputs() -> Vec<Output> {
        vec![
            Output::new("output".to_string(), Value::Curve(Curve::default()), None)
                .with_description("The generated open random-walk curve."),
        ]
    }

    /// Generates the random-walk curve from the given inputs.
    pub async fn run(inputs: &mut [Input]) -> Result<OperationResponse, OperationError> {
        let start_time = Instant::now();
        let mut input_errors: Vec<(usize, String)> = vec![];

        let seed_converted = convert_input(inputs, 0, ValueType::Integer, &mut input_errors);
        let sx_converted = convert_input(inputs, 1, ValueType::Decimal, &mut input_errors);
        let sy_converted = convert_input(inputs, 2, ValueType::Decimal, &mut input_errors);
        let steps_converted = convert_input(inputs, 3, ValueType::Integer, &mut input_errors);
        let step_size_converted = convert_input(inputs, 4, ValueType::Decimal, &mut input_errors);
        let wander_converted = convert_input(inputs, 5, ValueType::Decimal, &mut input_errors);

        if !input_errors.is_empty() {
            return Err(OperationError { input_errors, node_error: None });
        }

        let Value::Integer(seed) = seed_converted.unwrap() else { unreachable!() };
        let Value::Decimal(sx) = sx_converted.unwrap() else { unreachable!() };
        let Value::Decimal(sy) = sy_converted.unwrap() else { unreachable!() };
        let Value::Integer(steps) = steps_converted.unwrap() else { unreachable!() };
        let Value::Decimal(step_size) = step_size_converted.unwrap() else { unreachable!() };
        let Value::Decimal(wander) = wander_converted.unwrap() else { unreachable!() };

        let steps = steps.clamp(2, 2000) as usize;
        let step_size = (step_size as f64).clamp(0.001, 0.1);
        let wander = (wander as f64).clamp(0.0, 1.0);

        let points = random_walk_points(seed, [sx as f64, sy as f64], steps, step_size, wander);
        let curve = linear_curve(points, false);

        Ok(OperationResponse {
            time: Instant::now().duration_since(start_time),
            responses: vec![OutputResponse { value: Value::Curve(curve) }],
        })
    }
}
