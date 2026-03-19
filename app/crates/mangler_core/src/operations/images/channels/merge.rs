//! Channel merge operation.
//!
//! Recombines four separate grayscale images (one per channel) into a single
//! RGBA image. Each input is converted to luminance (grayscale) to extract
//! the channel value. The output dimensions match the red channel input;
//! other channels that are smaller default to 0 (or 255 for alpha).

use crate::get_id;
use crate::value::ValueType;
use image::RgbaImage;
use crate::input::Input;
use crate::node_settings::NodeSettings;
use crate::operations::{OperationResponse, OperationError, OutputResponse, default_image, convert_input};
use crate::output::Output;
use crate::value::Value;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use std::time::Instant;

/// Operation that merges four channel images into a single RGBA image.
///
/// Each input image is converted to grayscale (luma8) to extract a single
/// channel value. The output image dimensions are determined by the red
/// channel input. If other channel images are smaller, out-of-bounds pixels
/// default to 0 for RGB and 255 for alpha.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpImageChannelMerge {}

impl OpImageChannelMerge {
    /// Returns the node metadata (name and description) for this operation.
    pub fn settings() -> NodeSettings {
        NodeSettings {
            name: "channel merge".to_string(),
            description: "Merges R, G, B, A channel images into one RGBA image.".to_string(),
        }
    }

    /// Creates the input definitions: four images for the red, green, blue, and alpha channels.
    pub fn create_inputs() -> Vec<Input> {
        vec![
            Input::new("red".to_string(), Value::DynamicImage { data: default_image(), change_id: get_id() }, None, None),
            Input::new("green".to_string(), Value::DynamicImage { data: default_image(), change_id: get_id() }, None, None),
            Input::new("blue".to_string(), Value::DynamicImage { data: default_image(), change_id: get_id() }, None, None),
            Input::new("alpha".to_string(), Value::DynamicImage { data: default_image(), change_id: get_id() }, None, None),
        ]
    }

    /// Creates the output definitions: the merged RGBA image.
    pub fn create_outputs() -> Vec<Output> {
        vec![
            Output::new("output".to_string(), Value::DynamicImage { data: default_image(), change_id: get_id() }, None),
        ]
    }

    /// Executes the operation: converts each input to grayscale and assembles the RGBA image.
    ///
    /// Uses the red channel's dimensions for the output. Channels smaller than the
    /// output default to 0 (or 255 for alpha) at out-of-bounds coordinates.
    pub async fn run(inputs: &mut [Input]) -> Result<OperationResponse, OperationError> {
        let start_time = Instant::now();
        let mut input_errors: Vec<(usize, String)> = vec![];

        // convert inputs
        let red_converted = convert_input(inputs, 0, ValueType::DynamicImage, &mut input_errors);
        let green_converted = convert_input(inputs, 1, ValueType::DynamicImage, &mut input_errors);
        let blue_converted = convert_input(inputs, 2, ValueType::DynamicImage, &mut input_errors);
        let alpha_converted = convert_input(inputs, 3, ValueType::DynamicImage, &mut input_errors);

        // return if error
        if !input_errors.is_empty() { return Err(OperationError { input_errors, node_error: None }); }

        // get values
        let Value::DynamicImage{data:red_data, change_id:_} = red_converted.unwrap() else { unreachable!() };
        let Value::DynamicImage{data:green_data, change_id:_} = green_converted.unwrap() else { unreachable!() };
        let Value::DynamicImage{data:blue_data, change_id:_} = blue_converted.unwrap() else { unreachable!() };
        let Value::DynamicImage{data:alpha_data, change_id:_} = alpha_converted.unwrap() else { unreachable!() };

        // run node — convert each input to single-channel grayscale
        let red_luma = red_data.to_luma8();
        let green_luma = green_data.to_luma8();
        let blue_luma = blue_data.to_luma8();
        let alpha_luma = alpha_data.to_luma8();

        // Use the red channel's dimensions as the output size
        let (width, height) = red_luma.dimensions();
        let mut output = RgbaImage::new(width, height);

        for y in 0..height {
            for x in 0..width {
                let r = red_luma.get_pixel(x, y).0[0];
                // Fall back to 0 for RGB and 255 for alpha when the channel image is smaller
                let g = if x < green_luma.width() && y < green_luma.height() { green_luma.get_pixel(x, y).0[0] } else { 0 };
                let b = if x < blue_luma.width() && y < blue_luma.height() { blue_luma.get_pixel(x, y).0[0] } else { 0 };
                let a = if x < alpha_luma.width() && y < alpha_luma.height() { alpha_luma.get_pixel(x, y).0[0] } else { 255 };
                output.put_pixel(x, y, image::Rgba([r, g, b, a]));
            }
        }

        Ok(OperationResponse {
            time: Instant::now().duration_since(start_time),
            responses: vec![
                OutputResponse { value: Value::DynamicImage { data: Arc::new(image::DynamicImage::ImageRgba8(output)), change_id: get_id() } },
            ],
        })
    }
}

#[cfg(test)]
#[path = "merge_tests.rs"]
mod tests;
