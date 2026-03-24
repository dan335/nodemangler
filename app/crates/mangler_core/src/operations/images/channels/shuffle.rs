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
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use std::time::Instant;

/// Operation that remaps image channels using selectable source indices.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpImageChannelShuffle {}

impl OpImageChannelShuffle {
    pub fn settings() -> NodeSettings {
        NodeSettings { name: "channel shuffle".to_string(), description: "Remaps image channels using selectable source channels.".to_string() }
    }

    pub fn create_inputs() -> Vec<Input> {
        vec![
            Input::new("image".to_string(), Value::Image { data: default_image(), change_id: get_id() }, None, None),
            Input::new("red source".to_string(), Value::Integer(0), Some(InputSettings::Slider { range: (0.0, 3.0), step_by: Some(1.0), clamp_to_range: true }), None),
            Input::new("green source".to_string(), Value::Integer(1), Some(InputSettings::Slider { range: (0.0, 3.0), step_by: Some(1.0), clamp_to_range: true }), None),
            Input::new("blue source".to_string(), Value::Integer(2), Some(InputSettings::Slider { range: (0.0, 3.0), step_by: Some(1.0), clamp_to_range: true }), None),
            Input::new("alpha source".to_string(), Value::Integer(3), Some(InputSettings::Slider { range: (0.0, 3.0), step_by: Some(1.0), clamp_to_range: true }), None),
        ]
    }

    pub fn create_outputs() -> Vec<Output> {
        vec![Output::new("output".to_string(), Value::Image { data: default_image(), change_id: get_id() }, None)]
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
        let mut output = FloatImage::new(width, height, 4);

        // Helper: get a virtual 4-channel RGBA from any channel count
        let get_rgba = |px: &[f32]| -> [f32; 4] {
            match ch {
                1 => [px[0], px[0], px[0], 1.0],
                2 => [px[0], px[0], px[0], px[1]],
                3 => [px[0], px[1], px[2], 1.0],
                _ => [px[0], px[1], px[2], px[3]],
            }
        };

        for y in 0..height {
            for x in 0..width {
                let px = data.get_pixel(x, y);
                let channels = get_rgba(px);
                output.put_pixel(x, y, &[channels[red_idx], channels[green_idx], channels[blue_idx], channels[alpha_idx]]);
            }
        }

        Ok(OperationResponse { ai_cost_usd: None,
            time: Instant::now().duration_since(start_time),
            responses: vec![OutputResponse { value: Value::Image { data: Arc::new(output), change_id: get_id() } }],
        })
    }
}

#[cfg(test)]
#[path = "shuffle_tests.rs"]
mod tests;
