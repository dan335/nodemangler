//! Slope blur operation for images.
//!
//! Blurs the image along directions derived from the gradient of a separate
//! grayscale slope map. The gradient direction at each pixel determines the
//! blur direction, creating an effect similar to paint being smeared downhill.
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

/// Slope blur operation that blurs along gradient directions derived from a slope map.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpImageAdjustmentSlopeBlur {}

impl OpImageAdjustmentSlopeBlur {
    /// Returns the node metadata (name and description) for the slope blur operation.
    pub fn settings() -> NodeSettings {
        NodeSettings {
            name: "slope blur".to_string(),
            description: "Blurs along directions determined by a grayscale slope map.".to_string(),
            help: "At each pixel, computes the 2D gradient of the slope map via central finite differences (first channel used as height), normalises it, and then blurs the source along that unit direction using bilinear samples spaced over +/- intensity pixels. Flat regions of the slope map (gradient near zero) leave pixels untouched.\n\nThe slope map is resized to match the source if needed. The effect resembles wet paint running downhill when the slope map is a heightfield, and is a staple for weathering, drip, and anisotropic smear effects. Parallelised across rows via rayon.".to_string(),
        }
    }

    /// Creates the input ports: source image, slope map (grayscale gradient source),
    /// intensity (pixel spread), and number of samples.
    pub fn create_inputs() -> Vec<Input> {
        vec![
            Input::new("image".to_string(), Value::Image { data: default_image(), change_id: get_id() }, None, None)
                .with_description("Source image to smear along gradient directions."),
            Input::new("slope map".to_string(), Value::Image { data: default_image(), change_id: get_id() }, None, None)
                .with_description("Grayscale heightfield; its gradient gives the blur direction per pixel."),
            Input::new("intensity".to_string(), Value::Decimal(10.0), Some(InputSettings::Slider { range: (0.0, 100.0), step_by: Some(0.5), clamp_to_range: true }), None)
                .with_description("Half-length in pixels of the line sampled along each gradient."),
            Input::new("samples".to_string(), Value::Integer(10), Some(InputSettings::DragValue { speed: None, clamp: Some((1.0, 100.0)) }), None)
                .with_description("Number of taps averaged along the gradient direction."),
        ]
    }

    /// Creates the output port: the slope-blurred image.
    pub fn create_outputs() -> Vec<Output> {
        vec![
            Output::new("output".to_string(), Value::Image { data: default_image(), change_id: get_id() }, None)
                .with_description("Image smeared along the slope-map gradient at each pixel."),
        ]
    }

    /// Executes the slope blur. Computes per-pixel gradient direction from the slope map
    /// using finite differences, then averages bilinear samples along that direction.
    pub async fn run(inputs: &mut [Input]) -> Result<OperationResponse, OperationError> {
        let start_time = Instant::now();
        let mut input_errors: Vec<(usize, String)> = vec![];

        // convert inputs
        let image_converted = convert_input(inputs, 0, ValueType::Image, &mut input_errors);
        let slope_map_converted = convert_input(inputs, 1, ValueType::Image, &mut input_errors);
        let intensity_converted = convert_input(inputs, 2, ValueType::Decimal, &mut input_errors);
        let samples_converted = convert_input(inputs, 3, ValueType::Integer, &mut input_errors);

        // return if error
        if !input_errors.is_empty() { return Err(OperationError { input_errors, node_error: None }); }

        // get values
        let Value::Image { data, change_id: _ } = image_converted.unwrap() else { unreachable!() };
        let Value::Image { data: slope_data, change_id: _ } = slope_map_converted.unwrap() else { unreachable!() };
        let Value::Decimal(intensity) = intensity_converted.unwrap() else { unreachable!() };
        let Value::Integer(samples) = samples_converted.unwrap() else { unreachable!() };

        // run node
        let samples = samples.max(1) as u32;
        let intensity = intensity.max(0.0);

        let (width, height) = data.dimensions();
        let ch = data.channels() as usize;

        // Resize slope map to match source dimensions if needed
        let slope_resized = if slope_data.width() != width || slope_data.height() != height {
            slope_data.resize(width, height)
        } else {
            (*slope_data).clone()
        };

        let data_ref = &data;
        let slope_ref = &slope_resized;
        let h = height as usize;
        let w = width as usize;

        // Process each row in parallel
        let pixels: Vec<f32> = (0..h).into_par_iter().flat_map_iter(move |y| {
            // Thread-local sample buffer to avoid per-pixel allocation
            let mut sample = vec![0.0f32; ch];
            let mut row_pixels = Vec::with_capacity(w * ch);

            for x in 0..w {
                // Compute luminance at a neighboring pixel for gradient estimation.
                // Uses the first channel of the slope map as intensity.
                let luminance_at = |lx: u32, ly: u32| -> f32 {
                    let px = slope_ref.get_pixel(lx.min(width - 1), ly.min(height - 1));
                    // Use first channel as luminance (works for 1-ch grayscale and multi-ch)
                    px[0]
                };

                let xu = x as u32;
                let yu = y as u32;
                let x_left = if xu > 0 { xu - 1 } else { 0 };
                let x_right = if xu < width - 1 { xu + 1 } else { width - 1 };
                let y_top = if yu > 0 { yu - 1 } else { 0 };
                let y_bottom = if yu < height - 1 { yu + 1 } else { height - 1 };

                // Finite-difference gradient of the slope map
                let grad_x = luminance_at(x_right, yu) - luminance_at(x_left, yu);
                let grad_y = luminance_at(xu, y_bottom) - luminance_at(xu, y_top);

                // Normalize the gradient direction
                let grad_len = (grad_x * grad_x + grad_y * grad_y).sqrt();
                let (dx, dy) = if grad_len > 1e-6 {
                    (grad_x / grad_len, grad_y / grad_len)
                } else {
                    (0.0, 0.0)
                };

                let mut sums = vec![0.0f64; ch];

                // Sample along the gradient direction, centered on this pixel
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
#[path = "slope_blur_tests.rs"]
mod tests;
