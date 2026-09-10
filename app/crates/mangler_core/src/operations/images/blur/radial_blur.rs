//! Radial (spin) blur operation for images.
//!
//! Applies a circular motion blur around the image center by sampling
//! pixels at multiple angular offsets at the same radial distance.
//! Works directly on [`FloatImage`] f32 data.

use crate::float_image::FloatImage;
use crate::get_id;
use crate::input::{Input, InputSettings};
use crate::node_settings::NodeSettings;
use crate::convert_inputs;
use crate::operations::{OperationResponse, OperationError, OutputResponse, default_image, image_input};
use crate::output::Output;
use crate::value::Value;
use rayon::prelude::*;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use std::time::Instant;

/// Radial blur operation that creates a circular spin blur effect around the image center.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpImageAdjustmentRadialBlur {}

impl OpImageAdjustmentRadialBlur {
    /// Returns the node metadata (name and description) for the radial blur operation.
    pub fn settings() -> NodeSettings {
        NodeSettings {
            name: "radial blur".to_string(),
            description: "Applies a circular spin blur around the image center.".to_string(),
            help: "For each output pixel, converts its position relative to the image centre into polar coordinates (angle, distance), then averages samples taken at evenly spaced angular offsets over +/- half of the sweep angle while keeping the same distance. This produces the rotational motion-blur look, strongest at the edges and negligible at the centre.\n\nSamples are bilinear for smooth results. Work is parallelised across rows. Angle 0 or samples = 1 returns the image unchanged. The centre is fixed at the image middle; for off-centre spins, use a safe-transform node first.".to_string(),
        }
    }

    /// Creates the input ports: image, spin angle (degrees), and number of samples.
    pub fn create_inputs() -> Vec<Input> {
        vec![
            image_input("image")
                .with_description("Source image to spin around the image centre."),
            Input::new("angle".to_string(), Value::Decimal(10.0), Some(InputSettings::Slider { range: (0.0, 180.0), step_by: Some(1.0), clamp_to_range: true }), None)
                .with_description("Total sweep in degrees each pixel is smeared across its arc."),
            Input::new("samples".to_string(), Value::Integer(10), Some(InputSettings::DragValue { speed: None, clamp: Some((1.0, 100.0)) }), None)
                .with_description("Number of angular taps averaged along each pixel's arc."),
        ]
    }

    /// Creates the output port: the radially blurred image.
    pub fn create_outputs() -> Vec<Output> {
        vec![
            Output::new("output".to_string(), Value::Image { data: default_image(), change_id: get_id() }, None)
                .with_description("Image with a circular spin blur around its centre."),
        ]
    }

    /// Executes the radial blur. For each pixel, computes the angle and distance from
    /// the image center, then averages samples taken at angular offsets around that arc.
    pub async fn run(inputs: &mut [Input]) -> Result<OperationResponse, OperationError> {
        let start_time = Instant::now();

        convert_inputs! { inputs;
            Image(data) = 0,
            Decimal(angle) = 1,
            Integer(samples) = 2,
        }

        // run node
        let samples = samples.max(1) as u32;
        let angle_rad = angle.to_radians();

        let (width, height) = data.dimensions();
        let ch = data.channels() as usize;
        let cx = width as f32 / 2.0;
        let cy = height as f32 / 2.0;
        let data_ref = &data;
        let h = height as usize;
        let w = width as usize;

        // Process each row in parallel, sampling along arcs around the center
        let pixels: Vec<f32> = (0..h).into_par_iter().flat_map_iter(move |y| {
            // Thread-local sample buffer to avoid per-pixel allocation
            let mut sample = vec![0.0f32; ch];
            let mut row_pixels = Vec::with_capacity(w * ch);

            for x in 0..w {
                // Compute polar coordinates relative to the image center
                let ddx = x as f32 - cx;
                let ddy = y as f32 - cy;
                let base_angle = ddy.atan2(ddx);
                let dist = (ddx * ddx + ddy * ddy).sqrt();

                // channels are always <= 4, so a stack array avoids a
                // per-pixel heap allocation
                let mut sums = [0.0f64; 4];

                // Sample at angular offsets along the arc
                for i in 0..samples {
                    let t = if samples > 1 {
                        (i as f32 / (samples - 1) as f32) - 0.5
                    } else {
                        0.0
                    };
                    let sample_angle = base_angle + t * angle_rad;
                    let sx = cx + dist * sample_angle.cos();
                    let sy = cy + dist * sample_angle.sin();
                    data_ref.bilinear_sample(sx, sy, &mut sample);
                    for c in 0..ch {
                        sums[c] += sample[c] as f64;
                    }
                }

                // Average across all samples
                let count = samples as f64;
                for val in sums.iter().take(ch) {
                    row_pixels.push((val / count) as f32);
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
#[path = "radial_blur_tests.rs"]
mod tests;
