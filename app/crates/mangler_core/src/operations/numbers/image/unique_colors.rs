//! Count of distinct colors after quantization.
//!
//! Quantizes each pixel's RGB to a grid of `levels` steps per channel and
//! counts how many distinct buckets are occupied. Quantizing first keeps the
//! count meaningful for photographic images, where tiny float differences
//! would otherwise make almost every pixel "unique".

use crate::get_id;
use crate::input::{Input, InputSettings};
use crate::node_settings::NodeSettings;
use crate::operations::{OperationResponse, OperationError, OutputResponse, default_image, convert_input};
use crate::output::Output;
use crate::value::{Value, ValueType};
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::time::Instant;

/// Operation that counts distinct quantized colors in an image.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpNumberImageUniqueColors {}

impl OpNumberImageUniqueColors {
    /// Returns the node metadata (name, description, help).
    pub fn settings() -> NodeSettings {
        NodeSettings {
            name: "unique colors".to_string(),
            description: "Counts distinct colors after quantizing each channel.".to_string(),
            help: "Snaps every pixel's red, green, and blue to a grid of `levels` steps per channel, then counts how many distinct color buckets are used. Alpha is ignored.\n\nQuantizing first is what makes the count useful: raw floats differ by rounding on almost every pixel, so without it a photo would report nearly one color per pixel. Fewer levels give a coarser, more forgiving count; more levels approach an exact count. The maximum is levels^3.".to_string(),
        }
    }

    /// Creates the input ports: the image and the per-channel quantization level.
    pub fn create_inputs() -> Vec<Input> {
        vec![
            Input::new("image".to_string(), Value::Image { data: default_image(), change_id: get_id() }, None, None)
                .with_description("Image whose distinct colors are counted."),
            Input::new("levels".to_string(), Value::Integer(32), Some(InputSettings::DragValue { clamp: Some((2.0, 256.0)), speed: None }), None)
                .with_description("Quantization steps per channel (2..256). Fewer = coarser count."),
        ]
    }

    /// Creates the output port: the distinct-color count.
    pub fn create_outputs() -> Vec<Output> {
        vec![
            Output::new("count".to_string(), Value::Integer(0), None)
                .with_description("Number of distinct quantized colors."),
        ]
    }

    /// Executes the unique-color count.
    pub async fn run(inputs: &mut [Input]) -> Result<OperationResponse, OperationError> {
        let start_time = Instant::now();
        let mut input_errors: Vec<(usize, String)> = vec![];

        let image_converted = convert_input(inputs, 0, ValueType::Image, &mut input_errors);
        let levels_converted = convert_input(inputs, 1, ValueType::Integer, &mut input_errors);

        if !input_errors.is_empty() { return Err(OperationError { input_errors, node_error: None }); }

        let Value::Image { data, change_id: _ } = image_converted.unwrap() else { unreachable!() };
        let Value::Integer(levels) = levels_converted.unwrap() else { unreachable!() };

        let levels = levels.clamp(2, 256) as u32;
        let scale = (levels - 1) as f32;
        let q = |v: f32| (v.clamp(0.0, 1.0) * scale).round() as u32;

        let mut set: HashSet<u32> = HashSet::new();
        for px in data.pixels() {
            let (r, g, b, _) = super::pixel_rgba(px);
            let key = q(r) * levels * levels + q(g) * levels + q(b);
            set.insert(key);
        }

        Ok(OperationResponse {
            time: Instant::now().duration_since(start_time),
            responses: vec![OutputResponse { value: Value::Integer(set.len() as i32) }],
        })
    }
}

#[cfg(test)]
#[path = "unique_colors_tests.rs"]
mod tests;
