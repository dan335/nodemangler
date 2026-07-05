//! Dominant-color palette as hex codes.
//!
//! Quantizes every pixel's RGB to a grid of `levels` steps per channel, tallies
//! the buckets, and emits the `count` most common colors as newline-separated
//! `#RRGGBB` codes.

use crate::get_id;
use crate::input::{Input, InputSettings};
use crate::node_settings::NodeSettings;
use crate::operations::{OperationResponse, OperationError, OutputResponse, default_image, convert_input};
use crate::output::Output;
use crate::value::{Value, ValueType};
use serde::{Deserialize, Serialize};
use std::time::Instant;

use std::collections::HashMap;
use crate::operations::numbers::image::pixel_rgba;

/// Operation that extracts a dominant-color palette as hex codes.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpTextImagePaletteHex {}

impl OpTextImagePaletteHex {
    /// Returns the node metadata (name, description, help).
    pub fn settings() -> NodeSettings {
        NodeSettings {
            name: "palette hex".to_string(),
            description: "Extracts the most common colors as newline-separated hex codes.".to_string(),
            help: "Snaps every pixel's red, green, and blue to a grid of `levels` steps per channel, tallies how often each bucket occurs, and outputs the `count` most common colors as `#RRGGBB` codes, one per line (most frequent first). Alpha is ignored.\n\nFewer `levels` merge similar shades into broader swatches; more `levels` keep finer distinctions. Ties are broken by color key for deterministic output.".to_string(),
        }
    }

    /// Creates the input ports: the image, the palette size, and the quantization level.
    pub fn create_inputs() -> Vec<Input> {
        vec![
            Input::new("image".to_string(), Value::Image { data: default_image(), change_id: get_id() }, None, None)
                .with_description("Image to extract the palette from."),
            Input::new("count".to_string(), Value::Integer(5), Some(InputSettings::DragValue { clamp: Some((1.0, 32.0)), speed: None }), None)
                .with_description("Number of colors to output (1..32)."),
            Input::new("levels".to_string(), Value::Integer(6), Some(InputSettings::DragValue { clamp: Some((2.0, 32.0)), speed: None }), None)
                .with_description("Quantization steps per channel (2..32). Fewer = broader swatches."),
        ]
    }

    /// Creates the output port: the newline-separated hex codes.
    pub fn create_outputs() -> Vec<Output> {
        vec![
            Output::new("output".to_string(), Value::Text(String::new()), None)
                .with_description("Newline-separated `#RRGGBB` codes, most common first."),
        ]
    }

    /// Executes the palette extraction.
    pub async fn run(inputs: &mut [Input]) -> Result<OperationResponse, OperationError> {
        let start_time = Instant::now();
        let mut input_errors: Vec<(usize, String)> = vec![];

        let image_converted = convert_input(inputs, 0, ValueType::Image, &mut input_errors);
        let count_converted = convert_input(inputs, 1, ValueType::Integer, &mut input_errors);
        let levels_converted = convert_input(inputs, 2, ValueType::Integer, &mut input_errors);

        if !input_errors.is_empty() { return Err(OperationError { input_errors, node_error: None }); }

        let Value::Image { data, change_id: _ } = image_converted.unwrap() else { unreachable!() };
        let Value::Integer(count) = count_converted.unwrap() else { unreachable!() };
        let Value::Integer(levels) = levels_converted.unwrap() else { unreachable!() };

        let levels = levels.clamp(2, 32) as u32;
        let count = count.clamp(1, 32) as usize;

        let mut hist: HashMap<u32, u64> = HashMap::new();
        for px in data.pixels() {
            let (r, g, b, _) = pixel_rgba(px);
            let q = |v: f32| (v.clamp(0.0, 1.0) * ((levels - 1) as f32)).round() as u32;
            let key = q(r) * levels * levels + q(g) * levels + q(b);
            *hist.entry(key).or_insert(0) += 1;
        }

        // Frequency descending, key ascending for deterministic tie-breaks.
        let mut items: Vec<(u32, u64)> = hist.into_iter().collect();
        items.sort_by(|a, b| b.1.cmp(&a.1).then(a.0.cmp(&b.0)));

        let to255 = |q: u32| ((q as f32 / ((levels - 1) as f32)) * 255.0).round() as u32;
        let lines: Vec<String> = items.iter().take(count).map(|(key, _)| {
            let qr = key / (levels * levels);
            let qg = (key / levels) % levels;
            let qb = key % levels;
            format!("#{:02X}{:02X}{:02X}", to255(qr), to255(qg), to255(qb))
        }).collect();

        Ok(OperationResponse {
            time: Instant::now().duration_since(start_time),
            responses: vec![OutputResponse { value: Value::Text(lines.join("\n")) }],
        })
    }
}

#[cfg(test)]
#[path = "palette_hex_tests.rs"]
mod tests;
