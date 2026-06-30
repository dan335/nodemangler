//! Gaussian (white) noise image generator.
//!
//! Produces a seamlessly tiling grayscale image of pseudo-random per-pixel noise.
//! Uses integer hashing of pixel coordinates with periodic wrapping, so the
//! pattern repeats every `scale` pixels in each axis for seamless tiling.

use rayon::prelude::*;
use crate::color::color_spaces::rgb_linear::linear_to_nonlinear_srgb;
use crate::float_image::FloatImage;
use crate::get_id;
use crate::input::{Input, InputSettings};
use crate::node_settings::NodeSettings;
use crate::operations::{OperationResponse, OperationError, OutputResponse, default_image, convert_input};
use crate::output::Output;
use crate::value::{Value, ValueType};
use super::pixel_hash;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use std::time::Instant;

/// Operation that generates a seamlessly tiling grayscale image of white noise.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpImageNoiseGaussian {}

impl OpImageNoiseGaussian {
    /// Returns the node metadata (name and description) for this operation.
    pub fn settings() -> NodeSettings {
        NodeSettings {
            name: "white noise".to_string(),
            description: "Creates a seamlessly tiling image of per-pixel white noise.".to_string(),
            help: "Uncorrelated white noise: each pixel is hashed independently from its wrapped integer coordinates and the seed, giving a uniform random brightness in [0,1] with no spatial structure at all. Adjacent pixels have no relationship, so the output has flat frequency content across the spectrum.\n\nScale is the tile period in pixels: coordinates are wrapped modulo scale, so the pattern repeats every scale pixels for seamless tiling.\n\nUseful as a dither source, film grain, a starting mask for stochastic operations, or as a feed into blurs that turn it into coherent noise.".to_string(),
        }
    }

    /// Creates the default inputs: seed, width, height, and scale (tile period in pixels).
    pub fn create_inputs() -> Vec<Input> {
        vec![
            Input::new("seed".to_string(), Value::Integer(1), Some(InputSettings::DragValue { clamp: None, speed: None }), None)
                .with_description("Random seed for the white noise pattern; change to reshuffle the pixels."),
            Input::new("width".to_string(), Value::Integer(512), Some(InputSettings::DragValue { clamp: Some((1.0, 10000.0)), speed: None }), None)
                .with_description("Output image width in pixels."),
            Input::new("height".to_string(), Value::Integer(512), Some(InputSettings::DragValue { clamp: Some((1.0, 10000.0)), speed: None }), None)
                .with_description("Output image height in pixels."),
            Input::new("scale".to_string(), Value::Integer(512), Some(InputSettings::DragValue { clamp: Some((1.0, 10000.0)), speed: None }), None)
                .with_description("Tile period in pixels; the noise repeats every scale pixels for seamless tiling."),
        ]
    }

    /// Creates the default output: a single grayscale image.
    pub fn create_outputs() -> Vec<Output> {
        vec![
            Output::new("output".to_string(), Value::Image { data: default_image(), change_id: get_id() }, None)
                .with_description("Seamlessly tiling grayscale per-pixel white noise image."),
        ]
    }

    /// Generates a white noise image from the given inputs.
    ///
    /// Each pixel is independently hashed from its wrapped coordinates and seed,
    /// producing uniform random brightness. Wrapping at `scale` ensures tiling.
    pub async fn run(inputs: &mut [Input]) -> Result<OperationResponse, OperationError> {
        let start_time = Instant::now();
        let mut input_errors: Vec<(usize, String)> = vec![];

        let seed_converted = convert_input(inputs, 0, ValueType::Integer, &mut input_errors);
        let width_converted = convert_input(inputs, 1, ValueType::Integer, &mut input_errors);
        let height_converted = convert_input(inputs, 2, ValueType::Integer, &mut input_errors);
        let scale_converted = convert_input(inputs, 3, ValueType::Integer, &mut input_errors);

        if !input_errors.is_empty() { return Err(OperationError { input_errors, node_error: None }); }

        let Value::Integer(mut seed) = seed_converted.unwrap() else { unreachable!() };
        let Value::Integer(mut width) = width_converted.unwrap() else { unreachable!() };
        let Value::Integer(mut height) = height_converted.unwrap() else { unreachable!() };
        let Value::Integer(scale) = scale_converted.unwrap() else { unreachable!() };

        width = width.max(1);
        height = height.max(1);
        seed = seed.max(1);
        let period = scale.max(1) as u32;
        let seed_u32 = seed as u32;

        let w = width as usize;
        let h = height as usize;

        // Each pixel is hashed independently; wrap coordinates at `period` for tiling
        let pixels: Vec<f32> = (0..h).into_par_iter().flat_map_iter(|y| {
            (0..w).map(move |x| {
                let wx = (x as u32) % period;
                let wy = (y as u32) % period;
                let noise = pixel_hash(wx, wy, seed_u32);
                linear_to_nonlinear_srgb(noise)
            })
        }).collect();

        // Build a single-channel FloatImage from the computed pixel values
        let mut float_image = FloatImage::new(width as u32, height as u32, 1);
        for (i, &val) in pixels.iter().enumerate() {
            let x = (i % w) as u32;
            let y = (i / w) as u32;
            float_image.put_pixel(x, y, &[val]);
        }

        Ok(OperationResponse { 
            time: Instant::now().duration_since(start_time),
            responses: vec![
                OutputResponse { value: Value::Image { data: Arc::new(float_image), change_id: get_id() } },
            ],
        })
    }
}

#[cfg(test)]
#[path = "gaussian_tests.rs"]
mod tests;
