//! Channel shuffle (remap) operation.
//!
//! Remaps the RGBA channels of an image by selecting which source channel
//! (0=R, 1=G, 2=B, 3=A) feeds each output channel.

use crate::float_image::FloatImage;
use crate::get_id;
use crate::value::ValueType;
use crate::input::{Input, InputSettings};
use crate::node_settings::NodeSettings;
use crate::operations::{OperationResponse, OperationError, OutputResponse, default_image, convert_input};
use crate::output::Output;
use crate::value::Value;
use rayon::prelude::*;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use std::time::Instant;

/// Minimum pixel count before the remap is parallelized over rows.
const PARALLEL_PIXELS: usize = 1 << 16;

/// Operation that remaps image channels using selectable source indices.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpImageChannelShuffle {}

impl OpImageChannelShuffle {
    pub fn settings() -> NodeSettings {
        NodeSettings {
            name: "channel shuffle".to_string(),
            description: "Remaps image channels using selectable source channels.".to_string(),
            help: "For each output channel (R, G, B, A), picks which source channel (indexed 0=R, 1=G, 2=B, 3=A) to read. The source is first promoted to a virtual RGBA pixel: 1-channel inputs replicate across RGB with alpha 1, 2-channel inputs use luminance+alpha, 3-channel inputs assume alpha 1, and 4-channel inputs pass through.\n\nOutput is always 4-channel RGBA regardless of source channel count. Setting every output to the same source produces a grayscale splat; swapping red and blue gives a BGR->RGB correction; feeding the alpha index into RGB visualises the mask.".to_string(),
        }
    }

    pub fn create_inputs() -> Vec<Input> {
        vec![
            Input::new("image".to_string(), Value::Image { data: default_image(), change_id: get_id() }, None, None)
                .with_description("Source image whose channels are reordered."),
            Input::new("red source".to_string(), Value::Integer(0), Some(InputSettings::Slider { range: (0.0, 3.0), step_by: Some(1.0), clamp_to_range: true }), None)
                .with_description("Which source channel (0=R, 1=G, 2=B, 3=A) feeds the output red."),
            Input::new("green source".to_string(), Value::Integer(1), Some(InputSettings::Slider { range: (0.0, 3.0), step_by: Some(1.0), clamp_to_range: true }), None)
                .with_description("Which source channel (0=R, 1=G, 2=B, 3=A) feeds the output green."),
            Input::new("blue source".to_string(), Value::Integer(2), Some(InputSettings::Slider { range: (0.0, 3.0), step_by: Some(1.0), clamp_to_range: true }), None)
                .with_description("Which source channel (0=R, 1=G, 2=B, 3=A) feeds the output blue."),
            Input::new("alpha source".to_string(), Value::Integer(3), Some(InputSettings::Slider { range: (0.0, 3.0), step_by: Some(1.0), clamp_to_range: true }), None)
                .with_description("Which source channel (0=R, 1=G, 2=B, 3=A) feeds the output alpha."),
        ]
    }

    pub fn create_outputs() -> Vec<Output> {
        vec![Output::new("output".to_string(), Value::Image { data: default_image(), change_id: get_id() }, None)
            .with_description("RGBA image with channels remapped from the chosen source indices.")]
    }

    /// Remaps each pixel's channels based on source indices. Always outputs 4-channel RGBA.
    pub async fn run(inputs: &mut [Input]) -> Result<OperationResponse, OperationError> {
        let start_time = Instant::now();
        let mut input_errors: Vec<(usize, String)> = vec![];

        let image_converted = convert_input(inputs, 0, ValueType::Image, &mut input_errors);
        let red_source_converted = convert_input(inputs, 1, ValueType::Integer, &mut input_errors);
        let green_source_converted = convert_input(inputs, 2, ValueType::Integer, &mut input_errors);
        let blue_source_converted = convert_input(inputs, 3, ValueType::Integer, &mut input_errors);
        let alpha_source_converted = convert_input(inputs, 4, ValueType::Integer, &mut input_errors);

        if !input_errors.is_empty() { return Err(OperationError { input_errors, node_error: None }); }

        let Value::Image{data, change_id:_} = image_converted.unwrap() else { unreachable!() };
        let Value::Integer(red_source) = red_source_converted.unwrap() else { unreachable!() };
        let Value::Integer(green_source) = green_source_converted.unwrap() else { unreachable!() };
        let Value::Integer(blue_source) = blue_source_converted.unwrap() else { unreachable!() };
        let Value::Integer(alpha_source) = alpha_source_converted.unwrap() else { unreachable!() };

        let red_idx = red_source.clamp(0, 3) as usize;
        let green_idx = green_source.clamp(0, 3) as usize;
        let blue_idx = blue_source.clamp(0, 3) as usize;
        let alpha_idx = alpha_source.clamp(0, 3) as usize;

        let (width, height) = data.dimensions();
        let ch = data.channels() as usize;
        let w = width as usize;
        let src = data.as_raw();
        let mut out_data = vec![0.0f32; w * height as usize * 4];

        // Remap one source row into one output row, with the channel-count
        // dispatch hoisted out of the per-pixel loop.
        let process_row = |(dst_row, src_row): (&mut [f32], &[f32])| {
            let write = |dst: &mut [f32], c: [f32; 4]| {
                dst[0] = c[red_idx];
                dst[1] = c[green_idx];
                dst[2] = c[blue_idx];
                dst[3] = c[alpha_idx];
            };
            let pairs = dst_row.chunks_exact_mut(4).zip(src_row.chunks_exact(ch));
            // Promote each pixel to a virtual RGBA, then pick the source indices.
            match ch {
                1 => pairs.for_each(|(d, px)| write(d, [px[0], px[0], px[0], 1.0])),
                2 => pairs.for_each(|(d, px)| write(d, [px[0], px[0], px[0], px[1]])),
                3 => pairs.for_each(|(d, px)| write(d, [px[0], px[1], px[2], 1.0])),
                _ => pairs.for_each(|(d, px)| write(d, [px[0], px[1], px[2], px[3]])),
            }
        };

        if w > 0 {
            if w * height as usize >= PARALLEL_PIXELS {
                out_data.par_chunks_exact_mut(w * 4).zip(src.par_chunks_exact(w * ch)).for_each(process_row);
            } else {
                out_data.chunks_exact_mut(w * 4).zip(src.chunks_exact(w * ch)).for_each(process_row);
            }
        }

        let output = FloatImage::from_raw(width, height, 4, out_data).unwrap();

        Ok(OperationResponse { 
            time: Instant::now().duration_since(start_time),
            responses: vec![OutputResponse { value: Value::Image { data: Arc::new(output), change_id: get_id() } }],
        })
    }
}

#[cfg(test)]
#[path = "shuffle_tests.rs"]
mod tests;
