//! Periodic wave curve generator.
//!
//! Displaces a straight A-to-B axis perpendicular to itself in one of three
//! periodic shapes: a dense-sampled sine wave, or an exact-vertex zigzag
//! (symmetric triangle wave) / sawtooth (asymmetric ramp-and-snap).

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
#[path = "wave_tests.rs"]
mod tests;

/// Unit axis (A->B), unit perpendicular (90 degrees clockwise of the axis in
/// this y-down space), and axis length. A degenerate (coincident) A/B
/// returns a length of 0.0 with an arbitrary but valid axis/perpendicular
/// pair, so callers can still fall back to a 2-point curve.
fn wave_axis(start: [f64; 2], end: [f64; 2]) -> ([f64; 2], [f64; 2], f64) {
    let dx = end[0] - start[0];
    let dy = end[1] - start[1];
    let len = (dx * dx + dy * dy).sqrt();
    if len <= 1e-9 {
        return ([1.0, 0.0], [0.0, 1.0], 0.0);
    }
    let u = [dx / len, dy / len];
    let p = [-u[1], u[0]];
    (u, p, len)
}

/// Point at axis-fraction `t` (0 = start, 1 = end) displaced `disp` along the
/// perpendicular.
fn axis_point(start: [f64; 2], u: [f64; 2], p: [f64; 2], len: f64, t: f64, disp: f64) -> [f32; 2] {
    [
        (start[0] + t * len * u[0] + disp * p[0]) as f32,
        (start[1] + t * len * u[1] + disp * p[1]) as f32,
    ]
}

/// Dense sine-wave sampling: `cycles * samples_per_cycle` segments (at least
/// 1) evenly spaced along the axis, displaced by `amplitude * sin(phase)`.
pub(crate) fn sine_wave_points(start: [f64; 2], end: [f64; 2], cycles: f64, amplitude: f64, samples_per_cycle: f64) -> Vec<[f32; 2]> {
    let (u, p, len) = wave_axis(start, end);
    if len <= 0.0 {
        return vec![[start[0] as f32, start[1] as f32], [end[0] as f32, end[1] as f32]];
    }
    let n = ((cycles * samples_per_cycle).round() as usize).max(1);
    (0..=n)
        .map(|i| {
            let t = i as f64 / n as f64;
            let disp = amplitude * (t * cycles * std::f64::consts::TAU).sin();
            axis_point(start, u, p, len, t, disp)
        })
        .collect()
}

/// Exact-vertex symmetric triangle wave: one vertex every quarter-cycle,
/// alternating 0, +amplitude, 0, -amplitude.
pub(crate) fn zigzag_points(start: [f64; 2], end: [f64; 2], cycles: f64, amplitude: f64) -> Vec<[f32; 2]> {
    let (u, p, len) = wave_axis(start, end);
    if len <= 0.0 {
        return vec![[start[0] as f32, start[1] as f32], [end[0] as f32, end[1] as f32]];
    }
    let quarters = ((cycles * 4.0).round() as usize).max(1);
    (0..=quarters)
        .map(|i| {
            let t = i as f64 / quarters as f64;
            let disp = match i % 4 {
                0 => 0.0,
                1 => amplitude,
                2 => 0.0,
                _ => -amplitude,
            };
            axis_point(start, u, p, len, t, disp)
        })
        .collect()
}

/// Exact-vertex sawtooth: a linear ramp from -amplitude to +amplitude across
/// each full cycle, then an instantaneous drop back to -amplitude (a
/// zero-length-along-axis "tooth" segment) before the next ramp.
pub(crate) fn sawtooth_points(start: [f64; 2], end: [f64; 2], cycles: f64, amplitude: f64) -> Vec<[f32; 2]> {
    let (u, p, len) = wave_axis(start, end);
    if len <= 0.0 {
        return vec![[start[0] as f32, start[1] as f32], [end[0] as f32, end[1] as f32]];
    }
    let full = (cycles.round() as usize).max(1);
    let mut out = Vec::with_capacity(2 * full);
    out.push(axis_point(start, u, p, len, 0.0, -amplitude));
    for i in 0..full {
        let t_end = (i + 1) as f64 / full as f64;
        out.push(axis_point(start, u, p, len, t_end, amplitude));
        if i + 1 < full {
            out.push(axis_point(start, u, p, len, t_end, -amplitude));
        }
    }
    out
}

/// Operation that generates a periodic wave curve displaced from a straight
/// A-to-B axis.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpCurveGeneratorWave {}

impl OpCurveGeneratorWave {
    /// Returns the node metadata (name, description, help).
    pub fn settings() -> NodeSettings {
        NodeSettings {
            name: "wave".to_string(),
            description: "Generates a periodic wave curve along a straight axis.".to_string(),
            help: "Displaces a straight line from start to end perpendicular to itself in a periodic pattern: 'sine' is a dense-sampled sine wave, 'zigzag' is an exact-vertex symmetric triangle wave, and 'sawtooth' is an exact-vertex ramp that climbs from -amplitude to +amplitude each cycle then snaps back. cycles counts full periods along the axis; samples per cycle only affects the sine shape (zigzag/sawtooth use their exact breakpoints instead of dense sampling).\n\nAll positions and the amplitude are normalized 0-1 curve-space units. Feed the output into rasterize curve, or into meander for a hand-authored seed shape.".to_string(),
        }
    }

    /// Creates the default inputs: start x/y, end x/y, shape, cycles, amplitude, samples/cycle.
    pub fn create_inputs() -> Vec<Input> {
        vec![
            Input::new("start x".to_string(), Value::Decimal(0.1), Some(InputSettings::Slider { range: (0.0, 1.0), step_by: None, clamp_to_range: false }), None)
                .with_description("Horizontal start of the wave's axis in normalized [0,1] curve space."),
            Input::new("start y".to_string(), Value::Decimal(0.5), Some(InputSettings::Slider { range: (0.0, 1.0), step_by: None, clamp_to_range: false }), None)
                .with_description("Vertical start of the wave's axis in normalized [0,1] curve space."),
            Input::new("end x".to_string(), Value::Decimal(0.9), Some(InputSettings::Slider { range: (0.0, 1.0), step_by: None, clamp_to_range: false }), None)
                .with_description("Horizontal end of the wave's axis in normalized [0,1] curve space."),
            Input::new("end y".to_string(), Value::Decimal(0.5), Some(InputSettings::Slider { range: (0.0, 1.0), step_by: None, clamp_to_range: false }), None)
                .with_description("Vertical end of the wave's axis in normalized [0,1] curve space."),
            Input::new("shape".to_string(), Value::Text("sine".to_string()), Some(InputSettings::Dropdown {
                options: vec!["sine".to_string(), "zigzag".to_string(), "sawtooth".to_string()],
            }), None)
                .with_description("Wave shape: sine (smooth, dense-sampled), zigzag (symmetric triangle wave), or sawtooth (ramp and snap)."),
            Input::new("cycles".to_string(), Value::Decimal(4.0), Some(InputSettings::Slider { range: (0.5, 32.0), step_by: Some(0.1), clamp_to_range: false }), None)
                .with_description("Number of full periods along the axis."),
            Input::new("amplitude".to_string(), Value::Decimal(0.1), Some(InputSettings::Slider { range: (0.0, 0.5), step_by: None, clamp_to_range: false }), None)
                .with_description("Perpendicular displacement amplitude in normalized units."),
            Input::new("samples per cycle".to_string(), Value::Integer(16), Some(InputSettings::DragValue { clamp: Some((4.0, 64.0)), speed: None }), None)
                .with_description("Sample density per cycle for the sine shape only; zigzag and sawtooth use their exact breakpoints instead."),
        ]
    }

    /// Creates the default output: a single curve output.
    pub fn create_outputs() -> Vec<Output> {
        vec![
            Output::new("output".to_string(), Value::Curve(Curve::default()), None)
                .with_description("The generated open wave curve."),
        ]
    }

    /// Generates the wave curve from the given inputs.
    pub async fn run(inputs: &mut [Input]) -> Result<OperationResponse, OperationError> {
        let start_time = Instant::now();
        let mut input_errors: Vec<(usize, String)> = vec![];

        let sx_converted = convert_input(inputs, 0, ValueType::Decimal, &mut input_errors);
        let sy_converted = convert_input(inputs, 1, ValueType::Decimal, &mut input_errors);
        let ex_converted = convert_input(inputs, 2, ValueType::Decimal, &mut input_errors);
        let ey_converted = convert_input(inputs, 3, ValueType::Decimal, &mut input_errors);
        let shape_converted = convert_input(inputs, 4, ValueType::Text, &mut input_errors);
        let cycles_converted = convert_input(inputs, 5, ValueType::Decimal, &mut input_errors);
        let amplitude_converted = convert_input(inputs, 6, ValueType::Decimal, &mut input_errors);
        let samples_converted = convert_input(inputs, 7, ValueType::Integer, &mut input_errors);

        if !input_errors.is_empty() {
            return Err(OperationError { input_errors, node_error: None });
        }

        let Value::Decimal(sx) = sx_converted.unwrap() else { unreachable!() };
        let Value::Decimal(sy) = sy_converted.unwrap() else { unreachable!() };
        let Value::Decimal(ex) = ex_converted.unwrap() else { unreachable!() };
        let Value::Decimal(ey) = ey_converted.unwrap() else { unreachable!() };
        let Value::Text(shape) = shape_converted.unwrap() else { unreachable!() };
        let Value::Decimal(cycles) = cycles_converted.unwrap() else { unreachable!() };
        let Value::Decimal(amplitude) = amplitude_converted.unwrap() else { unreachable!() };
        let Value::Integer(samples_per_cycle) = samples_converted.unwrap() else { unreachable!() };

        let start = [sx as f64, sy as f64];
        let end = [ex as f64, ey as f64];
        let cycles = (cycles as f64).clamp(0.5, 32.0);
        let amplitude = (amplitude as f64).clamp(0.0, 0.5);
        let samples_per_cycle = (samples_per_cycle.clamp(4, 64)) as f64;

        let points = match shape.as_str() {
            "zigzag" => zigzag_points(start, end, cycles, amplitude),
            "sawtooth" => sawtooth_points(start, end, cycles, amplitude),
            _ => sine_wave_points(start, end, cycles, amplitude, samples_per_cycle),
        };
        let curve = linear_curve(points, false);

        Ok(OperationResponse {
            time: Instant::now().duration_since(start_time),
            responses: vec![OutputResponse { value: Value::Curve(curve) }],
        })
    }
}
