//! Automatic levels adjustment operation for images.
//!
//! Analyzes the image histogram to find the actual luminance range, then
//! remaps pixel values to fill the full [0, 1] range. Configurable clip
//! percentages allow ignoring outlier pixels at both ends.

use crate::get_id;
use crate::value::ValueType;
use image::DynamicImage;
use crate::input::{Input, InputSettings};
use crate::node_settings::NodeSettings;
use crate::operations::{OperationResponse, OperationError, OutputResponse, default_image, convert_input};
use crate::output::Output;
use crate::value::Value;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use std::time::Instant;

/// Automatic levels adjustment that stretches the histogram to fill the full tonal range.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpImageAdjustmentAutoLevels{}

impl OpImageAdjustmentAutoLevels {
    /// Returns the node metadata (name and description) for the auto levels operation.
    pub fn settings() -> NodeSettings {
        NodeSettings {
            name: "auto levels".to_string(),
            description: "Automatically adjusts white and black points.".to_string(),
        }
    }

    /// Creates the input ports: image and clip percentages for black and white ends of the histogram.
    pub fn create_inputs() -> Vec<Input> {
        vec![
            Input::new("image".to_string(),  Value::DynamicImage { data:default_image(), change_id:get_id() }, None, None),
            Input::new("clip black".to_string(), Value::Decimal(0.005), Some(InputSettings::Slider { range: (0.0, 0.5), step_by: Some(0.001), clamp_to_range: true }), None),
            Input::new("clip white".to_string(), Value::Decimal(0.005), Some(InputSettings::Slider { range: (0.0, 0.5), step_by: Some(0.001), clamp_to_range: true }), None),
        ]
    }

    /// Creates the output port: the auto-levels-adjusted image.
    pub fn create_outputs() -> Vec<Output> {
        vec![
            Output::new("output".to_string(), Value::DynamicImage { data:default_image(), change_id:get_id()}, None),
        ]
    }

    /// Executes the auto levels adjustment. Builds a 256-bin luminance histogram,
    /// finds clip-adjusted black and white points, then linearly remaps all channels.
    pub async fn run(inputs: &mut [Input]) -> Result<OperationResponse, OperationError> {
        let start_time = Instant::now();
        let mut input_errors: Vec<(usize, String)> = vec![];

        // convert inputs
        let image_converted = convert_input(inputs, 0, ValueType::DynamicImage, &mut input_errors);
        let clip_black_converted = convert_input(inputs, 1, ValueType::Decimal, &mut input_errors);
        let clip_white_converted = convert_input(inputs, 2, ValueType::Decimal, &mut input_errors);

        // return if error
        if !input_errors.is_empty() { return Err(OperationError { input_errors, node_error: None }); }

        // get values
        let Value::DynamicImage{data, change_id:_} = image_converted.unwrap() else { unreachable!() };
        let Value::Decimal(clip_black) = clip_black_converted.unwrap() else { unreachable!() };
        let Value::Decimal(clip_white) = clip_white_converted.unwrap() else { unreachable!() };

        // run node
        let mut buffer = data.to_rgba32f();
        let clip_black = clip_black;
        let clip_white = clip_white;

        // build 256-bin histogram of luminance values
        let mut histogram = [0u32; 256];
        let total_pixels = buffer.pixels().len() as f32;
        for pixel in buffer.pixels() {
            let lum = 0.2126 * pixel[0] + 0.7152 * pixel[1] + 0.0722 * pixel[2];
            let bin = (lum * 255.0).clamp(0.0, 255.0) as usize;
            histogram[bin] += 1;
        }

        // find black point: luminance where clip_black fraction of pixels are below
        let black_threshold = (clip_black * total_pixels) as u32;
        let mut cumulative = 0u32;
        let mut black_point = 0.0_f32;
        for (i, &count) in histogram.iter().enumerate() {
            cumulative += count;
            if cumulative >= black_threshold {
                black_point = i as f32 / 255.0;
                break;
            }
        }

        // find white point: luminance where clip_white fraction of pixels are above
        let white_threshold = (clip_white * total_pixels) as u32;
        cumulative = 0;
        let mut white_point = 1.0_f32;
        for (i, &count) in histogram.iter().enumerate().rev() {
            cumulative += count;
            if cumulative >= white_threshold {
                white_point = i as f32 / 255.0;
                break;
            }
        }

        // remap if valid range
        if white_point > black_point {
            let range = white_point - black_point;
            for pixel in buffer.pixels_mut() {
                for c in 0..3 {
                    let val = pixel[c];
                    pixel[c] = ((val - black_point) / range).clamp(0.0, 1.0);
                }
                // alpha unchanged
            }
        }

        let adjusted = DynamicImage::ImageRgba32F(buffer);

        Ok(OperationResponse {
            time: Instant::now().duration_since(start_time),
            responses: vec![
                OutputResponse {value: Value::DynamicImage { data:Arc::new(adjusted), change_id:get_id() }},
            ],
        })
    }
}

#[cfg(test)]
#[path = "auto_levels_tests.rs"]
mod tests;
