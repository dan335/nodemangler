//! Ambient occlusion generation from a height map.
//!
//! Approximates ambient occlusion by sampling height differences at evenly
//! spaced angles around each pixel. Higher neighboring surfaces contribute
//! more occlusion, producing darker values in concavities and crevices.

use crate::get_id;
use crate::value::ValueType;
use image::DynamicImage;
use rayon::prelude::*;
use crate::input::{Input, InputSettings};
use crate::node_settings::NodeSettings;
use crate::operations::{OperationResponse, OperationError, OutputResponse, default_image, convert_input};
use crate::output::Output;
use crate::value::Value;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use std::time::Instant;

/// Operation that computes ambient occlusion from a grayscale height map.
///
/// For each pixel, samples are taken at evenly spaced angles at the given radius.
/// The height difference (clamped to positive) divided by distance accumulates
/// occlusion, which is then scaled by intensity and subtracted from 1.0.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpImagePbrAoFromHeight {}

impl OpImagePbrAoFromHeight {
    /// Returns the node metadata (name and description) for this operation.
    pub fn settings() -> NodeSettings {
        NodeSettings {
            name: "ao from height".to_string(),
            description: "Computes ambient occlusion from a height map.".to_string(),
        }
    }

    /// Creates the default inputs: height map image, radius, intensity, and sample count.
    pub fn create_inputs() -> Vec<Input> {
        vec![
            Input::new("image".to_string(), Value::DynamicImage { data: default_image(), change_id: get_id() }, None, None),
            Input::new("radius".to_string(), Value::Integer(8), Some(InputSettings::DragValue { speed: None, clamp: Some((1.0, 64.0)) }), None),
            Input::new("intensity".to_string(), Value::Decimal(1.0), Some(InputSettings::Slider { range: (0.1, 10.0), step_by: Some(0.1), clamp_to_range: true }), None),
            Input::new("samples".to_string(), Value::Integer(16), Some(InputSettings::DragValue { speed: None, clamp: Some((4.0, 64.0)) }), None),
        ]
    }

    /// Creates the default output: a single RGBA32F ambient occlusion image.
    pub fn create_outputs() -> Vec<Output> {
        vec![
            Output::new("output".to_string(), Value::DynamicImage { data: default_image(), change_id: get_id() }, None),
        ]
    }

    /// Computes ambient occlusion from the input height map by radial sampling.
    pub async fn run(inputs: &mut [Input]) -> Result<OperationResponse, OperationError> {
        let start_time = Instant::now();
        let mut input_errors: Vec<(usize, String)> = vec![];

        // convert inputs
        let image_converted = convert_input(inputs, 0, ValueType::DynamicImage, &mut input_errors);
        let radius_converted = convert_input(inputs, 1, ValueType::Integer, &mut input_errors);
        let intensity_converted = convert_input(inputs, 2, ValueType::Decimal, &mut input_errors);
        let samples_converted = convert_input(inputs, 3, ValueType::Integer, &mut input_errors);

        // return if error
        if !input_errors.is_empty() { return Err(OperationError { input_errors, node_error: None }); }

        // get values
        let Value::DynamicImage { data, change_id: _ } = image_converted.unwrap() else { unreachable!() };
        let Value::Integer(radius) = radius_converted.unwrap() else { unreachable!() };
        let Value::Decimal(intensity) = intensity_converted.unwrap() else { unreachable!() };
        let Value::Integer(samples) = samples_converted.unwrap() else { unreachable!() };

        // run node
        let buffer = data.to_rgba32f();
        let width = buffer.width() as usize;
        let height = buffer.height() as usize;
        let radius = (radius as i64).clamp(1, 64) as usize;
        let intensity = intensity;
        let samples = (samples as i64).clamp(4, 64) as usize;

        // Extract luminance (Rec. 709) as height values
        let mut heights: Vec<f32> = Vec::with_capacity(width * height);
        for pixel in buffer.pixels() {
            let h = 0.2126 * pixel[0] + 0.7152 * pixel[1] + 0.0722 * pixel[2];
            heights.push(h);
        }

        let two_pi = std::f32::consts::TAU;
        let heights_ref = &heights;

        let pixels: Vec<f32> = (0..height).into_par_iter().flat_map_iter(move |y| {
            (0..width).flat_map(move |x| {
                let h = heights_ref[y * width + x];
                let mut occlusion = 0.0f32;

                for i in 0..samples {
                    let angle = i as f32 * two_pi / samples as f32;
                    let ddx = angle.cos() * radius as f32;
                    let ddy = angle.sin() * radius as f32;

                    let sx = (x as f32 + ddx).round().clamp(0.0, (width - 1) as f32) as usize;
                    let sy = (y as f32 + ddy).round().clamp(0.0, (height - 1) as f32) as usize;

                    let nh = heights_ref[sy * width + sx];
                    let dist = ((sx as f32 - x as f32).powi(2) + (sy as f32 - y as f32).powi(2)).sqrt().max(1.0);
                    let diff = (nh - h).max(0.0);
                    occlusion += diff / dist;
                }

                occlusion /= samples as f32;
                let ao = (1.0 - occlusion * intensity).clamp(0.0, 1.0);
                [ao, ao, ao, 1.0]
            })
        }).collect();

        let out_buffer = image::Rgba32FImage::from_raw(width as u32, height as u32, pixels).unwrap();
        let result = DynamicImage::ImageRgba32F(out_buffer);

        Ok(OperationResponse {
            time: Instant::now().duration_since(start_time),
            responses: vec![
                OutputResponse { value: Value::DynamicImage { data: Arc::new(result), change_id: get_id() } },
            ],
        })
    }
}


#[cfg(test)]
#[path = "ao_from_height_tests.rs"]
mod tests;
