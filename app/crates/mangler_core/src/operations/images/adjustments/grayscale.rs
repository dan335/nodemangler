//! Grayscale conversion operation for images.
//!
//! Converts an image to a 1-channel grayscale FloatImage using Rec. 601
//! luminance weights (0.299 R + 0.587 G + 0.114 B). For 1-channel input,
//! returns as-is.

use crate::float_image::FloatImage;
use crate::get_id;
use crate::input::Input;
use crate::node_settings::NodeSettings;
use crate::convert_inputs;
use crate::operations::{OperationResponse, OperationError, OutputResponse, default_image, image_input};
use crate::output::Output;
use crate::value::Value;
use rayon::prelude::*;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use std::time::Instant;

/// Grayscale conversion operation that removes color information from an image.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpImageAdjustmentGrayscale {}

impl OpImageAdjustmentGrayscale {
    /// Returns the node metadata (name and description) for the grayscale operation.
    pub fn settings() -> NodeSettings {
        NodeSettings {
            name: "grayscale".to_string(),
            description: "Converts an image to grayscale using luminance weighting.".to_string(),
            help: "Collapses colour to a single luminance channel using the Rec. 601 weights 0.299 R + 0.587 G + 0.114 B, producing a 1-channel FloatImage. Inputs that are already 1-channel are returned as-is without copying per-pixel data.\n\nAlpha is dropped because the output has no alpha channel; for mask-style workflows where alpha matters, extract the luminance via `channels split` instead. Downstream nodes that need an RGB image will see the grayscale replicated across channels on demand.".to_string(),
        }
    }

    /// Creates the input port: a single image to convert.
    pub fn create_inputs() -> Vec<Input> {
        vec![
            image_input("image")
                .with_description("Source colour image to collapse into luminance."),
        ]
    }

    /// Creates the output port: the grayscale-converted image (1 channel).
    pub fn create_outputs() -> Vec<Output> {
        vec![
            Output::new("output".to_string(), Value::Image { data: default_image(), change_id:get_id() }, None)
                .with_description("Single-channel image holding per-pixel luminance."),
        ]
    }

    /// Executes the grayscale conversion on the input image.
    /// For 1-channel input, returns as-is. For 3/4-channel, computes Rec. 601 luminance.
    pub async fn run(inputs: &mut [Input]) -> Result<OperationResponse, OperationError> {
        let start_time = Instant::now();

        convert_inputs! { inputs;
            Image(data) = 0,
        }

        // run node — convert to 1-channel grayscale
        let ch = data.channels() as usize;
        if ch == 1 {
            // Already grayscale: pass the input through untouched.
            return Ok(OperationResponse {
                time: Instant::now().duration_since(start_time),
                responses: vec![
                    OutputResponse { value: Value::Image { data, change_id: get_id() } },
                ],
            });
        }

        // Compute luminance for each pixel: lum = 0.299*r + 0.587*g + 0.114*b
        let (w, h) = data.dimensions();
        let mut out = FloatImage::new(w, h, 1);
        out.par_pixels_mut()
            .zip(data.par_pixels())
            .for_each(|(dst, px)| {
                let r = px[0];
                let g = if ch >= 2 { px[1] } else { r };
                let b = if ch >= 3 { px[2] } else { r };
                dst[0] = crate::luma::rec601(r, g, b);
            });

        Ok(OperationResponse {
            time: Instant::now().duration_since(start_time),
            responses: vec![
                OutputResponse {value: Value::Image { data:Arc::new(out), change_id:get_id() }},
            ],
        })
    }
}

#[cfg(test)]
#[path = "grayscale_tests.rs"]
mod tests;
