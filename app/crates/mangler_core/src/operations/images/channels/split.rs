//! Channel split operation.
//!
//! Decomposes an image into four separate 1-channel FloatImages, one per
//! channel (red, green, blue, alpha). Missing channels default to 0 (or 1 for alpha).

use crate::float_image::FloatImage;
use crate::get_id;
use crate::input::Input;
use crate::node_settings::NodeSettings;
use crate::convert_inputs;
use crate::operations::{OperationResponse, OperationError, OutputResponse, image_input, image_output};
use crate::output::Output;
use crate::value::Value;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use std::time::Instant;

use super::extract_channel;

/// Operation that splits an image into its individual R, G, B, and A channels.
/// Each output is a 1-channel FloatImage.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpImageChannelSplit {}

impl OpImageChannelSplit {
    pub fn settings() -> NodeSettings {
        NodeSettings {
            name: "channel split".to_string(),
            description: "Splits an image into R, G, B, A channels.".to_string(),
            help: "Emits four single-channel FloatImage outputs corresponding to red, green, blue, and alpha. For sources with fewer channels, missing colour components are zero-filled, while alpha defaults to 1.0 for RGB and 1 or 3-channel sources and to the second channel for 2-channel grayscale+alpha input.\n\nEach output has the same dimensions as the input but just one channel, making them directly usable as masks or as scalar inputs to nodes that accept grayscale. Pair with `channel merge` to rebuild after per-channel processing.".to_string(),
        }
    }

    pub fn create_inputs() -> Vec<Input> {
        vec![image_input("image")
            .with_description("Source image to decompose into individual channel images.")]
    }

    pub fn create_outputs() -> Vec<Output> {
        vec![
            image_output("red")
                .with_description("Single-channel image holding the source red channel."),
            image_output("green")
                .with_description("Single-channel image holding the source green channel."),
            image_output("blue")
                .with_description("Single-channel image holding the source blue channel."),
            image_output("alpha")
                .with_description("Single-channel image holding the source alpha (or 1.0 if absent)."),
        ]
    }

    /// Splits the input image into four 1-channel images (R, G, B, A).
    pub async fn run(inputs: &mut [Input]) -> Result<OperationResponse, OperationError> {
        let start_time = Instant::now();

        convert_inputs! { inputs;
            Image(data) = 0,
        }

        let (width, height) = data.dimensions();
        let ch = data.channels() as usize;
        let src = data.as_raw();
        let n = (width as usize) * (height as usize);

        // Extract each channel with the dispatch hoisted out of the pixel loop;
        // missing channels are constant-filled (0 for colour, 1 for alpha).
        let red = extract_channel(src, ch, |px| px[0]);
        let green = if ch >= 2 { extract_channel(src, ch, |px| px[1]) } else { vec![0.0; n] };
        let blue = if ch >= 3 { extract_channel(src, ch, |px| px[2]) } else { vec![0.0; n] };
        let alpha = match ch {
            2 => extract_channel(src, ch, |px| px[1]),
            4 => extract_channel(src, ch, |px| px[3]),
            _ => vec![1.0; n],
        };

        let red_buf = FloatImage::from_raw(width, height, 1, red).unwrap();
        let green_buf = FloatImage::from_raw(width, height, 1, green).unwrap();
        let blue_buf = FloatImage::from_raw(width, height, 1, blue).unwrap();
        let alpha_buf = FloatImage::from_raw(width, height, 1, alpha).unwrap();

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
