//! Histogram range remapping operation for images.
//!
//! Finds the actual minimum and maximum luminance in the image, then linearly
//! remaps all pixel values so the output spans a user-specified target range.

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

/// Histogram range operation that remaps pixel values to a target luminance range.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpImageAdjustmentHistogramRange{}

impl OpImageAdjustmentHistogramRange {
    /// Returns the node metadata (name and description) for the histogram range operation.
    pub fn settings() -> NodeSettings {
        NodeSettings {
            name: "histogram range".to_string(),
            description: "Remaps image luminance to a target range.".to_string(),
        }
    }

    /// Creates the input ports: image, target range min, and target range max.
    pub fn create_inputs() -> Vec<Input> {
        vec![
            Input::new("image".to_string(),  Value::DynamicImage { data:default_image(), change_id:get_id() }, None, None),
            Input::new("range min".to_string(), Value::Decimal(0.0), Some(InputSettings::Slider { range: (0.0, 1.0), step_by: Some(0.01), clamp_to_range: true }), None),
            Input::new("range max".to_string(), Value::Decimal(1.0), Some(InputSettings::Slider { range: (0.0, 1.0), step_by: Some(0.01), clamp_to_range: true }), None),
        ]
    }

    /// Creates the output port: the range-remapped image.
    pub fn create_outputs() -> Vec<Output> {
        vec![
            Output::new("output".to_string(), Value::DynamicImage { data:default_image(), change_id:get_id()}, None),
        ]
    }

    /// Executes the histogram range remapping. Scans for actual min/max, then linearly
    /// maps each channel from [actual_min, actual_max] to [range_min, range_max].
    pub async fn run(inputs: &mut [Input]) -> Result<OperationResponse, OperationError> {
        let start_time = Instant::now();
        let mut input_errors: Vec<(usize, String)> = vec![];

        // convert inputs
        let image_converted = convert_input(inputs, 0, ValueType::DynamicImage, &mut input_errors);
        let range_min_converted = convert_input(inputs, 1, ValueType::Decimal, &mut input_errors);
        let range_max_converted = convert_input(inputs, 2, ValueType::Decimal, &mut input_errors);

        // return if error
        if !input_errors.is_empty() { return Err(OperationError { input_errors, node_error: None }); }

        // get values
        let Value::DynamicImage{data, change_id:_} = image_converted.unwrap() else { unreachable!() };
        let Value::Decimal(range_min) = range_min_converted.unwrap() else { unreachable!() };
        let Value::Decimal(range_max) = range_max_converted.unwrap() else { unreachable!() };

        // run node
        let mut buffer = data.to_rgba32f();
        let range_min = range_min;
        let range_max = range_max;

        // find actual min/max luminance
        let mut actual_min: f32 = f32::MAX;
        let mut actual_max: f32 = f32::MIN;
        for pixel in buffer.pixels() {
            let lum = 0.2126 * pixel[0] + 0.7152 * pixel[1] + 0.0722 * pixel[2];
            if lum < actual_min { actual_min = lum; }
            if lum > actual_max { actual_max = lum; }
        }

        let actual_range = actual_max - actual_min;
        let target_range = range_max - range_min;

        for pixel in buffer.pixels_mut() {
            let alpha = pixel[3];
            for c in 0..3 {
                if actual_range <= 0.0 {
                    pixel[c] = range_min;
                } else {
                    let val = pixel[c];
                    let new_val = range_min + (val - actual_min) / actual_range * target_range;
                    pixel[c] = new_val.clamp(0.0, 1.0);
                }
            }
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

#[cfg(test)]
#[path = "histogram_range_tests.rs"]
mod tests;
