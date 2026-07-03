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
use rayon::prelude::*;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use std::time::Instant;

/// Minimum pixel count before extraction is parallelized; below this the
/// rayon dispatch overhead outweighs the trivial per-pixel work.
const PARALLEL_PIXELS: usize = 1 << 16;

/// Extracts one scalar per pixel from an interleaved raw buffer.
fn extract_channel<F: Fn(&[f32]) -> f32 + Sync>(src: &[f32], ch: usize, f: F) -> Vec<f32> {
    if src.len() / ch >= PARALLEL_PIXELS {
        src.par_chunks_exact(ch).map(&f).collect()
    } else {
        src.chunks_exact(ch).map(f).collect()
    }
}

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
        vec![Input::new("image".to_string(), Value::Image { data: default_image(), change_id: get_id() }, None, None)
            .with_description("Source image to decompose into individual channel images.")]
    }

    pub fn create_outputs() -> Vec<Output> {
        vec![
            Output::new("red".to_string(), Value::Image { data: default_image(), change_id: get_id() }, None)
                .with_description("Single-channel image holding the source red channel."),
            Output::new("green".to_string(), Value::Image { data: default_image(), change_id: get_id() }, None)
                .with_description("Single-channel image holding the source green channel."),
            Output::new("blue".to_string(), Value::Image { data: default_image(), change_id: get_id() }, None)
                .with_description("Single-channel image holding the source blue channel."),
            Output::new("alpha".to_string(), Value::Image { data: default_image(), change_id: get_id() }, None)
                .with_description("Single-channel image holding the source alpha (or 1.0 if absent)."),
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
