//! Channel split operation.
//!
//! Decomposes an image into four separate 1-channel FloatImages, one per
//! channel (red, green, blue, alpha). Missing channels default to 0 (or 1 for alpha).

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

/// Operation that splits an image into its individual R, G, B, and A channels.
/// Each output is a 1-channel FloatImage.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpImageChannelSplit {}

impl OpImageChannelSplit {
    pub fn settings() -> NodeSettings {
        NodeSettings { name: "channel split".to_string(), description: "Splits an image into R, G, B, A channels.".to_string() }
    }

    pub fn create_inputs() -> Vec<Input> {
        vec![Input::new("image".to_string(), Value::Image { data: default_image(), change_id: get_id() }, None, None)]
    }

    pub fn create_outputs() -> Vec<Output> {
        vec![
            Output::new("red".to_string(), Value::Image { data: default_image(), change_id: get_id() }, None),
            Output::new("green".to_string(), Value::Image { data: default_image(), change_id: get_id() }, None),
            Output::new("blue".to_string(), Value::Image { data: default_image(), change_id: get_id() }, None),
            Output::new("alpha".to_string(), Value::Image { data: default_image(), change_id: get_id() }, None),
        ]
    }

    /// Splits the input image into four 1-channel images (R, G, B, A).
    pub async fn run(inputs: &mut [Input]) -> Result<OperationResponse, OperationError> {
        let start_time = Instant::now();
        let mut input_errors: Vec<(usize, String)> = vec![];

        let image_converted = convert_input(inputs, 0, ValueType::Image, &mut input_errors);
        if !input_errors.is_empty() { return Err(OperationError { input_errors, node_error: None }); }

        let Value::Image{data, change_id:_} = image_converted.unwrap() else { unreachable!() };

        let (width, height) = data.dimensions();
        let ch = data.channels() as usize;

        let mut red_buf = FloatImage::new(width, height, 1);
        let mut green_buf = FloatImage::new(width, height, 1);
        let mut blue_buf = FloatImage::new(width, height, 1);
        let mut alpha_buf = FloatImage::new(width, height, 1);

        // Extract each channel, defaulting missing channels
        for y in 0..height {
            for x in 0..width {
                let px = data.get_pixel(x, y);
                let r = px[0];
                let g = if ch >= 2 { px[1] } else { 0.0 };
                let b = if ch >= 3 { px[2] } else { 0.0 };
                let a = if ch == 2 { px[1] } else if ch == 4 { px[3] } else { 1.0 };

                red_buf.put_pixel(x, y, &[r]);
                green_buf.put_pixel(x, y, &[g]);
                blue_buf.put_pixel(x, y, &[b]);
                alpha_buf.put_pixel(x, y, &[a]);
            }
        }

        Ok(OperationResponse {
            time: Instant::now().duration_since(start_time),
            responses: vec![
                OutputResponse { value: Value::Image { data: Arc::new(red_buf), change_id: get_id() } },
                OutputResponse { value: Value::Image { data: Arc::new(green_buf), change_id: get_id() } },
                OutputResponse { value: Value::Image { data: Arc::new(blue_buf), change_id: get_id() } },
                OutputResponse { value: Value::Image { data: Arc::new(alpha_buf), change_id: get_id() } },
            ],
        })
    }
}

#[cfg(test)]
#[path = "split_tests.rs"]
mod tests;
