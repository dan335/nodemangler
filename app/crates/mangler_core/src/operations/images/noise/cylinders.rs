//! Cylinders noise image generator.
//!
//! Produces a seamlessly tiling grayscale image of concentric cylindrical rings.
//! Uses toroidal distance so the rings wrap at tile boundaries, centering the
//! pattern at (0.5, 0.5) in UV space.

use rayon::prelude::*;
use crate::color::color_spaces::rgb_linear::linear_to_nonlinear_srgb;
use crate::float_image::FloatImage;
use crate::get_id;
use crate::input::{Input, InputSettings};
use crate::node_settings::NodeSettings;
use crate::operations::{OperationResponse, OperationError, OutputResponse, default_image, convert_input};
use crate::output::Output;
use crate::value::{Value, ValueType};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use std::time::Instant;

/// Operation that generates a seamlessly tiling concentric cylinder pattern.
///
/// Uses toroidal distance from the tile center so that the rings wrap around
/// at all edges, producing a seamless tile. The `frequency` parameter controls
/// how many rings appear within the tile.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpImageNoiseCylinders {}

impl OpImageNoiseCylinders {
    /// Returns the node metadata (name and description) for this operation.
    pub fn settings() -> NodeSettings {
        NodeSettings {
            name: "concentric rings".to_string(),
            description: "Seamlessly tiling concentric cylinder rings using toroidal distance.".to_string(),
            help: "A deterministic radial wave function, not a stochastic noise. Each pixel's value comes from a triangle wave applied to the toroidal (wrapped) distance from the tile center, so the pattern reads as concentric rings that wrap cleanly at the edges.\n\nFrequency sets how many rings radiate from the center across the tile. Useful as a mask for radial gradients, tree-ring textures, ripples, or as a seed for warp operations that turn the regular rings into something more organic.".to_string(),
        }
    }

    /// Creates the default inputs: width, height, and frequency.
    pub fn create_inputs() -> Vec<Input> {
        vec![
            Input::new("width".to_string(), Value::Integer(512), Some(InputSettings::DragValue { clamp: Some((1.0, 10000.0)), speed: None }), None)
                .with_description("Output image width in pixels."),
            Input::new("height".to_string(), Value::Integer(512), Some(InputSettings::DragValue { clamp: Some((1.0, 10000.0)), speed: None }), None)
                .with_description("Output image height in pixels."),
            Input::new("frequency".to_string(), Value::Decimal(1.0), Some(InputSettings::DragValue { clamp: None, speed: None }), None)
                .with_description("Number of concentric rings radiating from the tile center."),
        ]
    }

    /// Creates the default output: a single grayscale image.
    pub fn create_outputs() -> Vec<Output> {
        vec![
            Output::new("output".to_string(), Value::Image { data: default_image(), change_id: get_id() }, None)
                .with_description("Seamlessly tiling grayscale image of concentric cylindrical rings."),
        ]
    }

    /// Generates a seamlessly tiling cylinders noise image from the given inputs.
    ///
    /// For each pixel, computes the toroidal distance from the tile center (0.5, 0.5),
    /// then applies a cylinder wave function: `1.0 - min(fract, 1.0 - fract) * 4.0`,
    /// which produces smooth concentric rings that tile seamlessly.
    pub async fn run(inputs: &mut [Input]) -> Result<OperationResponse, OperationError> {
        let start_time = Instant::now();
        let mut input_errors: Vec<(usize, String)> = vec![];

        // Convert inputs
        let width_converted = convert_input(inputs, 0, ValueType::Integer, &mut input_errors);
        let height_converted = convert_input(inputs, 1, ValueType::Integer, &mut input_errors);
        let frequency_converted = convert_input(inputs, 2, ValueType::Decimal, &mut input_errors);

        // Return if error
        if !input_errors.is_empty() { return Err(OperationError { input_errors, node_error: None }); }

        // Get values
        let Value::Integer(mut width) = width_converted.unwrap() else { unreachable!() };
        let Value::Integer(mut height) = height_converted.unwrap() else { unreachable!() };
        let Value::Decimal(frequency) = frequency_converted.unwrap() else { unreachable!() };

        width = width.max(1);
        height = height.max(1);
        let freq = frequency as f64;
        let w = width as usize;
        let h = height as usize;

        // Compute all pixels in parallel using toroidal distance for seamless tiling.
        let pixels: Vec<f32> = (0..h).into_par_iter().flat_map_iter(move |y| {
            (0..w).map(move |x| {
                // Normalize to [0, 1]
                let u = x as f64 / w as f64;
                let v = y as f64 / h as f64;

                // Toroidal distance from center (0.5, 0.5):
                // shortest distance wrapping around tile edges
                let dx_abs = (u - 0.5).abs();
                let dy_abs = (v - 0.5).abs();
                let dx = dx_abs.min(1.0 - dx_abs);
                let dy = dy_abs.min(1.0 - dy_abs);
                let dist = (dx * dx + dy * dy).sqrt();

                // Cylinder wave: same function as the noise crate's Cylinders
                let scaled = dist * freq;
                let fract = scaled - scaled.floor();
                let noise = (1.0 - fract.min(1.0 - fract) * 4.0) as f32 * 0.5 + 0.5;

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
#[path = "cylinders_tests.rs"]
mod tests;
