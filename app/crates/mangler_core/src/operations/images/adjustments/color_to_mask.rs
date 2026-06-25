//! Color selection to mask operation.
//!
//! Compares every pixel against a target color and emits a single-channel
//! mask where pixels similar to the target are 1.0 and dissimilar pixels are
//! 0.0. A tolerance controls how far from the target still counts as a hit,
//! and a softness adds a smooth fade around that tolerance band so the mask
//! does not aliased-cut along the threshold boundary.

use crate::color::Color;
use crate::float_image::FloatImage;
use crate::get_id;
use crate::input::{Input, InputSettings};
use crate::node_settings::NodeSettings;
use crate::operations::{OperationResponse, OperationError, OutputResponse, default_image, convert_input};
use crate::output::Output;
use crate::value::{Value, ValueType};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use std::time::Instant;

/// Produces a mask marking pixels whose color is close to a target color.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpImageAdjustmentColorToMask {}

impl OpImageAdjustmentColorToMask {
    pub fn settings() -> NodeSettings {
        NodeSettings {
            name: "color to mask".to_string(),
            description: "Outputs a single-channel mask isolating pixels close to a target color.".to_string(),
            help: "For each RGB pixel computes the Euclidean distance to the target color in the sRGB cube and normalises by sqrt(3) so the distance sits in [0, 1]. Pixels whose normalised distance is below `tolerance` are fully selected (mask = 1); pixels beyond `tolerance + softness` are fully rejected (mask = 0); the interval between is a smoothstep so the mask fades cleanly instead of clipping.\n\nMask is single-channel regardless of input channel count. Alpha of the input image is ignored — only RGB contributes to the distance. 1 or 2 channel inputs (grayscale) treat the target color's luminance as the match target.".to_string(),
        }
    }

    pub fn create_inputs() -> Vec<Input> {
        vec![
            Input::new("image".to_string(), Value::Image { data: default_image(), change_id: get_id() }, None, None)
                .with_description("Source image to scan for the target color."),
            Input::new("color".to_string(), Value::Color(Color { r: 1.0, g: 0.0, b: 0.0, a: 1.0 }), None, None)
                .with_description("Target color to select; pixels close to this produce mask = 1."),
            Input::new("tolerance".to_string(), Value::Decimal(0.1), Some(InputSettings::Slider { range: (0.0, 1.0), step_by: Some(0.001), clamp_to_range: true }), None)
                .with_description("Maximum normalised color distance that still counts as fully selected."),
            Input::new("softness".to_string(), Value::Decimal(0.1), Some(InputSettings::Slider { range: (0.0, 1.0), step_by: Some(0.001), clamp_to_range: true }), None)
                .with_description("Width of the smooth falloff past the tolerance band; zero gives a hard edge."),
        ]
    }

    pub fn create_outputs() -> Vec<Output> {
        vec![
            Output::new("output".to_string(), Value::Image { data: default_image(), change_id: get_id() }, None)
                .with_description("Single-channel mask image; 1 = matches target color, 0 = rejected."),
        ]
    }

    pub async fn run(inputs: &mut [Input]) -> Result<OperationResponse, OperationError> {
        let start_time = Instant::now();
        let mut input_errors: Vec<(usize, String)> = vec![];

        let image_converted = convert_input(inputs, 0, ValueType::Image, &mut input_errors);
        let color_converted = convert_input(inputs, 1, ValueType::Color, &mut input_errors);
        let tolerance_converted = convert_input(inputs, 2, ValueType::Decimal, &mut input_errors);
        let softness_converted = convert_input(inputs, 3, ValueType::Decimal, &mut input_errors);

        if !input_errors.is_empty() { return Err(OperationError { input_errors, node_error: None }); }

        let Value::Image { data, change_id: _ } = image_converted.unwrap() else { unreachable!() };
        let Value::Color(target) = color_converted.unwrap() else { unreachable!() };
        let Value::Decimal(tolerance) = tolerance_converted.unwrap() else { unreachable!() };
        let Value::Decimal(softness) = softness_converted.unwrap() else { unreachable!() };

        let tolerance = tolerance.clamp(0.0, 1.0);
        let softness = softness.max(0.0);
        // Upper edge of the falloff; anything beyond is fully rejected.
        let outer = tolerance + softness;

        let (w, h) = data.dimensions();
        let ch = data.channels() as usize;
        let mut output = FloatImage::new(w, h, 1);

        // Precompute the target grayscale equivalent for 1/2-channel inputs,
        // using the same Rec.709 weights used elsewhere in the codebase.
        let target_luma = 0.2126 * target.r + 0.7152 * target.g + 0.0722 * target.b;

        // sqrt(3) is the maximum Euclidean distance in the unit RGB cube,
        // so dividing by it normalises the distance into [0, 1].
        let norm = 3.0_f32.sqrt();

        for y in 0..h {
            for x in 0..w {
                let px = data.get_pixel(x, y);
                let dist = if ch >= 3 {
                    let dr = px[0] - target.r;
                    let dg = px[1] - target.g;
                    let db = px[2] - target.b;
                    (dr * dr + dg * dg + db * db).sqrt() / norm
                } else {
                    // Grayscale input: compare luminance only.
                    (px[0] - target_luma).abs()
                };

                let mask = smooth_select(dist, tolerance, outer);
                output.put_pixel(x, y, &[mask]);
            }
        }

        Ok(OperationResponse {
            time: Instant::now().duration_since(start_time),
            responses: vec![
                OutputResponse { value: Value::Image { data: Arc::new(output), change_id: get_id() } },
            ],
        })
    }
}

/// Returns 1 when `d <= tol`, 0 when `d >= outer`, and a smoothstep fade
/// between the two. Collapses to a hard threshold if `outer <= tol`.
#[inline]
fn smooth_select(d: f32, tol: f32, outer: f32) -> f32 {
    if d <= tol { return 1.0; }
    if d >= outer || outer <= tol { return 0.0; }
    let t = (d - tol) / (outer - tol);
    // 1 - smoothstep: 1 at t=0, 0 at t=1.
    let s = t * t * (3.0 - 2.0 * t);
    1.0 - s
}

#[cfg(test)]
#[path = "color_to_mask_tests.rs"]
mod tests;
