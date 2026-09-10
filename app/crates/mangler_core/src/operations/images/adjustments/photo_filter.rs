//! Photoshop-style Photo Filter: a colour tint scaled by density, with an
//! optional preserve-luminosity rescale.
//!
//! Multiplies every RGB pixel by a filter colour that has been blended toward
//! white by `1 - density`, so density 0 leaves the image untouched and density
//! 1 applies the pure filter colour as a multiplicative gain. When preserve
//! luminosity is on, the tinted pixel is rescaled so its Rec.709 luma matches
//! the original — changing colour without changing perceived brightness, which
//! is what Photoshop's Photo Filter does by default.

use crate::color::Color;
use crate::get_id;
use crate::input::{Input, InputSettings};
use crate::node_settings::NodeSettings;
use crate::convert_inputs;
use crate::operations::{OperationResponse, OperationError, OutputResponse, image_input, image_output};
use crate::output::Output;
use crate::value::Value;
use rayon::prelude::*;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use std::time::Instant;

/// Photo Filter adjustment: a density-scaled multiplicative colour tint.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpImageAdjustmentPhotoFilter {}

impl OpImageAdjustmentPhotoFilter {
    /// Returns the node metadata (name, description, help) for the photo filter.
    pub fn settings() -> NodeSettings {
        NodeSettings {
            name: "photo filter".to_string(),
            description: "Applies a colour tint scaled by density, optionally preserving luminosity.".to_string(),
            help: "Emulates Photoshop's Photo Filter adjustment. Each RGB channel is multiplied by a per-channel factor derived from the filter colour and the density: factor = 1 + density * (colour_channel - 1), which is a linear blend from 1 (no change) at density 0 toward the filter colour at density 1. The default colour is the classic 'Warming Filter (85)' orange with density 0.25.\n\nDensity 0 leaves the image exactly unchanged. Higher densities push the multiplicative tint further, warming or cooling the image depending on the colour.\n\nWith 'preserve luminosity' enabled, the tinted pixel is rescaled so its Rec.709 luma (0.2126 R + 0.7152 G + 0.0722 B) matches the original pixel's luma. This keeps overall brightness constant and changes only colour — matching Photoshop's default behaviour. Pixels whose tinted luma is effectively zero are left as-is to avoid dividing by zero.\n\nOutputs are not clamped, so a bright filter can push channels past 1.0 for downstream nodes to handle. Grayscale inputs (fewer than 3 channels) have no chroma to tint and pass through unchanged; alpha is always preserved.".to_string(),
        }
    }

    /// Creates input ports: image, filter colour, density, preserve-luminosity.
    pub fn create_inputs() -> Vec<Input> {
        vec![
            image_input("image")
                .with_description("Source colour image to tint."),
            // Default = Photoshop 'Warming Filter (85)' orange.
            Input::new("color".to_string(), Value::Color(Color { r: 0.925, g: 0.541, b: 0.0, a: 1.0 }), None, None)
                .with_description("Filter colour multiplied onto the image, scaled by density."),
            Input::new("density".to_string(), Value::Decimal(0.25), Some(InputSettings::Slider { range: (0.0, 1.0), step_by: Some(0.01), clamp_to_range: true }), None)
                .with_description("Strength of the tint; 0 = no change, 1 = full filter colour."),
            Input::new("preserve luminosity".to_string(), Value::Bool(true), None, None)
                .with_description("Rescale so the result's Rec.709 luma matches the original (colour-only shift)."),
        ]
    }

    /// Creates the output port: the tinted image.
    pub fn create_outputs() -> Vec<Output> {
        vec![
            image_output("output")
                .with_description("Image with the density-scaled colour filter applied."),
        ]
    }

    /// Executes the photo filter: multiply RGB by the density-scaled colour,
    /// then optionally rescale each pixel to preserve its luminosity.
    pub async fn run(inputs: &mut [Input]) -> Result<OperationResponse, OperationError> {
        let start_time = Instant::now();

        convert_inputs! { inputs;
            Image(data) = 0,
            Color(color) = 1,
            Decimal(density) = 2,
            Bool(preserve_luminosity) = 3,
        }

        let ch = data.channels() as usize;
        if ch < 3 {
            // Grayscale: no chroma to tint, pass through unchanged.
            return Ok(OperationResponse {
                time: Instant::now().duration_since(start_time),
                responses: vec![OutputResponse { value: Value::Image { data, change_id: get_id() } }],
            });
        }

        // Per-channel multiplicative factors: lerp(1, colour, density).
        let fr = 1.0 + density * (color.r - 1.0);
        let fg = 1.0 + density * (color.g - 1.0);
        let fb = 1.0 + density * (color.b - 1.0);

        let mut result = (*data).clone();
        result.par_pixels_mut().for_each(|pixel| {
            let r = pixel[0];
            let g = pixel[1];
            let b = pixel[2];

            // Apply the tint.
            let mut o = [r * fr, g * fg, b * fb];

            if preserve_luminosity {
                // Rescale so the tinted luma matches the original luma.
                let l0 = crate::luma::rec709(r, g, b);
                let l1 = crate::luma::rec709(o[0], o[1], o[2]);
                if l1 > 1e-6 {
                    let k = l0 / l1;
                    o[0] *= k;
                    o[1] *= k;
                    o[2] *= k;
                }
            }

            pixel[0] = o[0];
            pixel[1] = o[1];
            pixel[2] = o[2];
            // Alpha (pixel[3] on 4-channel) is left untouched.
        });

        Ok(OperationResponse {
            time: Instant::now().duration_since(start_time),
            responses: vec![OutputResponse { value: Value::Image { data: Arc::new(result), change_id: get_id() } }],
        })
    }
}

#[cfg(test)]
#[path = "photo_filter_tests.rs"]
mod tests;
