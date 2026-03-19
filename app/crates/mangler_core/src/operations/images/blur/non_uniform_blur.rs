//! Non-uniform (variable-radius) blur operation for images.
//!
//! Applies a per-pixel blur where the radius at each pixel is determined by
//! the luminance of a separate grayscale blur map. Bright areas in the map
//! get more blur; dark areas stay sharp. Uses a Vogel disc sampling pattern.

use crate::get_id;
use crate::value::ValueType;
use crate::input::{Input, InputSettings};
use crate::node_settings::NodeSettings;
use crate::operations::{OperationResponse, OperationError, OutputResponse, default_image, convert_input};
use crate::operations::images::transform::warp::bilinear_sample_rgba;
use crate::output::Output;
use crate::value::Value;
use image::DynamicImage;
use image::imageops::FilterType;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use std::time::Instant;

/// Non-uniform blur operation with per-pixel radius controlled by a grayscale map.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpImageAdjustmentNonUniformBlur {}

impl OpImageAdjustmentNonUniformBlur {
    /// Returns the node metadata (name and description) for the non-uniform blur operation.
    pub fn settings() -> NodeSettings {
        NodeSettings {
            name: "non-uniform blur".to_string(),
            description: "Blurs with per-pixel intensity controlled by a grayscale map.".to_string(),
        }
    }

    /// Creates the input ports: source image, blur map (grayscale radius control),
    /// maximum blur intensity (pixels), and sample count per pixel.
    pub fn create_inputs() -> Vec<Input> {
        vec![
            Input::new("image".to_string(), Value::DynamicImage { data: default_image(), change_id: get_id() }, None, None),
            Input::new("blur map".to_string(), Value::DynamicImage { data: default_image(), change_id: get_id() }, None, None),
            Input::new("max intensity".to_string(), Value::Decimal(10.0), Some(InputSettings::Slider { range: (0.0, 50.0), step_by: Some(0.5), clamp_to_range: true }), None),
            Input::new("samples".to_string(), Value::Integer(16), Some(InputSettings::DragValue { speed: None, clamp: Some((1.0, 64.0)) }), None),
        ]
    }

    /// Creates the output port: the non-uniformly blurred image.
    pub fn create_outputs() -> Vec<Output> {
        vec![
            Output::new("output".to_string(), Value::DynamicImage { data: default_image(), change_id: get_id() }, None),
        ]
    }

    /// Executes the non-uniform blur. Resizes the blur map to match the source image,
    /// generates a Vogel disc sampling pattern, and averages bilinear samples per pixel.
    pub async fn run(inputs: &mut [Input]) -> Result<OperationResponse, OperationError> {
        let start_time = Instant::now();
        let mut input_errors: Vec<(usize, String)> = vec![];

        // convert inputs
        let image_converted = convert_input(inputs, 0, ValueType::DynamicImage, &mut input_errors);
        let blur_map_converted = convert_input(inputs, 1, ValueType::DynamicImage, &mut input_errors);
        let max_intensity_converted = convert_input(inputs, 2, ValueType::Decimal, &mut input_errors);
        let samples_converted = convert_input(inputs, 3, ValueType::Integer, &mut input_errors);

        // return if error
        if !input_errors.is_empty() { return Err(OperationError { input_errors, node_error: None }); }

        // get values
        let Value::DynamicImage { data, change_id: _ } = image_converted.unwrap() else { unreachable!() };
        let Value::DynamicImage { data: blur_map_data, change_id: _ } = blur_map_converted.unwrap() else { unreachable!() };
        let Value::Decimal(max_intensity) = max_intensity_converted.unwrap() else { unreachable!() };
        let Value::Integer(samples) = samples_converted.unwrap() else { unreachable!() };

        // run node
        let samples = samples.max(1) as u32;
        let max_intensity = max_intensity.max(0.0);

        let rgba = data.to_rgba8();
        let (width, height) = rgba.dimensions();

        // resize blur map to match source if needed
        let blur_map_resized = if blur_map_data.width() != width || blur_map_data.height() != height {
            blur_map_data.resize_exact(width, height, FilterType::Lanczos3)
        } else {
            (*blur_map_data).clone()
        };
        let blur_map_rgba = blur_map_resized.to_rgba8();

        // precompute concentric disc sample offsets (fixed pattern)
        // generate points in concentric rings for a unit disc
        let mut offsets: Vec<(f32, f32)> = Vec::with_capacity(samples as usize);
        if samples == 1 {
            offsets.push((0.0, 0.0));
        } else {
            // distribute points in rings using a sunflower/Vogel disc pattern
            let golden_angle: f32 = 2.399_963_2; // pi * (3 - sqrt(5))
            for i in 0..samples {
                let r = (i as f32 + 0.5).sqrt() / (samples as f32).sqrt();
                let theta = i as f32 * golden_angle;
                offsets.push((r * theta.cos(), r * theta.sin()));
            }
        }

        let mut output_buf = image::RgbaImage::new(width, height);

        for y in 0..height {
            for x in 0..width {
                // read blur map luminance for per-pixel radius
                let map_px = blur_map_rgba.get_pixel(x, y);
                let luminance = 0.299 * (map_px[0] as f32 / 255.0)
                    + 0.587 * (map_px[1] as f32 / 255.0)
                    + 0.114 * (map_px[2] as f32 / 255.0);
                let radius = luminance * max_intensity;

                let mut r_sum: f64 = 0.0;
                let mut g_sum: f64 = 0.0;
                let mut b_sum: f64 = 0.0;
                let mut a_sum: f64 = 0.0;

                for &(ox, oy) in &offsets {
                    let sx = x as f32 + ox * radius;
                    let sy = y as f32 + oy * radius;
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
#[path = "non_uniform_blur_tests.rs"]
mod tests;
