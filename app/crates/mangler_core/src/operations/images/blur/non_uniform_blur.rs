//! Non-uniform (variable-radius) blur operation for images.
//!
//! Applies a per-pixel blur where the radius at each pixel is determined by
//! the first channel of a separate blur map. Bright areas in the map
//! get more blur; dark areas stay sharp. Uses a Vogel disc sampling pattern.
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

/// Non-uniform blur operation with per-pixel radius controlled by a grayscale map.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpImageAdjustmentNonUniformBlur {}

impl OpImageAdjustmentNonUniformBlur {
    /// Returns the node metadata (name and description) for the non-uniform blur operation.
    pub fn settings() -> NodeSettings {
        NodeSettings {
            name: "non-uniform blur".to_string(),
            description: "Blurs with per-pixel intensity controlled by a grayscale map.".to_string(),
        }
    }

    /// Creates the input ports: source image, blur map (grayscale radius control),
    /// maximum blur intensity (pixels), and sample count per pixel.
    pub fn create_inputs() -> Vec<Input> {
        vec![
            Input::new("image".to_string(), Value::Image { data: default_image(), change_id: get_id() }, None, None),
            Input::new("blur map".to_string(), Value::Image { data: default_image(), change_id: get_id() }, None, None),
            Input::new("max intensity".to_string(), Value::Decimal(10.0), Some(InputSettings::Slider { range: (0.0, 50.0), step_by: Some(0.5), clamp_to_range: true }), None),
            Input::new("samples".to_string(), Value::Integer(16), Some(InputSettings::DragValue { speed: None, clamp: Some((1.0, 64.0)) }), None),
        ]
    }

    /// Creates the output port: the non-uniformly blurred image.
    pub fn create_outputs() -> Vec<Output> {
        vec![
            Output::new("output".to_string(), Value::Image { data: default_image(), change_id: get_id() }, None),
        ]
    }

    /// Executes the non-uniform blur. Resizes the blur map to match the source image,
    /// generates a Vogel disc sampling pattern, and averages bilinear samples per pixel.
    pub async fn run(inputs: &mut [Input]) -> Result<OperationResponse, OperationError> {
        let start_time = Instant::now();
        let mut input_errors: Vec<(usize, String)> = vec![];

        // convert inputs
        let image_converted = convert_input(inputs, 0, ValueType::Image, &mut input_errors);
        let blur_map_converted = convert_input(inputs, 1, ValueType::Image, &mut input_errors);
        let max_intensity_converted = convert_input(inputs, 2, ValueType::Decimal, &mut input_errors);
        let samples_converted = convert_input(inputs, 3, ValueType::Integer, &mut input_errors);

        // return if error
        if !input_errors.is_empty() { return Err(OperationError { input_errors, node_error: None }); }

        // get values
        let Value::Image { data, change_id: _ } = image_converted.unwrap() else { unreachable!() };
        let Value::Image { data: blur_map_data, change_id: _ } = blur_map_converted.unwrap() else { unreachable!() };
        let Value::Decimal(max_intensity) = max_intensity_converted.unwrap() else { unreachable!() };
        let Value::Integer(samples) = samples_converted.unwrap() else { unreachable!() };

        // run node
        let samples = samples.max(1) as u32;
        let max_intensity = max_intensity.max(0.0);

        let (width, height) = data.dimensions();
        let ch = data.channels() as usize;

        // Resize blur map to match source dimensions if needed
        let blur_map_resized = if blur_map_data.width() != width || blur_map_data.height() != height {
            blur_map_data.resize(width, height)
        } else {
            (*blur_map_data).clone()
        };

        // Precompute concentric disc sample offsets using a Vogel (sunflower) disc pattern
        let mut offsets: Vec<(f32, f32)> = Vec::with_capacity(samples as usize);
        if samples == 1 {
            offsets.push((0.0, 0.0));
        } else {
            // Golden angle in radians: pi * (3 - sqrt(5))
            let golden_angle: f32 = 2.399_963_2;
            for i in 0..samples {
                let r = (i as f32 + 0.5).sqrt() / (samples as f32).sqrt();
                let theta = i as f32 * golden_angle;
                offsets.push((r * theta.cos(), r * theta.sin()));
            }
        }

        let data_ref = &data;
        let blur_map_ref = &blur_map_resized;
        let offsets_ref = &offsets;
        let h = height as usize;
        let w = width as usize;

        // Process each row in parallel
        let pixels: Vec<f32> = (0..h).into_par_iter().flat_map_iter(move |y| {
            // Thread-local sample buffer to avoid per-pixel allocation
            let mut sample = vec![0.0f32; ch];
            let mut row_pixels = Vec::with_capacity(w * ch);

            for x in 0..w {
                // Read the first channel of the blur map as the local blur radius factor
                let map_px = blur_map_ref.get_pixel(x as u32, y as u32);
                let luminance = map_px[0];
                let radius = luminance * max_intensity;

                let mut sums = vec![0.0f64; ch];

                // Sample in a disc pattern scaled by the local radius
                for &(ox, oy) in offsets_ref {
                    let sx = x as f32 + ox * radius;
                    let sy = y as f32 + oy * radius;
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
#[path = "non_uniform_blur_tests.rs"]
mod tests;
