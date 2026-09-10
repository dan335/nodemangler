//! Histogram select — extracts a luminance band as a soft-edged mask.
//!
//! Given a target position and range on the luminance axis, the output is
//! brightest where the input's luminance is at `position` and fades to black
//! at the edges of the band. `contrast` controls how sharp that fade is: 0
//! gives a fully soft transition, 1 gives a hard rectangular mask.

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

/// Histogram select: isolate a luminance band as a mask.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpImageAdjustmentHistogramSelect {}

impl OpImageAdjustmentHistogramSelect {
    pub fn settings() -> NodeSettings {
        NodeSettings {
            name: "histogram select".to_string(),
            description: "Outputs a soft-edged mask where input luminance falls within a chosen band.".to_string(),
            help: "Computes Rec. 709 luminance per pixel, then emits 1 when the pixel's distance from the band centre is within a fully-opaque inner region and 0 beyond the outer edge. Between those two radii the output is a Hermite smoothstep fade.\n\nThe contrast input grows the inner region from 0 (fully soft triangular falloff) to 1 (rectangular, hard-edged band). Range sets the total band width; half of it is the outer edge. Output is a single-channel mask, independent of the source's channel count. Handy for isolating highlights/shadows/midtones for downstream masking or blending.".to_string(),
        }
    }

    pub fn create_inputs() -> Vec<Input> {
        vec![
            image_input("image")
                .with_description("Source image whose luminance is selected into a mask."),
            Input::new("position".to_string(), Value::Decimal(0.5), Some(InputSettings::Slider { range: (0.0, 1.0), step_by: Some(0.01), clamp_to_range: true }), None)
                .with_description("Target luminance at the centre of the selection band."),
            Input::new("range".to_string(), Value::Decimal(0.2), Some(InputSettings::Slider { range: (0.0, 1.0), step_by: Some(0.01), clamp_to_range: true }), None)
                .with_description("Total width of the luminance band around the position."),
            Input::new("contrast".to_string(), Value::Decimal(0.0), Some(InputSettings::Slider { range: (0.0, 1.0), step_by: Some(0.01), clamp_to_range: true }), None)
                .with_description("Hardness of the band edges; 0 is fully soft, 1 is a rectangular cut."),
        ]
    }

    pub fn create_outputs() -> Vec<Output> {
        vec![
            Output::new("output".to_string(), Value::Image { data: default_image(), change_id: get_id() }, None)
                .with_description("Single-channel mask that is bright inside the selected luminance band."),
        ]
    }

    pub async fn run(inputs: &mut [Input]) -> Result<OperationResponse, OperationError> {
        let start_time = Instant::now();

        convert_inputs! { inputs;
            Image(data) = 0,
            Decimal(position) = 1,
            Decimal(range) = 2,
            Decimal(contrast) = 3,
        }

        let half_range = (range * 0.5).max(1e-6);
        let contrast = contrast.clamp(0.0, 1.0);
        // `contrast` widens the fully-opaque interior of the band:
        // - 0 → soft gradient across the full half-range (triangular mask)
        // - 1 → rectangular mask (entire band is 1.0, nothing in between)
        let soft_edge = half_range * contrast;
        let hard_edge = half_range;

        let (width, height) = data.dimensions();
        let ch = data.channels() as usize;
        let color_ch = if ch == 2 || ch == 4 { ch - 1 } else { ch };

        let mut output = FloatImage::new(width, height, 1);
        output
            .par_pixels_mut()
            .zip(data.par_pixels())
            .for_each(|(dst, p)| {
                let lum = if color_ch >= 3 {
                    crate::luma::rec709(p[0], p[1], p[2])
                } else {
                    p[0]
                };
                let d = (lum - position).abs();
                // Mask is 1 inside the fully-opaque inner radius (soft_edge),
                // 0 beyond the outer radius (hard_edge), and a Hermite smoothstep
                // fade between them. The shared `smoothstep` clamps the input and
                // degenerates to a hard step when the edges coincide (contrast 1),
                // giving the rectangular band for free.
                dst[0] = 1.0 - smoothstep(soft_edge, hard_edge, d);
            });

        Ok(OperationResponse {
            time: Instant::now().duration_since(start_time),
            responses: vec![
                OutputResponse { value: Value::Image { data: Arc::new(output), change_id: get_id() } },
            ],
        })
    }
}

#[cfg(test)]
#[path = "histogram_select_tests.rs"]
mod tests;
