//! Most common colors sampling operation.
//!
//! Analyzes an image to find the most frequently occurring colors by
//! quantizing each pixel's HSL representation and counting occurrences.
//! Returns the top 5 most common colors.

use crate::color::Color;
use crate::input::{Input, InputSettings};
use crate::node_settings::NodeSettings;
use crate::operations::{OperationResponse, OperationError, OutputResponse, convert_input};
use crate::output::Output;
use crate::value::{Value, ValueType};
use serde::{Deserialize, Serialize};
use std::time::Instant;
use std::collections::HashMap;

/// Operation that extracts the top 5 most common colors from an image.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpColorSampleMostCommonColors {}

impl OpColorSampleMostCommonColors {
    /// Returns the node metadata (name and description) for this operation.
    pub fn settings() -> NodeSettings {
        NodeSettings {
            name: "most common colors".to_string(),
            description: "Finds the most common colors in an image.".to_string(),
        }
    }

    /// Creates the input definitions: an image and quantization precision for hue, saturation, and lightness.
    pub fn create_inputs() -> Vec<Input> {
        vec![
            Input::new("image".to_string(), Value::Image{data:crate::operations::default_image(), change_id:crate::get_id()}, None, None),
            Input::new("hue quantization".to_string(), Value::Decimal(10.0), Some(InputSettings::Slider { range: (1.0, 100.0), step_by: Some(1.0), clamp_to_range: true}), None),
            Input::new("saturation quantization".to_string(), Value::Decimal(10.0), Some(InputSettings::Slider { range: (1.0, 100.0), step_by: Some(1.0), clamp_to_range: true}), None),
            Input::new("lightness quantization".to_string(), Value::Decimal(10.0), Some(InputSettings::Slider { range: (1.0, 100.0), step_by: Some(1.0), clamp_to_range: true}), None),
        ]
    }

    /// Creates 5 color output slots, one for each of the top most common colors.
    pub fn create_outputs() -> Vec<Output> {
        vec![
            Output::new("1".to_string(), Value::Color(Color::default()), None),
            Output::new("2".to_string(), Value::Color(Color::default()), None),
            Output::new("3".to_string(), Value::Color(Color::default()), None),
            Output::new("4".to_string(), Value::Color(Color::default()), None),
            Output::new("5".to_string(), Value::Color(Color::default()), None),
        ]
    }

    /// Executes the operation, scanning all pixels and returning the 5 most common quantized colors.
    pub async fn run(inputs: &mut [Input]) -> Result<OperationResponse, OperationError> {
        let start_time = Instant::now();
        let mut input_errors: Vec<(usize, String)> = vec![];

        // convert inputs
        let image_converted = convert_input(inputs, 0, ValueType::Image, &mut input_errors);
        let hue_precision_converted = convert_input(inputs, 1, ValueType::Decimal, &mut input_errors);
        let saturation_precision_converted = convert_input(inputs, 2, ValueType::Decimal, &mut input_errors);
        let lightness_precision_converted = convert_input(inputs, 3, ValueType::Decimal, &mut input_errors);


        // return if error
        if !input_errors.is_empty() { return Err(OperationError { input_errors, node_error: None }); }

        // get values
        let Value::Image{data:image, change_id:_} = image_converted.unwrap() else { unreachable!() };
        let Value::Decimal(hue_precision) = hue_precision_converted.unwrap() else { unreachable!() };
        let Value::Decimal(saturation_precision) = saturation_precision_converted.unwrap() else { unreachable!() };
        let Value::Decimal(lightness_precision) = lightness_precision_converted.unwrap() else { unreachable!() };

        // Quantize each pixel's HSL values into buckets and count occurrences.
        // Higher precision values produce more buckets (finer color distinction).
        let mut color_counts: HashMap<[i32; 3], u32> = HashMap::new();

        let ch = image.channels() as usize;
        for pixel in image.pixels() {
            // Extract RGB from any channel count
            let (r, g, b) = if ch >= 3 {
                (pixel[0], pixel[1], pixel[2])
            } else {
                (pixel[0], pixel[0], pixel[0])
            };
            let color = Color::from_srgb_float(r, g, b, 1.0);
            let hsl = color.to_hsl();
            // Round each channel to its quantized bucket index
            let h = ((hsl.0 / 360.0) * hue_precision).round() as i32;
            let s = (hsl.1 * saturation_precision).round() as i32;
            let l = (hsl.2 * lightness_precision).round() as i32;
            *color_counts.entry([h, s, l]).or_insert(0) += 1;
        }

        // Sort buckets by pixel count (most frequent first)
        let mut sorted_colors: Vec<(&[i32; 3], &u32)> = color_counts.iter().collect();
        sorted_colors.sort_by(|a, b| b.1.cmp(a.1));

        let mut responses: Vec<OutputResponse> = Vec::new();

        // Convert the top 5 quantized HSL buckets back to colors
        for (hsl, _count) in sorted_colors.iter().take(5) {
            let h = ((hsl[0] as f32) / hue_precision) * 360.0;
            let s = (hsl[1] as f32) / saturation_precision;
            let l = (hsl[2] as f32) / lightness_precision;
            responses.push(OutputResponse {
                value: Value::Color(Color::from_hsl(h, s, l, 1.0)),
            });
        }

        // Pad with default colors if fewer than 5 distinct buckets exist
        while responses.len() < 5 {
            responses.push(OutputResponse {
                value: Value::Color(Color::default()),
            });
        }

        Ok(OperationResponse {
            time: Instant::now().duration_since(start_time),
            responses,
        })
    }
}

#[cfg(test)]
#[path = "most_common_colors_tests.rs"]
mod tests;
