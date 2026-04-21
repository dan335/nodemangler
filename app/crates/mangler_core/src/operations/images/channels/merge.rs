//! Channel merge operation.
//!
//! Recombines four separate images (one per channel) into a single
//! 4-channel RGBA FloatImage. Each input's first channel is used as
//! the channel value.

use crate::float_image::FloatImage;
use crate::get_id;
use crate::value::ValueType;
use crate::input::Input;
use crate::node_settings::NodeSettings;
use crate::operations::{OperationResponse, OperationError, OutputResponse, default_image, convert_input};
use crate::output::Output;
use crate::value::Value;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use std::time::Instant;

/// Operation that merges four channel images into a single RGBA image.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpImageChannelMerge {}

impl OpImageChannelMerge {
    pub fn settings() -> NodeSettings {
        NodeSettings { name: "channel merge".to_string(), description: "Merges R, G, B, A channel images into one RGBA image.".to_string() }
    }

    pub fn create_inputs() -> Vec<Input> {
        vec![
            Input::new("red".to_string(), Value::Image { data: default_image(), change_id: get_id() }, None, None),
            Input::new("green".to_string(), Value::Image { data: default_image(), change_id: get_id() }, None, None),
            Input::new("blue".to_string(), Value::Image { data: default_image(), change_id: get_id() }, None, None),
            Input::new("alpha".to_string(), Value::Image { data: default_image(), change_id: get_id() }, None, None),
        ]
    }

    pub fn create_outputs() -> Vec<Output> {
        vec![Output::new("output".to_string(), Value::Image { data: default_image(), change_id: get_id() }, None)]
    }

    /// Merges four images by taking each one's first channel (or luminance) as an RGBA component.
    pub async fn run(inputs: &mut [Input]) -> Result<OperationResponse, OperationError> {
        let start_time = Instant::now();
        let mut input_errors: Vec<(usize, String)> = vec![];

        let red_converted = convert_input(inputs, 0, ValueType::Image, &mut input_errors);
        let green_converted = convert_input(inputs, 1, ValueType::Image, &mut input_errors);
        let blue_converted = convert_input(inputs, 2, ValueType::Image, &mut input_errors);
        let alpha_converted = convert_input(inputs, 3, ValueType::Image, &mut input_errors);

        if !input_errors.is_empty() { return Err(OperationError { input_errors, node_error: None }); }

        let Value::Image{data:red_data, change_id:_} = red_converted.unwrap() else { unreachable!() };
        let Value::Image{data:green_data, change_id:_} = green_converted.unwrap() else { unreachable!() };
        let Value::Image{data:blue_data, change_id:_} = blue_converted.unwrap() else { unreachable!() };
        let Value::Image{data:alpha_data, change_id:_} = alpha_converted.unwrap() else { unreachable!() };

        // Helper: extract the first-channel or luminance value from a FloatImage pixel
        let channel_val = |img: &FloatImage, x: u32, y: u32| -> f32 {
            if x >= img.width() || y >= img.height() { return 0.0; }
            let px = img.get_pixel(x, y);
            let ch = img.channels() as usize;
            if ch >= 3 { 0.299 * px[0] + 0.587 * px[1] + 0.114 * px[2] } else { px[0] }
        };

        // Use the red channel's dimensions as the output size
        let (width, height) = red_data.dimensions();
        let mut output = FloatImage::new(width, height, 4);

        for y in 0..height {
            for x in 0..width {
                let r = channel_val(&red_data, x, y);
                let g = channel_val(&green_data, x, y);
                let b = channel_val(&blue_data, x, y);
                // Alpha defaults to 1.0 for out-of-bounds pixels
                let a = if x < alpha_data.width() && y < alpha_data.height() {
                    channel_val(&alpha_data, x, y)
                } else { 1.0 };
                output.put_pixel(x, y, &[r, g, b, a]);
            }
        }

        Ok(OperationResponse { 
            time: Instant::now().duration_since(start_time),
            responses: vec![OutputResponse { value: Value::Image { data: Arc::new(output), change_id: get_id() } }],
        })
    }
}

#[cfg(test)]
#[path = "merge_tests.rs"]
mod tests;
