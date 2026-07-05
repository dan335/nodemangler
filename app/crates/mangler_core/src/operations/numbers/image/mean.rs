//! Mean (average) image statistic.
//!
//! Reduces an image to the average luminance and the average of each RGBA
//! channel across all pixels. Grayscale inputs replicate their single channel
//! across RGB; a missing alpha counts as fully opaque.

use crate::get_id;
use crate::input::Input;
use crate::node_settings::NodeSettings;
use crate::operations::{OperationResponse, OperationError, OutputResponse, default_image, convert_input};
use crate::output::Output;
use crate::value::{Value, ValueType};
use serde::{Deserialize, Serialize};
use std::time::Instant;

use super::{pixel_luma, pixel_rgba};

/// Operation that computes the mean luminance and per-channel means of an image.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpNumberImageMean {}

impl OpNumberImageMean {
    /// Returns the node metadata (name, description, help).
    pub fn settings() -> NodeSettings {
        NodeSettings {
            name: "mean".to_string(),
            description: "Averages every pixel to a mean luminance and per-channel means.".to_string(),
            help: "Walks every pixel and averages it, emitting the mean Rec. 601 luminance (0.299 R + 0.587 G + 0.114 B) plus the mean red, green, blue, and alpha. Grayscale (1–2 channel) inputs replicate their value across RGB; images without an alpha channel report alpha as 1.\n\nUse the luminance output as an overall brightness scalar, or the per-channel means to detect a color cast. This is a plain arithmetic mean over all pixels — for a robust center use the median node instead.".to_string(),
        }
    }

    /// Creates the input port: a single image to average.
    pub fn create_inputs() -> Vec<Input> {
        vec![
            Input::new("image".to_string(), Value::Image { data: default_image(), change_id: get_id() }, None, None)
                .with_description("Image whose pixels are averaged."),
        ]
    }

    /// Creates the output ports: luminance and per-channel means.
    pub fn create_outputs() -> Vec<Output> {
        vec![
            Output::new("luminance".to_string(), Value::Decimal(0.0), None)
                .with_description("Mean Rec. 601 luminance across all pixels."),
            Output::new("red".to_string(), Value::Decimal(0.0), None)
                .with_description("Mean red channel."),
            Output::new("green".to_string(), Value::Decimal(0.0), None)
                .with_description("Mean green channel."),
            Output::new("blue".to_string(), Value::Decimal(0.0), None)
                .with_description("Mean blue channel."),
            Output::new("alpha".to_string(), Value::Decimal(1.0), None)
                .with_description("Mean alpha (1.0 when the image has no alpha channel)."),
        ]
    }

    /// Executes the mean computation.
    pub async fn run(inputs: &mut [Input]) -> Result<OperationResponse, OperationError> {
        let start_time = Instant::now();
        let mut input_errors: Vec<(usize, String)> = vec![];

        let image_converted = convert_input(inputs, 0, ValueType::Image, &mut input_errors);

        if !input_errors.is_empty() { return Err(OperationError { input_errors, node_error: None }); }

        let Value::Image { data, change_id: _ } = image_converted.unwrap() else { unreachable!() };

        let (mut sl, mut sr, mut sg, mut sb, mut sa) = (0.0f64, 0.0f64, 0.0f64, 0.0f64, 0.0f64);
        let mut count = 0u64;
        for px in data.pixels() {
            let (r, g, b, a) = pixel_rgba(px);
            sl += pixel_luma(px) as f64;
            sr += r as f64;
            sg += g as f64;
            sb += b as f64;
            sa += a as f64;
            count += 1;
        }

        let (lum, red, green, blue, alpha) = if count == 0 {
            (0.0, 0.0, 0.0, 0.0, 1.0)
        } else {
            let n = count as f64;
            ((sl / n) as f32, (sr / n) as f32, (sg / n) as f32, (sb / n) as f32, (sa / n) as f32)
        };

        Ok(OperationResponse {
            time: Instant::now().duration_since(start_time),
            responses: vec![
                OutputResponse { value: Value::Decimal(lum) },
                OutputResponse { value: Value::Decimal(red) },
                OutputResponse { value: Value::Decimal(green) },
                OutputResponse { value: Value::Decimal(blue) },
                OutputResponse { value: Value::Decimal(alpha) },
            ],
        })
    }
}

#[cfg(test)]
#[path = "mean_tests.rs"]
mod tests;
