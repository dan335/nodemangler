//! Open simplex noise image generator.
//!
//! Produces a seamlessly tiling grayscale image using OpenSimplex noise, a
//! patent-free alternative to simplex noise. Tiling is achieved by mapping
//! 2D coordinates onto a 4D torus.

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
use noise::{NoiseFn, OpenSimplex};

/// Operation that generates a seamlessly tiling grayscale image from OpenSimplex noise.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpImageNoiseOpenSimplex {}

impl OpImageNoiseOpenSimplex {
    /// Returns the node metadata (name and description) for this operation.
    pub fn settings() -> NodeSettings {
        NodeSettings {
            name: "open simplex noise".to_string(),
            description: "Creates a seamlessly tiling image from open simplex noise.".to_string(),
        }
    }

    /// Creates the default inputs: seed, width, height, and scale.
    pub fn create_inputs() -> Vec<Input> {
        vec![
            Input::new("seed".to_string(), Value::Integer(1), Some(InputSettings::DragValue { clamp: None, speed: None }), None),
            Input::new("width".to_string(), Value::Integer(512), Some(InputSettings::DragValue {clamp:Some((1.0,10000.0)), speed: None }), None),
            Input::new("height".to_string(), Value::Integer(512), Some(InputSettings::DragValue {clamp:Some((1.0,10000.0)), speed: None }), None),
            Input::new("scale".to_string(), Value::Integer(10), Some(InputSettings::DragValue { clamp: Some((1.0, 1000.0)), speed: None }), None),
        ]
    }

    /// Creates the default output: a single grayscale image.
    pub fn create_outputs() -> Vec<Output> {
        vec![
            Output::new("output".to_string(), Value::Image { data:default_image(), change_id:get_id() }, None),
        ]
    }

    /// Generates an OpenSimplex noise image from the given inputs.
    pub async fn run(inputs: &mut [Input]) -> Result<OperationResponse, OperationError> {
        let start_time = Instant::now();
        let mut input_errors: Vec<(usize, String)> = vec![];

        // convert inputs
        let seed_converted = convert_input(inputs, 0, ValueType::Integer, &mut input_errors);
        let width_converted = convert_input(inputs, 1, ValueType::Integer, &mut input_errors);
        let height_converted = convert_input(inputs, 2, ValueType::Integer, &mut input_errors);
        let scale_converted = convert_input(inputs, 3, ValueType::Integer, &mut input_errors);

        // return if error
        if !input_errors.is_empty() { return Err(OperationError { input_errors, node_error: None }); }

        // get values
        let Value::Integer(mut seed) = seed_converted.unwrap() else { unreachable!() };
        let Value::Integer(mut width) = width_converted.unwrap() else { unreachable!() };
        let Value::Integer(mut height) = height_converted.unwrap() else { unreachable!() };
        let Value::Integer(scale) = scale_converted.unwrap() else { unreachable!() };

        // run node
        width = width.max(1);
        height = height.max(1);
        seed = seed.max(1);
        let scale = scale.max(1) as f64;

        let open_simplex = OpenSimplex::new(seed as u32);
        let noise_ref = &open_simplex;

        let w = width as usize;
        let h = height as usize;
        // Compute pixels in parallel, iterating in row-major order (y outer, x inner)
        let pixels: Vec<f32> = (0..h).into_par_iter().flat_map_iter(|y| {
            (0..w).map(move |x| {
                // Map 2D coordinates onto a 4D torus for seamless tiling.
                // OpenSimplex supports 4D natively, so this produces correct tiles.
                let tau = std::f64::consts::TAU;
                let u = x as f64 / w as f64;
                let v = y as f64 / h as f64;
                let r = scale / tau;
                let noise = noise_ref.get([
                    (tau * u).cos() * r,
                    (tau * u).sin() * r,
                    (tau * v).cos() * r,
                    (tau * v).sin() * r,
                ]) as f32 * 0.5 + 0.5;
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

        Ok(OperationResponse { ai_cost_usd: None,
            time: Instant::now().duration_since(start_time),
            responses: vec![
                OutputResponse { value: Value::Image { data: Arc::new(float_image), change_id: get_id() } },
            ],
        })
    }
}


#[cfg(test)]
#[path = "open_simplex_tests.rs"]
mod tests;
