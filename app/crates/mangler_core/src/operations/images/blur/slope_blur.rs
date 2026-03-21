//! Slope blur operation for images.
//!
//! Blurs the image along directions derived from the gradient of a separate
//! grayscale slope map. The gradient direction at each pixel determines the
//! blur direction, creating an effect similar to paint being smeared downhill.

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
use image::imageops::FilterType;
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
        }
    }

    /// Creates the input ports: source image, slope map (grayscale gradient source),
    /// intensity (pixel spread), and number of samples.
    pub fn create_inputs() -> Vec<Input> {
        vec![
            Input::new("image".to_string(), Value::DynamicImage { data: default_image(), change_id: get_id() }, None, None),
            Input::new("slope map".to_string(), Value::DynamicImage { data: default_image(), change_id: get_id() }, None, None),
            Input::new("intensity".to_string(), Value::Decimal(10.0), Some(InputSettings::Slider { range: (0.0, 100.0), step_by: Some(0.5), clamp_to_range: true }), None),
            Input::new("samples".to_string(), Value::Integer(10), Some(InputSettings::DragValue { speed: None, clamp: Some((1.0, 100.0)) }), None),
        ]
    }

    /// Creates the output port: the slope-blurred image.
    pub fn create_outputs() -> Vec<Output> {
        vec![
            Output::new("output".to_string(), Value::DynamicImage { data: default_image(), change_id: get_id() }, None),
        ]
    }

    /// Executes the slope blur. Computes per-pixel gradient direction from the slope map
    /// using finite differences, then averages bilinear samples along that direction.
    pub async fn run(inputs: &mut [Input]) -> Result<OperationResponse, OperationError> {
        let start_time = Instant::now();
        let mut input_errors: Vec<(usize, String)> = vec![];

        // convert inputs
        let image_converted = convert_input(inputs, 0, ValueType::DynamicImage, &mut input_errors);
        let slope_map_converted = convert_input(inputs, 1, ValueType::DynamicImage, &mut input_errors);
        let intensity_converted = convert_input(inputs, 2, ValueType::Decimal, &mut input_errors);
        let samples_converted = convert_input(inputs, 3, ValueType::Integer, &mut input_errors);

        // return if error
        if !input_errors.is_empty() { return Err(OperationError { input_errors, node_error: None }); }

        // get values
        let Value::DynamicImage { data, change_id: _ } = image_converted.unwrap() else { unreachable!() };
        let Value::DynamicImage { data: slope_data, change_id: _ } = slope_map_converted.unwrap() else { unreachable!() };
        let Value::Decimal(intensity) = intensity_converted.unwrap() else { unreachable!() };
        let Value::Integer(samples) = samples_converted.unwrap() else { unreachable!() };

        // run node
        let samples = samples.max(1) as u32;
        let intensity = intensity.max(0.0);

        let rgba = data.to_rgba8();
        let (width, height) = rgba.dimensions();

        // resize slope map to match source if needed
        let slope_resized = if slope_data.width() != width || slope_data.height() != height {
            slope_data.resize_exact(width, height, FilterType::Lanczos3)
        } else {
            (*slope_data).clone()
        };
        let slope_rgba = slope_resized.to_rgba8();

        let rgba_ref = &rgba;
        let slope_ref = &slope_rgba;
        let h = height as usize;
        let w = width as usize;

        let pixels: Vec<u8> = (0..h).into_par_iter().flat_map_iter(move |y| {
            (0..w).flat_map(move |x| {
                let luminance_at = |lx: u32, ly: u32| -> f32 {
                    let px = slope_ref.get_pixel(lx.min(width - 1), ly.min(height - 1));
                    0.299 * (px[0] as f32 / 255.0) + 0.587 * (px[1] as f32 / 255.0) + 0.114 * (px[2] as f32 / 255.0)
                };

                let xu = x as u32;
                let yu = y as u32;
                let x_left = if xu > 0 { xu - 1 } else { 0 };
                let x_right = if xu < width - 1 { xu + 1 } else { width - 1 };
                let y_top = if yu > 0 { yu - 1 } else { 0 };
                let y_bottom = if yu < height - 1 { yu + 1 } else { height - 1 };

                let grad_x = luminance_at(x_right, yu) - luminance_at(x_left, yu);
                let grad_y = luminance_at(xu, y_bottom) - luminance_at(xu, y_top);

                let grad_len = (grad_x * grad_x + grad_y * grad_y).sqrt();
                let (dx, dy) = if grad_len > 1e-6 {
                    (grad_x / grad_len, grad_y / grad_len)
                } else {
                    (0.0, 0.0)
                };

                let mut r_sum: f64 = 0.0;
                let mut g_sum: f64 = 0.0;
                let mut b_sum: f64 = 0.0;
                let mut a_sum: f64 = 0.0;

                for i in 0..samples {
                    let t = if samples > 1 {
                        (i as f32 / (samples - 1) as f32) * 2.0 - 1.0
                    } else {
                        0.0
                    };
                    let offset = t * intensity;
                    let sx = x as f32 + dx * offset;
                    let sy = y as f32 + dy * offset;
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
#[path = "slope_blur_tests.rs"]
mod tests;
