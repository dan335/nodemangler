//! Histogram scan (luminance isolation) operation for images.
//!
//! Isolates a narrow band of luminance values from the image, producing a
//! grayscale mask. Uses smoothstep transitions at the edges of the band
//! to avoid hard cutoffs.

use crate::float_image::FloatImage;
use crate::get_id;
use crate::input::{Input, InputSettings};
use crate::node_settings::NodeSettings;
use crate::convert_inputs;
use crate::operations::{OperationResponse, OperationError, OutputResponse, default_image, image_input};
use crate::output::Output;
use crate::value::Value;
use super::common::smoothstep;
use rayon::prelude::*;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use std::time::Instant;

/// Histogram scan operation that isolates a luminance range into a grayscale mask.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpImageAdjustmentHistogramScan{}

impl OpImageAdjustmentHistogramScan {
    /// Returns the node metadata (name and description) for the histogram scan operation.
    pub fn settings() -> NodeSettings {
        NodeSettings {
            name: "histogram scan".to_string(),
            description: "Isolates a luminance range from the image.".to_string(),
            help: "Computes Rec. 709 luminance per pixel, then produces a grayscale mask that is bright where luminance sits inside [position - range, position + range] and dark elsewhere. Soft smoothstep transitions at both edges (edge width equals max(0.01, range * 0.1)) avoid hard stair-step cutoffs.\n\nOutput is always a 4-channel image with the RGB channels set to the mask value and alpha forwarded from the source. Range 0 collapses the band to a razor-thin line and returns a nearly black mask. Useful for building selective adjustments, e.g. pulling midtones into a separate process chain.".to_string(),
        }
    }

    /// Creates the input ports: image, center position of the luminance band, and band width (range).
    pub fn create_inputs() -> Vec<Input> {
        vec![
            image_input("image")
                .with_description("Source image whose luminance is scanned for a narrow band."),
            Input::new("position".to_string(), Value::Decimal(0.5), Some(InputSettings::Slider { range: (0.0, 1.0), step_by: Some(0.01), clamp_to_range: true }), None)
                .with_description("Centre luminance of the band that will become white in the mask."),
            Input::new("range".to_string(), Value::Decimal(0.1), Some(InputSettings::Slider { range: (0.0, 1.0), step_by: Some(0.01), clamp_to_range: true }), None)
                .with_description("Half-width of the selected band around the centre position."),
        ]
    }

    /// Creates the output port: the luminance isolation mask.
    pub fn create_outputs() -> Vec<Output> {
        vec![
            Output::new("output".to_string(), Value::Image { data:default_image(), change_id:get_id()}, None)
                .with_description("Grayscale mask that is bright inside the selected luminance band."),
        ]
    }

    /// Executes the histogram scan. Computes Rec. 709 luminance, then applies smoothstep
    /// transitions at the low and high edges of the selected band. Output is a 4-channel image.
    pub async fn run(inputs: &mut [Input]) -> Result<OperationResponse, OperationError> {
        let start_time = Instant::now();

        convert_inputs! { inputs;
            Image(data) = 0,
            Decimal(position) = 1,
            Decimal(range) = 2,
        }

        // run node — work directly on FloatImage
        let (width, height) = data.dimensions();
        let ch = data.channels() as usize;
        let low = position - range;
        let high = position + range;

        let mut output = FloatImage::new(width, height, 4);

        output
            .par_pixels_mut()
            .zip(data.par_pixels())
            .for_each(|(dst, px)| {
                // Compute luminance from available channels
                let lum = if ch >= 3 {
                    crate::luma::rec709(px[0], px[1], px[2])
                } else {
                    px[0]
                };
                let alpha = if ch == 2 || ch == 4 { px[ch - 1] } else { 1.0 };

                // smoothstep at boundaries for anti-aliasing
                let edge_width = 0.01_f32.max(range * 0.1);
                let low_edge = smoothstep(low - edge_width, low + edge_width, lum);
                let high_edge = 1.0 - smoothstep(high - edge_width, high + edge_width, lum);
                let result = low_edge * high_edge;

                dst.copy_from_slice(&[result, result, result, alpha]);
            });

        Ok(OperationResponse { 
            time: Instant::now().duration_since(start_time),
            responses: vec![
                OutputResponse {value: Value::Image { data:Arc::new(output), change_id:get_id() }},
            ],
        })
    }
}

#[cfg(test)]
#[path = "histogram_scan_tests.rs"]
mod tests;
