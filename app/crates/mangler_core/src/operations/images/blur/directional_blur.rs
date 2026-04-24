//! Directional blur operation for images.
//!
//! Blurs the image along a specified angle by averaging multiple bilinearly
//! sampled points distributed along a line centered at each pixel.
//! Works directly on [`FloatImage`] f32 data.

use crate::float_image::FloatImage;
use crate::get_id;
use crate::value::ValueType;
use crate::input::{Input, InputSettings};
use crate::node_settings::NodeSettings;
use crate::operations::{OperationResponse, OperationError, OutputResponse, default_image, convert_input};
use crate::output::Output;
use crate::value::Value;
use rayon::prelude::*;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use std::time::Instant;

/// Directional blur operation that smears the image along a specified angle.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpImageAdjustmentDirectionalBlur {}

impl OpImageAdjustmentDirectionalBlur {
    /// Returns the node metadata (name and description) for the directional blur operation.
    pub fn settings() -> NodeSettings {
        NodeSettings {
            name: "directional blur".to_string(),
            description: "Blurs an image along a specified angle.".to_string(),
            help: "Samples the image at equally spaced positions along a line of length 2 * intensity centred on each output pixel, then averages them. The line direction is (cos(angle), sin(angle)) with angle in degrees counter-clockwise from +X, so 0 smears horizontally and 90 smears vertically.\n\nSamples are bilinear so sub-pixel offsets produce smooth results. Higher sample counts yield smoother motion trails but cost linearly more work. Work is parallelised across rows via rayon. Intensity 0 or one sample returns the image unchanged (each tap lands on the centre pixel).".to_string(),
        }
    }

    /// Creates the input ports: image, angle (degrees), sample count, and intensity (pixel spread).
    pub fn create_inputs() -> Vec<Input> {
        vec![
            Input::new("image".to_string(), Value::Image { data: default_image(), change_id: get_id() }, None, None)
                .with_description("Source image to smear along a line."),
            Input::new("angle".to_string(), Value::Decimal(0.0), Some(InputSettings::Slider { range: (0.0, 360.0), step_by: Some(1.0), clamp_to_range: true }), None)
                .with_description("Direction of the blur line in degrees, measured counter-clockwise from +X."),
            Input::new("samples".to_string(), Value::Integer(10), Some(InputSettings::DragValue { speed: None, clamp: Some((1.0, 100.0)) }), None)
                .with_description("Number of taps averaged along the blur line; higher values are smoother but slower."),
            Input::new("intensity".to_string(), Value::Decimal(10.0), Some(InputSettings::Slider { range: (0.0, 100.0), step_by: Some(0.5), clamp_to_range: true }), None)
                .with_description("Half-length of the blur line in pixels."),
        ]
    }

    /// Creates the output port: the directionally blurred image.
    pub fn create_outputs() -> Vec<Output> {
        vec![
            Output::new("output".to_string(), Value::Image { data: default_image(), change_id: get_id() }, None)
                .with_description("Image smeared along the configured direction."),
        ]
    }

    /// Executes the directional blur. Samples are distributed symmetrically along the
    /// angle direction using FloatImage's bilinear interpolation.
    pub async fn run(inputs: &mut [Input]) -> Result<OperationResponse, OperationError> {
        let start_time = Instant::now();
        let mut input_errors: Vec<(usize, String)> = vec![];

        // convert inputs
        let image_converted = convert_input(inputs, 0, ValueType::Image, &mut input_errors);
        let angle_converted = convert_input(inputs, 1, ValueType::Decimal, &mut input_errors);
        let samples_converted = convert_input(inputs, 2, ValueType::Integer, &mut input_errors);
        let intensity_converted = convert_input(inputs, 3, ValueType::Decimal, &mut input_errors);

        // return if error
        if !input_errors.is_empty() { return Err(OperationError { input_errors, node_error: None }); }

        // get values
        let Value::Image { data, change_id: _ } = image_converted.unwrap() else { unreachable!() };
        let Value::Decimal(angle) = angle_converted.unwrap() else { unreachable!() };
        let Value::Integer(samples) = samples_converted.unwrap() else { unreachable!() };
        let Value::Decimal(intensity) = intensity_converted.unwrap() else { unreachable!() };

        // run node
        let samples = samples.max(1) as u32;
        let intensity = intensity.max(0.0);
        let angle_rad = angle.to_radians();
        let dx = angle_rad.cos();
        let dy = angle_rad.sin();

        let (width, height) = data.dimensions();
        let ch = data.channels() as usize;
        let data_ref = &data;
        let h = height as usize;
        let w = width as usize;

        // Process each row in parallel, accumulating bilinear samples per pixel
        let pixels: Vec<f32> = (0..h).into_par_iter().flat_map_iter(move |y| {
            // Thread-local sample buffer to avoid per-pixel allocation
            let mut sample = vec![0.0f32; ch];
            let mut row_pixels = Vec::with_capacity(w * ch);

            for x in 0..w {
                let mut sums = vec![0.0f64; ch];

                // Sample along the blur direction, centered on this pixel
                for i in 0..samples {
                    let t = if samples > 1 {
                        (i as f32 / (samples - 1) as f32) * 2.0 - 1.0
                    } else {
                        0.0
                    };
                    let offset = t * intensity;
                    let sx = x as f32 + dx * offset;
                    let sy = y as f32 + dy * offset;
                    data_ref.bilinear_sample(sx, sy, &mut sample);
                    for c in 0..ch {
                        sums[c] += sample[c] as f64;
                    }
                }

                // Average across all samples
                let count = samples as f64;
                for c in 0..ch {
                    row_pixels.push((sums[c] / count) as f32);
                }
            }
            row_pixels
        }).collect();

        // Build the output FloatImage from the computed pixel buffer
        let output = FloatImage::from_raw(width, height, data.channels(), pixels).unwrap();

        Ok(OperationResponse { 
            time: Instant::now().duration_since(start_time),
            responses: vec![
                OutputResponse { value: Value::Image { data: Arc::new(output), change_id: get_id() } },
            ],
        })
    }
}

#[cfg(test)]
#[path = "directional_blur_tests.rs"]
mod tests;
