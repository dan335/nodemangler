//! Grayscale conversion operation for images.
//!
//! Converts an image to a 1-channel grayscale FloatImage using Rec. 709
//! luminance weights. For 1-channel input, returns as-is.

use crate::float_image::FloatImage;
use crate::get_id;
use crate::input::Input;
use crate::node_settings::NodeSettings;
use crate::operations::{OperationResponse, OperationError, OutputResponse, default_image, convert_input};
use crate::output::Output;
use crate::value::{Value, ValueType};
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
            Input::new("image".to_string(), Value::Image { data: default_image(), change_id:get_id() }, None, None)
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
    /// For 1-channel input, returns as-is. For 3/4-channel, computes Rec. 709 luminance.
    pub async fn run(inputs: &mut [Input]) -> Result<OperationResponse, OperationError> {
        let start_time = Instant::now();
        let mut input_errors: Vec<(usize, String)> = vec![];

        // convert inputs
        let image_converted = convert_input(inputs, 0, ValueType::Image, &mut input_errors);

        // return if error
        if !input_errors.is_empty() { return Err(OperationError { input_errors, node_error: None }); }

        // get values
        let Value::Image{data, change_id:_} = image_converted.unwrap() else { unreachable!() };

        // run node — convert to 1-channel grayscale
        let ch = data.channels() as usize;
        let result = if ch == 1 {
            // Already grayscale, return as-is
            (*data).clone()
        } else {
            // Compute luminance for each pixel: lum = 0.299*r + 0.587*g + 0.114*b
            let (w, h) = data.dimensions();
            let mut out = FloatImage::new(w, h, 1);
            for y in 0..h {
                for x in 0..w {
                    let px = data.get_pixel(x, y);
                    let r = px[0];
                    let g = if ch >= 2 { px[1] } else { r };
                    let b = if ch >= 3 { px[2] } else { r };
                    let lum = 0.299 * r + 0.587 * g + 0.114 * b;
                    out.put_pixel(x, y, &[lum]);
                }
            }
            out
        };

        Ok(OperationResponse { 
            time: Instant::now().duration_since(start_time),
            responses: vec![
                OutputResponse {value: Value::Image { data:Arc::new(result), change_id:get_id() }},
            ],
        })
    }
}

#[cfg(test)]
#[path = "grayscale_tests.rs"]
mod tests;
