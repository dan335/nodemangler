//! Single channel extraction.
//!
//! Pulls one source channel (R/G/B/A/luminance) out of an image and emits
//! it as a 1-channel grayscale image. `channel shuffle` can also relocate
//! channels but always produces RGBA output; this node is the common
//! "just give me the alpha channel as grayscale" case.

use crate::float_image::FloatImage;
use crate::get_id;
use crate::input::{Input, InputSettings};
use crate::node_settings::NodeSettings;
use crate::operations::{OperationResponse, OperationError, OutputResponse, default_image, convert_input};
use crate::output::Output;
use crate::value::{Value, ValueType};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use std::time::Instant;

/// Extracts one channel of an image as a 1-channel grayscale image.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpImageChannelSelect {}

impl OpImageChannelSelect {
    pub fn settings() -> NodeSettings {
        NodeSettings {
            name: "channel select".to_string(),
            description: "Extracts a single channel of an image as a 1-channel grayscale output.".to_string(),
            help: "Reads one source channel — 0 = R, 1 = G, 2 = B, 3 = A, 4 = luminance — and emits it as a 1-channel FloatImage. When the requested channel is out of range for the input (e.g. picking B on a single-channel image) the first channel is used and zero is substituted for any missing index.\n\nFor luminance (channel = 4), Rec. 709 weights are applied to RGB (or passed through for 1-channel inputs). `channel shuffle` can remap channels similarly but always outputs 4-channel RGBA — prefer this node when you want a single-channel mask downstream.".to_string(),
        }
    }

    pub fn create_inputs() -> Vec<Input> {
        vec![
            Input::new("image".to_string(), Value::Image { data: default_image(), change_id: get_id() }, None, None)
                .with_description("Source image to read a channel from."),
            Input::new("channel".to_string(), Value::Integer(0), Some(InputSettings::Slider { range: (0.0, 4.0), step_by: Some(1.0), clamp_to_range: true }), None)
                .with_description("Channel to extract: 0=R, 1=G, 2=B, 3=A, 4=luminance."),
        ]
    }

    pub fn create_outputs() -> Vec<Output> {
        vec![
            Output::new("output".to_string(), Value::Image { data: default_image(), change_id: get_id() }, None)
                .with_description("1-channel grayscale image of the selected channel."),
        ]
    }

    pub async fn run(inputs: &mut [Input]) -> Result<OperationResponse, OperationError> {
        let start_time = Instant::now();
        let mut input_errors: Vec<(usize, String)> = vec![];

        let image_converted = convert_input(inputs, 0, ValueType::Image, &mut input_errors);
        let channel_converted = convert_input(inputs, 1, ValueType::Integer, &mut input_errors);

        if !input_errors.is_empty() { return Err(OperationError { input_errors, node_error: None }); }

        let Value::Image { data, change_id: _ } = image_converted.unwrap() else { unreachable!() };
        let Value::Integer(channel) = channel_converted.unwrap() else { unreachable!() };

        let channel = channel.clamp(0, 4) as usize;
        let (w, h) = data.dimensions();
        let ch = data.channels() as usize;
        let mut output = FloatImage::new(w, h, 1);

        for y in 0..h {
            for x in 0..w {
                let px = data.get_pixel(x, y);
                let v = if channel == 4 {
                    // Luminance: Rec.709 if the image has RGB, else channel 0.
                    if ch >= 3 {
                        0.2126 * px[0] + 0.7152 * px[1] + 0.0722 * px[2]
                    } else {
                        px[0]
                    }
                } else if channel < ch {
                    px[channel]
                } else {
                    // Out-of-range channel: alpha defaults to 1, others to 0.
                    if channel == 3 { 1.0 } else { 0.0 }
                };
                output.put_pixel(x, y, &[v]);
            }
        }

        Ok(OperationResponse {
            time: Instant::now().duration_since(start_time),
            responses: vec![
                OutputResponse { value: Value::Image { data: Arc::new(output), change_id: get_id() } },
            ],
        })
    }
}

#[cfg(test)]
#[path = "select_tests.rs"]
mod tests;
