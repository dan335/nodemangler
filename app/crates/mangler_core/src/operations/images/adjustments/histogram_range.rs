//! Histogram range remapping operation for images.
//!
//! Finds the actual minimum and maximum luminance in the image, then linearly
//! remaps all pixel values so the output spans a user-specified target range.

use crate::get_id;
use crate::input::{Input, InputSettings};
use crate::node_settings::NodeSettings;
use crate::convert_inputs;
use crate::operations::{OperationResponse, OperationError, OutputResponse, default_image, image_input};
use crate::output::Output;
use crate::value::Value;
use rayon::prelude::*;
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
            help: "Scans the image once to find the actual minimum and maximum luminance (Rec. 709 weighted for RGB, single channel for grayscale), then linearly remaps every colour channel from the detected span into the user's target min-max interval.\n\nIf the source luminance is flat (actual max equals actual min), every pixel collapses to range min. Output channels are clamped to 0-1 and alpha is preserved. Use this to compress or expand dynamic range into a predictable band, for example to squeeze a noise pattern into the 0.4-0.6 range before blending.".to_string(),
        }
    }

    /// Creates the input ports: image, target range min, and target range max.
    pub fn create_inputs() -> Vec<Input> {
        vec![
            image_input("image")
                .with_description("Source image whose actual luminance span is rescaled."),
            Input::new("range min".to_string(), Value::Decimal(0.0), Some(InputSettings::Slider { range: (0.0, 1.0), step_by: Some(0.01), clamp_to_range: true }), None)
                .with_description("Value the image's darkest pixels will be mapped to."),
            Input::new("range max".to_string(), Value::Decimal(1.0), Some(InputSettings::Slider { range: (0.0, 1.0), step_by: Some(0.01), clamp_to_range: true }), None)
                .with_description("Value the image's brightest pixels will be mapped to."),
        ]
    }

    /// Creates the output port: the range-remapped image.
    pub fn create_outputs() -> Vec<Output> {
        vec![
            Output::new("output".to_string(), Value::Image { data:default_image(), change_id:get_id()}, None)
                .with_description("Image linearly remapped so its tones span the target min–max range."),
        ]
    }

    /// Executes the histogram range remapping. Scans for actual min/max, then linearly
    /// maps each channel from [actual_min, actual_max] to [range_min, range_max].
    pub async fn run(inputs: &mut [Input]) -> Result<OperationResponse, OperationError> {
        let start_time = Instant::now();

        convert_inputs! { inputs;
            Image(data) = 0,
            Decimal(range_min) = 1,
            Decimal(range_max) = 2,
        }

        // run node — clone and work directly on FloatImage
        let mut result = (*data).clone();
        let ch = result.channels() as usize;
        let color_ch = if ch == 2 || ch == 4 { ch - 1 } else { ch };

        // find actual min/max luminance
        let mut actual_min: f32 = f32::MAX;
        let mut actual_max: f32 = f32::MIN;
        for pixel in result.pixels() {
            let lum = if color_ch >= 3 {
                crate::luma::rec709(pixel[0], pixel[1], pixel[2])
            } else {
                pixel[0]
            };
            if lum < actual_min { actual_min = lum; }
            if lum > actual_max { actual_max = lum; }
        }

        let actual_range = actual_max - actual_min;
        let target_range = range_max - range_min;

        result.par_pixels_mut().for_each(|pixel| {
            for val in pixel.iter_mut().take(color_ch) {
                if actual_range <= 0.0 {
                    *val = range_min;
                } else {
                    let new_val = range_min + (*val - actual_min) / actual_range * target_range;
                    *val = new_val.clamp(0.0, 1.0);
                }
            }
            // alpha unchanged
        });

        Ok(OperationResponse { 
            time: Instant::now().duration_since(start_time),
            responses: vec![
                OutputResponse {value: Value::Image { data:Arc::new(result), change_id:get_id() }},
            ],
        })
    }
}

#[cfg(test)]
#[path = "histogram_range_tests.rs"]
mod tests;
