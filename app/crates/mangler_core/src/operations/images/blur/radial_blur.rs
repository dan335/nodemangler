//! Radial (spin) blur operation for images.
//!
//! Applies a circular motion blur around the image center by sampling
//! pixels at multiple angular offsets at the same radial distance.

use crate::get_id;
use crate::value::ValueType;
use crate::input::{Input, InputSettings};
use crate::node_settings::NodeSettings;
use crate::operations::{OperationResponse, OperationError, OutputResponse, default_image, convert_input};
use crate::operations::images::transform::warp::bilinear_sample_rgba;
use crate::output::Output;
use crate::value::Value;
use image::DynamicImage;
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
        }
    }

    /// Creates the input ports: image, spin angle (degrees), and number of samples.
    pub fn create_inputs() -> Vec<Input> {
        vec![
            Input::new("image".to_string(), Value::DynamicImage { data: default_image(), change_id: get_id() }, None, None),
            Input::new("angle".to_string(), Value::Decimal(10.0), Some(InputSettings::Slider { range: (0.0, 180.0), step_by: Some(1.0), clamp_to_range: true }), None),
            Input::new("samples".to_string(), Value::Integer(10), Some(InputSettings::DragValue { speed: None, clamp: Some((1.0, 100.0)) }), None),
        ]
    }

    /// Creates the output port: the radially blurred image.
    pub fn create_outputs() -> Vec<Output> {
        vec![
            Output::new("output".to_string(), Value::DynamicImage { data: default_image(), change_id: get_id() }, None),
        ]
    }

    /// Executes the radial blur. For each pixel, computes the angle and distance from
    /// the image center, then averages samples taken at angular offsets around that arc.
    pub async fn run(inputs: &mut [Input]) -> Result<OperationResponse, OperationError> {
        let start_time = Instant::now();
        let mut input_errors: Vec<(usize, String)> = vec![];

        // convert inputs
        let image_converted = convert_input(inputs, 0, ValueType::DynamicImage, &mut input_errors);
        let angle_converted = convert_input(inputs, 1, ValueType::Decimal, &mut input_errors);
        let samples_converted = convert_input(inputs, 2, ValueType::Integer, &mut input_errors);

        // return if error
        if !input_errors.is_empty() { return Err(OperationError { input_errors, node_error: None }); }

        // get values
        let Value::DynamicImage { data, change_id: _ } = image_converted.unwrap() else { unreachable!() };
        let Value::Decimal(angle) = angle_converted.unwrap() else { unreachable!() };
        let Value::Integer(samples) = samples_converted.unwrap() else { unreachable!() };

        // run node
        let samples = samples.max(1) as u32;
        let angle_rad = angle.to_radians();

        let rgba = data.to_rgba8();
        let (width, height) = rgba.dimensions();
        let cx = width as f32 / 2.0;
        let cy = height as f32 / 2.0;
        let rgba_ref = &rgba;
        let h = height as usize;
        let w = width as usize;

        let pixels: Vec<u8> = (0..h).into_par_iter().flat_map_iter(move |y| {
            (0..w).flat_map(move |x| {
                let ddx = x as f32 - cx;
                let ddy = y as f32 - cy;
                let base_angle = ddy.atan2(ddx);
                let dist = (ddx * ddx + ddy * ddy).sqrt();

                let mut r_sum: f64 = 0.0;
                let mut g_sum: f64 = 0.0;
                let mut b_sum: f64 = 0.0;
                let mut a_sum: f64 = 0.0;

                for i in 0..samples {
                    let t = if samples > 1 {
                        (i as f32 / (samples - 1) as f32) - 0.5
                    } else {
                        0.0
                    };
                    let sample_angle = base_angle + t * angle_rad;
                    let sx = cx + dist * sample_angle.cos();
                    let sy = cy + dist * sample_angle.sin();
                    let pixel = bilinear_sample_rgba(rgba_ref, sx, sy);
                    r_sum += pixel[0] as f64;
                    g_sum += pixel[1] as f64;
                    b_sum += pixel[2] as f64;
                    a_sum += pixel[3] as f64;
                }

                let count = samples as f64;
                [
                    (r_sum / count) as u8,
                    (g_sum / count) as u8,
                    (b_sum / count) as u8,
                    (a_sum / count) as u8,
                ]
            })
        }).collect();

        let output_buf = image::RgbaImage::from_raw(width, height, pixels).unwrap();

        Ok(OperationResponse {
            time: Instant::now().duration_since(start_time),
            responses: vec![
                OutputResponse { value: Value::DynamicImage { data: Arc::new(DynamicImage::ImageRgba8(output_buf)), change_id: get_id() } },
            ],
        })
    }
}

#[cfg(test)]
#[path = "radial_blur_tests.rs"]
mod tests;
