//! Histogram scan (luminance isolation) operation for images.
//!
//! Isolates a narrow band of luminance values from the image, producing a
//! grayscale mask. Uses smoothstep transitions at the edges of the band
//! to avoid hard cutoffs.

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

/// Histogram scan operation that isolates a luminance range into a grayscale mask.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpImageAdjustmentHistogramScan{}

impl OpImageAdjustmentHistogramScan {
    /// Returns the node metadata (name and description) for the histogram scan operation.
    pub fn settings() -> NodeSettings {
        NodeSettings {
            name: "histogram scan".to_string(),
            description: "Isolates a luminance range from the image.".to_string(),
        }
    }

    /// Creates the input ports: image, center position of the luminance band, and band width (range).
    pub fn create_inputs() -> Vec<Input> {
        vec![
            Input::new("image".to_string(),  Value::DynamicImage { data:default_image(), change_id:get_id() }, None, None),
            Input::new("position".to_string(), Value::Decimal(0.5), Some(InputSettings::Slider { range: (0.0, 1.0), step_by: Some(0.01), clamp_to_range: true }), None),
            Input::new("range".to_string(), Value::Decimal(0.1), Some(InputSettings::Slider { range: (0.0, 1.0), step_by: Some(0.01), clamp_to_range: true }), None),
        ]
    }

    /// Creates the output port: the luminance isolation mask.
    pub fn create_outputs() -> Vec<Output> {
        vec![
            Output::new("output".to_string(), Value::DynamicImage { data:default_image(), change_id:get_id()}, None),
        ]
    }

    /// Executes the histogram scan. Computes Rec. 709 luminance, then applies smoothstep
    /// transitions at the low and high edges of the selected band.
    pub async fn run(inputs: &mut [Input]) -> Result<OperationResponse, OperationError> {
        let start_time = Instant::now();
        let mut input_errors: Vec<(usize, String)> = vec![];

        // convert inputs
        let image_converted = convert_input(inputs, 0, ValueType::DynamicImage, &mut input_errors);
        let position_converted = convert_input(inputs, 1, ValueType::Decimal, &mut input_errors);
        let range_converted = convert_input(inputs, 2, ValueType::Decimal, &mut input_errors);

        // return if error
        if !input_errors.is_empty() { return Err(OperationError { input_errors, node_error: None }); }

        // get values
        let Value::DynamicImage{data, change_id:_} = image_converted.unwrap() else { unreachable!() };
        let Value::Decimal(position) = position_converted.unwrap() else { unreachable!() };
        let Value::Decimal(range) = range_converted.unwrap() else { unreachable!() };

        // run node
        let mut buffer = data.to_rgba32f();
        let position = position;
        let range = range;
        let low = position - range;
        let high = position + range;

        for pixel in buffer.pixels_mut() {
            // Rec. 709 luminance
            let lum = 0.2126 * pixel[0] + 0.7152 * pixel[1] + 0.0722 * pixel[2];
            let alpha = pixel[3];

            // smoothstep at boundaries for anti-aliasing
            let edge_width = 0.01_f32.max(range * 0.1);
            let low_edge = smoothstep(low - edge_width, low + edge_width, lum);
            let high_edge = 1.0 - smoothstep(high - edge_width, high + edge_width, lum);
            let result = low_edge * high_edge;

            pixel[0] = result;
            pixel[1] = result;
            pixel[2] = result;
            pixel[3] = alpha;
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

/// Hermite smoothstep interpolation between two edges, producing a smooth 0-to-1 transition.
fn smoothstep(edge0: f32, edge1: f32, x: f32) -> f32 {
    let t = ((x - edge0) / (edge1 - edge0)).clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

#[cfg(test)]
#[path = "histogram_scan_tests.rs"]
mod tests;
