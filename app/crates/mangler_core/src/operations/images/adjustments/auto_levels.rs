//! Automatic levels adjustment operation for images.
//!
//! Analyzes the image histogram to find the actual luminance range, then
//! remaps pixel values to fill the full [0, 1] range. Configurable clip
//! percentages allow ignoring outlier pixels at both ends.

use crate::get_id;
use crate::value::ValueType;
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
            help: "Builds a 256-bin luminance histogram (Rec. 709 weighted for RGB, single channel for grayscale), then walks it from both ends to locate the black and white points after discarding the requested fractions of outlier pixels.\n\nThe remaining tonal range is linearly stretched so that the black point becomes 0 and the white point becomes 1, with every colour channel remapped by the same formula. Alpha is preserved. If the detected white point is not greater than the black point, the image is returned unchanged. Use higher clip fractions to ignore specular highlights or crushed shadows that would otherwise anchor the stretch.".to_string(),
        }
    }

    /// Creates the input ports: image and clip percentages for black and white ends of the histogram.
    pub fn create_inputs() -> Vec<Input> {
        vec![
            Input::new("image".to_string(),  Value::Image { data:default_image(), change_id:get_id() }, None, None)
                .with_description("Source image whose histogram is analyzed and stretched."),
            Input::new("clip black".to_string(), Value::Decimal(0.005), Some(InputSettings::Slider { range: (0.0, 0.5), step_by: Some(0.001), clamp_to_range: true }), None)
                .with_description("Fraction of darkest pixels to discard when finding the black point."),
            Input::new("clip white".to_string(), Value::Decimal(0.005), Some(InputSettings::Slider { range: (0.0, 0.5), step_by: Some(0.001), clamp_to_range: true }), None)
                .with_description("Fraction of brightest pixels to discard when finding the white point."),
        ]
    }

    /// Creates the output port: the auto-levels-adjusted image.
    pub fn create_outputs() -> Vec<Output> {
        vec![
            Output::new("output".to_string(), Value::Image { data:default_image(), change_id:get_id()}, None)
                .with_description("Image remapped so its tonal range fills the full 0–1 interval."),
        ]
    }

    /// Executes the auto levels adjustment. Builds a 256-bin luminance histogram,
    /// finds clip-adjusted black and white points, then linearly remaps all channels.
    pub async fn run(inputs: &mut [Input]) -> Result<OperationResponse, OperationError> {
        let start_time = Instant::now();
        let mut input_errors: Vec<(usize, String)> = vec![];

        // convert inputs
        let image_converted = convert_input(inputs, 0, ValueType::Image, &mut input_errors);
        let clip_black_converted = convert_input(inputs, 1, ValueType::Decimal, &mut input_errors);
        let clip_white_converted = convert_input(inputs, 2, ValueType::Decimal, &mut input_errors);

        // return if error
        if !input_errors.is_empty() { return Err(OperationError { input_errors, node_error: None }); }

        // get values
        let Value::Image{data, change_id:_} = image_converted.unwrap() else { unreachable!() };
        let Value::Decimal(clip_black) = clip_black_converted.unwrap() else { unreachable!() };
        let Value::Decimal(clip_white) = clip_white_converted.unwrap() else { unreachable!() };

        // run node — data is already f32, clone and work directly
        let mut result = (*data).clone();
        let ch = result.channels() as usize;
        let color_ch = if ch == 2 || ch == 4 { ch - 1 } else { ch };

        // build 256-bin histogram of luminance values
        let mut histogram = [0u32; 256];
        let total_pixels = (result.width() * result.height()) as f32;
        for pixel in result.pixels() {
            // Compute luminance from available color channels
            let lum = if color_ch >= 3 {
                0.2126 * pixel[0] + 0.7152 * pixel[1] + 0.0722 * pixel[2]
            } else {
                pixel[0]
            };
            let bin = (lum * 255.0).clamp(0.0, 255.0) as usize;
            histogram[bin] += 1;
        }

        // find black point: luminance where clip_black fraction of pixels are below
        let black_threshold = (clip_black * total_pixels) as u32;
        let mut cumulative = 0u32;
        let mut black_point = 0.0_f32;
        for (i, &count) in histogram.iter().enumerate() {
            cumulative += count;
            // Require the bin to actually contain pixels. Otherwise a clip
            // fraction that rounds to threshold 0 is satisfied on the first
            // (empty) bin, pinning the black point to 0.0 and turning the whole
            // stretch into an identity instead of a min→0 stretch.
            if count > 0 && cumulative >= black_threshold {
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
            // Same guard as the black point: skip empty bins so a threshold that
            // rounds to 0 does not pin the white point to 1.0 (identity).
            if count > 0 && cumulative >= white_threshold {
                white_point = i as f32 / 255.0;
                break;
            }
        }

        // remap if valid range
        if white_point > black_point {
            let range = white_point - black_point;
            for pixel in result.pixels_mut() {
                for c in 0..color_ch {
                    let val = pixel[c];
                    pixel[c] = ((val - black_point) / range).clamp(0.0, 1.0);
                }
                // alpha unchanged
            }
        }

        Ok(OperationResponse { 
            time: Instant::now().duration_since(start_time),
            responses: vec![
                OutputResponse {value: Value::Image { data:Arc::new(result), change_id:get_id() }},
            ],
        })
    }
}

#[cfg(test)]
#[path = "auto_levels_tests.rs"]
mod tests;
