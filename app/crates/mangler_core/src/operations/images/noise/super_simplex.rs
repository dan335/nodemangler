//! Super simplex noise image generator.
//!
//! Produces a seamlessly tiling grayscale image using an improved variant of
//! simplex noise with better isotropy and fewer visual artifacts. Tiling is
//! achieved via a 4-sample bilinear blend since SuperSimplex only supports 3D.

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
use noise::{NoiseFn, SuperSimplex};

/// Operation that generates a seamlessly tiling grayscale image from super simplex noise.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpImageNoiseSuperSimplex {}

impl OpImageNoiseSuperSimplex {
    /// Returns the node metadata (name and description) for this operation.
    pub fn settings() -> NodeSettings {
        NodeSettings {
            name: "super simplex noise".to_string(),
            description: "Creates a seamlessly tiling image from super simplex noise.".to_string(),
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

    /// Generates a super simplex noise image from the given inputs.
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
        let period = scale.max(1) as f64;

        let super_simplex = SuperSimplex::new(seed as u32);
        let noise_ref = &super_simplex;

        let w = width as usize;
        let h = height as usize;
        // Compute pixels in parallel, iterating in row-major order (y outer, x inner)
        let pixels: Vec<f32> = (0..h).into_par_iter().flat_map_iter(|y| {
            (0..w).map(move |x| {
                // SuperSimplex only supports up to 3D, so use 4-sample bilinear blend
                // for seamless tiling: sample at 4 offset positions and blend by position.
                let nx = x as f64 / w as f64 * period;
                let ny = y as f64 / h as f64 * period;
                let bx = x as f64 / w as f64;
                let by = y as f64 / h as f64;

                let s00 = noise_ref.get([nx, ny]) as f32;
                let s10 = noise_ref.get([nx + period, ny]) as f32;
                let s01 = noise_ref.get([nx, ny + period]) as f32;
                let s11 = noise_ref.get([nx + period, ny + period]) as f32;

                let bx = bx as f32;
                let by = by as f32;
                let top = s00 * (1.0 - bx) + s10 * bx;
                let bottom = s01 * (1.0 - bx) + s11 * bx;
                let noise = (top * (1.0 - by) + bottom * by) * 0.5 + 0.5;
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
#[path = "super_simplex_tests.rs"]
mod tests;
