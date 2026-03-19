//! Channel split operation.
//!
//! Decomposes an RGBA image into four separate grayscale images, one per
//! channel (red, green, blue, alpha). Each output image stores the channel
//! value replicated across R, G, and B with full opacity.

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

/// Operation that splits an image into its individual R, G, B, and A channels.
///
/// Each output is a grayscale image where the channel value is replicated
/// across all three RGB components (e.g., the red output has `[r, r, r, 255]`
/// per pixel).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpImageChannelSplit {}

impl OpImageChannelSplit {
    /// Returns the node metadata (name and description) for this operation.
    pub fn settings() -> NodeSettings {
        NodeSettings {
            name: "channel split".to_string(),
            description: "Splits an image into R, G, B, A channels.".to_string(),
        }
    }

    /// Creates the input definitions: a single RGBA image to split.
    pub fn create_inputs() -> Vec<Input> {
        vec![
            Input::new("image".to_string(), Value::DynamicImage { data: default_image(), change_id: get_id() }, None, None),
        ]
    }

    /// Creates the output definitions: four grayscale images (red, green, blue, alpha).
    pub fn create_outputs() -> Vec<Output> {
        vec![
            Output::new("red".to_string(), Value::DynamicImage { data: default_image(), change_id: get_id() }, None),
            Output::new("green".to_string(), Value::DynamicImage { data: default_image(), change_id: get_id() }, None),
            Output::new("blue".to_string(), Value::DynamicImage { data: default_image(), change_id: get_id() }, None),
            Output::new("alpha".to_string(), Value::DynamicImage { data: default_image(), change_id: get_id() }, None),
        ]
    }

    /// Executes the operation: splits the input image into four channel images.
    ///
    /// Each channel value is replicated across RGB in the output to produce
    /// a viewable grayscale representation of that channel.
    pub async fn run(inputs: &mut [Input]) -> Result<OperationResponse, OperationError> {
        let start_time = Instant::now();
        let mut input_errors: Vec<(usize, String)> = vec![];

        // convert inputs
        let image_converted = convert_input(inputs, 0, ValueType::DynamicImage, &mut input_errors);

        // return if error
        if !input_errors.is_empty() { return Err(OperationError { input_errors, node_error: None }); }

        // get values
        let Value::DynamicImage{data, change_id:_} = image_converted.unwrap() else { unreachable!() };

        // run node
        let rgba = data.to_rgba8();
        let (width, height) = rgba.dimensions();

        let mut red_buf = RgbaImage::new(width, height);
        let mut green_buf = RgbaImage::new(width, height);
        let mut blue_buf = RgbaImage::new(width, height);
        let mut alpha_buf = RgbaImage::new(width, height);

        // Write each channel value to all three RGB components of its output buffer
        for (x, y, pixel) in rgba.enumerate_pixels() {
            let [r, g, b, a] = pixel.0;
            red_buf.put_pixel(x, y, image::Rgba([r, r, r, 255]));
            green_buf.put_pixel(x, y, image::Rgba([g, g, g, 255]));
            blue_buf.put_pixel(x, y, image::Rgba([b, b, b, 255]));
            alpha_buf.put_pixel(x, y, image::Rgba([a, a, a, 255]));
        }

        Ok(OperationResponse {
            time: Instant::now().duration_since(start_time),
            responses: vec![
                OutputResponse { value: Value::DynamicImage { data: Arc::new(image::DynamicImage::ImageRgba8(red_buf)), change_id: get_id() } },
                OutputResponse { value: Value::DynamicImage { data: Arc::new(image::DynamicImage::ImageRgba8(green_buf)), change_id: get_id() } },
                OutputResponse { value: Value::DynamicImage { data: Arc::new(image::DynamicImage::ImageRgba8(blue_buf)), change_id: get_id() } },
                OutputResponse { value: Value::DynamicImage { data: Arc::new(image::DynamicImage::ImageRgba8(alpha_buf)), change_id: get_id() } },
            ],
        })
    }
}

#[cfg(test)]
#[path = "split_tests.rs"]
mod tests;
