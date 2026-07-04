//! Sine wave pattern generator.
//!
//! Produces a grayscale image of parallel sinusoidal bands at a chosen
//! frequency, orientation, and phase. With an integer frequency and an axis-
//! aligned angle the pattern tiles seamlessly.

use crate::color::color_spaces::rgb_linear::linear_to_nonlinear_srgb;
use crate::float_image::FloatImage;
use crate::get_id;
use crate::input::{Input, InputSettings};
use crate::node_settings::NodeSettings;
use crate::operations::{OperationResponse, OperationError, OutputResponse, default_image, convert_input};
use crate::output::Output;
use crate::value::{Value, ValueType};
use serde::{Deserialize, Serialize};
use std::f32::consts::PI;
use std::sync::Arc;
use std::time::Instant;

/// Sine wave (striped) pattern generator.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpImageNoiseWave {}

impl OpImageNoiseWave {
    /// Returns the node metadata (name and description) for the wave generator.
    pub fn settings() -> NodeSettings {
        NodeSettings {
            name: "wave".to_string(),
            description: "Generates a sine wave stripe pattern at a given frequency, angle, and phase.".to_string(),
            help: "Projects each pixel's normalized coordinate onto the wave direction (set by `angle`) and evaluates 0.5 + 0.5 * sin(2π * frequency * projection + phase). The result is a smooth set of parallel bands; frequency sets how many cycles span the image, angle rotates the bands, and phase slides them along.\n\nWith an integer frequency and a 0°/90° angle the pattern tiles seamlessly. Frequency 0 yields a flat field. Output is a single-channel grayscale image in [0,1] (sRGB-encoded like the other generators).".to_string(),
        }
    }

    /// Creates the default inputs: width, height, frequency, angle, and phase.
    pub fn create_inputs() -> Vec<Input> {
        vec![
            Input::new("width".to_string(), Value::Integer(512), Some(InputSettings::DragValue { clamp: Some((1.0, 10000.0)), speed: None }), None)
                .with_description("Output image width in pixels."),
            Input::new("height".to_string(), Value::Integer(512), Some(InputSettings::DragValue { clamp: Some((1.0, 10000.0)), speed: None }), None)
                .with_description("Output image height in pixels."),
            Input::new("frequency".to_string(), Value::Integer(5), Some(InputSettings::DragValue { clamp: Some((0.0, 1000.0)), speed: None }), None)
                .with_description("Number of wave cycles across the image; integers tile seamlessly."),
            Input::new("angle".to_string(), Value::Decimal(0.0), Some(InputSettings::Slider { range: (0.0, 360.0), step_by: Some(1.0), clamp_to_range: true }), None)
                .with_description("Orientation of the wave bands in degrees."),
            Input::new("phase".to_string(), Value::Decimal(0.0), Some(InputSettings::Slider { range: (0.0, 360.0), step_by: Some(1.0), clamp_to_range: true }), None)
                .with_description("Phase offset of the wave in degrees."),
        ]
    }

    /// Creates the default output: a single grayscale image.
    pub fn create_outputs() -> Vec<Output> {
        vec![
            Output::new("output".to_string(), Value::Image { data: default_image(), change_id: get_id() }, None)
                .with_description("Grayscale sine wave stripe pattern."),
        ]
    }

    /// Generates the wave pattern from the given inputs.
    pub async fn run(inputs: &mut [Input]) -> Result<OperationResponse, OperationError> {
        let start_time = Instant::now();
        let mut input_errors: Vec<(usize, String)> = vec![];

        let width_converted = convert_input(inputs, 0, ValueType::Integer, &mut input_errors);
        let height_converted = convert_input(inputs, 1, ValueType::Integer, &mut input_errors);
        let freq_converted = convert_input(inputs, 2, ValueType::Integer, &mut input_errors);
        let angle_converted = convert_input(inputs, 3, ValueType::Decimal, &mut input_errors);
        let phase_converted = convert_input(inputs, 4, ValueType::Decimal, &mut input_errors);

        if !input_errors.is_empty() { return Err(OperationError { input_errors, node_error: None }); }

        let Value::Integer(mut width) = width_converted.unwrap() else { unreachable!() };
        let Value::Integer(mut height) = height_converted.unwrap() else { unreachable!() };
        let Value::Integer(frequency) = freq_converted.unwrap() else { unreachable!() };
        let Value::Decimal(angle) = angle_converted.unwrap() else { unreachable!() };
        let Value::Decimal(phase) = phase_converted.unwrap() else { unreachable!() };

        width = width.max(1);
        height = height.max(1);
        let freq = frequency.max(0) as f32;
        let (sin_a, cos_a) = angle.to_radians().sin_cos();
        let phase_rad = phase.to_radians();

        let w = width as u32;
        let h = height as u32;
        let mut img = FloatImage::new(w, h, 1);
        for y in 0..h {
            for x in 0..w {
                let nx = x as f32 / w as f32;
                let ny = y as f32 / h as f32;
                let proj = nx * cos_a + ny * sin_a;
                let v = 0.5 + 0.5 * (2.0 * PI * freq * proj + phase_rad).sin();
                img.put_pixel(x, y, &[linear_to_nonlinear_srgb(v)]);
            }
        }

        Ok(OperationResponse {
            time: Instant::now().duration_since(start_time),
            responses: vec![OutputResponse { value: Value::Image { data: Arc::new(img), change_id: get_id() } }],
        })
    }
}

#[cfg(test)]
#[path = "wave_tests.rs"]
mod tests;
