//! ASCII-art rendering of an image.
//!
//! Samples the image on a character grid and maps each cell's luminance to a
//! glyph from a light→dark ramp, producing a multi-line text picture.

use crate::get_id;
use crate::input::{Input, InputSettings};
use crate::node_settings::NodeSettings;
use crate::operations::{OperationResponse, OperationError, OutputResponse, default_image, convert_input};
use crate::output::Output;
use crate::value::{Value, ValueType};
use serde::{Deserialize, Serialize};
use std::time::Instant;

use crate::operations::numbers::image::pixel_luma;

/// Operation that renders an image as ASCII art.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpTextImageAsciiArt {}

impl OpTextImageAsciiArt {
    /// Returns the node metadata (name, description, help).
    pub fn settings() -> NodeSettings {
        NodeSettings {
            name: "ascii art".to_string(),
            description: "Renders an image as multi-line ASCII art from a glyph ramp.".to_string(),
            help: "Samples the image on a `columns`-wide character grid and maps each cell's Rec. 601 luminance to a glyph from a light→dark ramp (` .:-=+*#%@`), so dark pixels become dense glyphs. Self-contained: no fonts or external assets.\n\nThe row count is derived from the image's aspect ratio and then halved to correct for character cells being taller than they are wide, so the picture keeps its proportions in a monospaced view.".to_string(),
        }
    }

    /// Creates the input ports: the image and the column count.
    pub fn create_inputs() -> Vec<Input> {
        vec![
            Input::new("image".to_string(), Value::Image { data: default_image(), change_id: get_id() }, None, None)
                .with_description("Image to render as ASCII art."),
            Input::new("columns".to_string(), Value::Integer(80), Some(InputSettings::DragValue { clamp: Some((8.0, 400.0)), speed: None }), None)
                .with_description("Number of character columns (8..400)."),
        ]
    }

    /// Creates the output port: the ASCII-art text.
    pub fn create_outputs() -> Vec<Output> {
        vec![
            Output::new("output".to_string(), Value::Text(String::new()), None)
                .with_description("Multi-line ASCII-art rendering of the image."),
        ]
    }

    /// Executes the ASCII-art rendering.
    pub async fn run(inputs: &mut [Input]) -> Result<OperationResponse, OperationError> {
        let start_time = Instant::now();
        let mut input_errors: Vec<(usize, String)> = vec![];

        let image_converted = convert_input(inputs, 0, ValueType::Image, &mut input_errors);
        let columns_converted = convert_input(inputs, 1, ValueType::Integer, &mut input_errors);

        if !input_errors.is_empty() { return Err(OperationError { input_errors, node_error: None }); }

        let Value::Image { data, change_id: _ } = image_converted.unwrap() else { unreachable!() };
        let Value::Integer(columns) = columns_converted.unwrap() else { unreachable!() };

        let (w, h) = data.dimensions();
        let ch = data.channels() as usize;
        let cols = columns.clamp(8, 400) as u32;
        // Halve the row count so tall character cells keep the image's proportions.
        let cell = (w as f32 / cols as f32).max(1.0);
        let rows = (((h as f32 / cell) / 2.0).round() as u32).max(1);

        // Light → dark ramp; a light pixel maps to the first glyph.
        let ramp = [' ', '.', ':', '-', '=', '+', '*', '#', '%', '@'];

        let mut out = String::with_capacity((rows * (cols + 1)) as usize);
        for ry in 0..rows {
            for cx in 0..cols {
                let sx = (cx as f32 + 0.5) / cols as f32 * w.saturating_sub(1) as f32;
                let sy = (ry as f32 + 0.5) / rows as f32 * h.saturating_sub(1) as f32;
                let mut buf = [0.0f32; 4];
                data.bilinear_sample(sx, sy, &mut buf[..ch]);
                let lum = pixel_luma(&buf[..ch]).clamp(0.0, 1.0);
                let idx = (((1.0 - lum) * (ramp.len() - 1) as f32).round() as usize).min(ramp.len() - 1);
                out.push(ramp[idx]);
            }
            out.push('\n');
        }

        Ok(OperationResponse {
            time: Instant::now().duration_since(start_time),
            responses: vec![OutputResponse { value: Value::Text(out) }],
        })
    }
}

#[cfg(test)]
#[path = "ascii_art_tests.rs"]
mod tests;
