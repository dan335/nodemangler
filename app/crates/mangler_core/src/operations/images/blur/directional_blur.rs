//! Directional blur operation for images.
//!
//! Blurs the image along a specified angle by averaging multiple bilinearly
//! sampled points distributed along a line centered at each pixel.

use crate::get_id;
use crate::value::ValueType;
use crate::input::{Input, InputSettings};
use crate::node_settings::NodeSettings;
use crate::operations::{OperationResponse, OperationError, OutputResponse, default_image, convert_input};
use crate::operations::images::transform::warp::bilinear_sample_rgba;
use crate::output::Output;
use crate::value::Value;
use image::DynamicImage;
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
        }
    }

    /// Creates the input ports: image, angle (degrees), sample count, and intensity (pixel spread).
    pub fn create_inputs() -> Vec<Input> {
        vec![
            Input::new("image".to_string(), Value::DynamicImage { data: default_image(), change_id: get_id() }, None, None),
            Input::new("angle".to_string(), Value::Decimal(0.0), Some(InputSettings::Slider { range: (0.0, 360.0), step_by: Some(1.0), clamp_to_range: true }), None),
            Input::new("samples".to_string(), Value::Integer(10), Some(InputSettings::DragValue { speed: None, clamp: Some((1.0, 100.0)) }), None),
            Input::new("intensity".to_string(), Value::Decimal(10.0), Some(InputSettings::Slider { range: (0.0, 100.0), step_by: Some(0.5), clamp_to_range: true }), None),
        ]
    }

    /// Creates the output port: the directionally blurred image.
    pub fn create_outputs() -> Vec<Output> {
        vec![
            Output::new("output".to_string(), Value::DynamicImage { data: default_image(), change_id: get_id() }, None),
        ]
    }

    /// Executes the directional blur. Samples are distributed symmetrically along the
    /// angle direction using bilinear interpolation.
    pub async fn run(inputs: &mut [Input]) -> Result<OperationResponse, OperationError> {
        let start_time = Instant::now();
        let mut input_errors: Vec<(usize, String)> = vec![];

        // convert inputs
        let image_converted = convert_input(inputs, 0, ValueType::DynamicImage, &mut input_errors);
        let angle_converted = convert_input(inputs, 1, ValueType::Decimal, &mut input_errors);
        let samples_converted = convert_input(inputs, 2, ValueType::Integer, &mut input_errors);
        let intensity_converted = convert_input(inputs, 3, ValueType::Decimal, &mut input_errors);

        // return if error
        if !input_errors.is_empty() { return Err(OperationError { input_errors, node_error: None }); }

        // get values
        let Value::DynamicImage { data, change_id: _ } = image_converted.unwrap() else { unreachable!() };
        let Value::Decimal(angle) = angle_converted.unwrap() else { unreachable!() };
        let Value::Integer(samples) = samples_converted.unwrap() else { unreachable!() };
        let Value::Decimal(intensity) = intensity_converted.unwrap() else { unreachable!() };

        // run node
        let samples = samples.max(1) as u32;
        let intensity = intensity.max(0.0);
        let angle_rad = angle.to_radians();
        let dx = angle_rad.cos();
        let dy = angle_rad.sin();

        let rgba = data.to_rgba8();
        let (width, height) = rgba.dimensions();
        let mut output_buf = image::RgbaImage::new(width, height);

        for y in 0..height {
            for x in 0..width {
                let mut r_sum: f64 = 0.0;
                let mut g_sum: f64 = 0.0;
                let mut b_sum: f64 = 0.0;
                let mut a_sum: f64 = 0.0;

                for i in 0..samples {
                    // Map sample index to [-1, 1] range for symmetric distribution
                    let t = if samples > 1 {
                        (i as f32 / (samples - 1) as f32) * 2.0 - 1.0
                    } else {
                        0.0
                    };
                    let offset = t * intensity;
                    let sx = x as f32 + dx * offset;
                    let sy = y as f32 + dy * offset;
                    let pixel = bilinear_sample_rgba(&rgba, sx, sy);
                    r_sum += pixel[0] as f64;
                    g_sum += pixel[1] as f64;
                    b_sum += pixel[2] as f64;
                    a_sum += pixel[3] as f64;
                }

                let count = samples as f64;
                output_buf.put_pixel(x, y, image::Rgba([
                    (r_sum / count) as u8,
                    (g_sum / count) as u8,
                    (b_sum / count) as u8,
                    (a_sum / count) as u8,
                ]));
            }
        }

        Ok(OperationResponse {
            time: Instant::now().duration_since(start_time),
            responses: vec![
                OutputResponse { value: Value::DynamicImage { data: Arc::new(DynamicImage::ImageRgba8(output_buf)), change_id: get_id() } },
            ],
        })
    }
}

#[cfg(test)]
#[path = "directional_blur_tests.rs"]
mod tests;
