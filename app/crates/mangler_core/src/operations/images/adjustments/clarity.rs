//! Clarity: midtone local-contrast enhancement for images.
//!
//! "Clarity" boosts (or softens) local contrast in the midtones the way the
//! Lightroom/Camera-Raw slider does. It works by building a large-radius blur of
//! the image's luminance, extracting the high-frequency `detail = luma - blur`
//! (an unsharp mask), and adding a midtone-weighted fraction of that detail back
//! into the luminance. The midtone weight tapers to zero at pure black and pure
//! white, which protects deep shadows and bright highlights from the halos a
//! naive unsharp mask would produce. The colour of each pixel is preserved by
//! scaling all channels by the same `new_luma / luma` ratio.
//!
//! This is a heuristic effect, not a physically-grounded model.

use crate::get_id;
use crate::input::{Input, InputSettings};
use crate::node_settings::NodeSettings;
use crate::convert_inputs;
use crate::operations::{OperationResponse, OperationError, OutputResponse, default_image, scale_to_resolution, image_input};
use crate::operations::images::blur::blur::box_blur_planar_passes;
use crate::output::Output;
use crate::value::Value;
use rayon::prelude::*;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use std::time::Instant;

/// Midtone local-contrast (clarity) adjustment operation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpImageAdjustmentClarity {}

/// Box-blurs a single-channel buffer with three separable moving-average passes.
///
impl OpImageAdjustmentClarity {
    /// Returns the node metadata (name, description, help) for the clarity operation.
    pub fn settings() -> NodeSettings {
        NodeSettings {
            name: "clarity".to_string(),
            description: "Midtone local-contrast enhancement. Positive adds punch, negative softens.".to_string(),
            help: "Enhances local contrast (\"clarity\") by applying an unsharp mask to the image's luminance with a large radius. The luminance is blurred (three box-blur passes approximating a Gaussian), and the high-frequency detail `luma - blur` is added back into the luminance, scaled by the amount.\n\nThe added detail is midtone-weighted: the weight is `1 - |2*luma - 1|`, which is 1 at mid-grey and falls to 0 at pure black and pure white. This protects deep shadows and bright highlights from the halos a plain unsharp mask would create. Each pixel's colour is preserved by multiplying all colour channels by the same `new_luma / luma` ratio; alpha is untouched.\n\nPositive amounts add punch and structure; negative amounts soften local contrast for a hazier look. Radius is authored in pixels at a 1024px reference and scaled to the actual image, so the same value produces the same relative effect at any resolution. Output is not clamped. This is a heuristic, not a physical model.".to_string(),
        }
    }

    /// Creates the input ports: source image, signed amount, and blur radius.
    pub fn create_inputs() -> Vec<Input> {
        vec![
            image_input("image")
                .with_description("Source image to enhance."),
            Input::new("amount".to_string(), Value::Decimal(0.0), Some(InputSettings::Slider { range: (-1.0, 1.0), step_by: Some(0.01), clamp_to_range: true }), None)
                .with_description("Local-contrast strength; positive adds midtone punch, negative softens, 0 leaves the image unchanged."),
            Input::new("radius".to_string(), Value::Decimal(50.0), Some(InputSettings::DragValue { speed: None, clamp: Some((1.0, 300.0)) }), None)
                .with_description("Blur radius in pixels at a 1024px reference (scales with image size); larger values act on coarser local contrast."),
        ]
    }

    /// Creates the output port: the clarity-adjusted image.
    pub fn create_outputs() -> Vec<Output> {
        vec![
            Output::new("output".to_string(), Value::Image { data: default_image(), change_id: get_id() }, None)
                .with_description("Image with midtone local contrast enhanced, alpha preserved."),
        ]
    }

    /// Executes the clarity operation.
    pub async fn run(inputs: &mut [Input]) -> Result<OperationResponse, OperationError> {
        let start_time = Instant::now();

        convert_inputs! { inputs;
            Image(data) = 0,
            Decimal(amount) = 1,
            Decimal(radius) = 2,
        }

        let amount = amount as f32;

        if amount == 0.0 {
            // Zero strength: the unsharp-mask contribution is exactly zero at
            // every pixel, so the output is bit-identical to the input.
            return Ok(OperationResponse {
                time: Instant::now().duration_since(start_time),
                responses: vec![
                    OutputResponse { value: Value::Image { data, change_id: get_id() } },
                ],
            });
        }

        let mut result = (*data).clone();
        let (w, h) = result.dimensions();
        let ch = result.channels() as usize;

        // Radius is authored in reference pixels (at 1024px) and scaled to the
        // actual image so the effect looks the same at any resolution.
        let r = scale_to_resolution(radius as f32, w, h).round().max(1.0) as u32;

        // Build a per-pixel luma buffer. Colour images use Rec.709 luma; images
        // with fewer than three channels use channel 0 directly.
        let luma: Vec<f32> = result
            .par_pixels()
            .map(|px| if ch >= 3 { crate::luma::rec709(px[0], px[1], px[2]) } else { px[0] })
            .collect();

        // Large-radius blur of the luma buffer approximates the local average.
        let blurred = box_blur_planar_passes(&luma, w, h, r, 3);

        // Apply the midtone-weighted unsharp mask, preserving colour by scaling.
        // Each pixel depends only on its own luma and blurred luma, so the pass
        // is run in parallel over the pixel slices rather than walking (x, y)
        // through bounds-checked `get_pixel_mut`.
        let color_n = if ch == 4 { 3 } else { ch };
        result
            .par_pixels_mut()
            .zip(luma.par_iter())
            .zip(blurred.par_iter())
            .for_each(|((px, &l), &b)| {
                // High-frequency detail (unsharp mask).
                let detail = l - b;
                // Midtone weight: 1 at mid-grey, 0 at black/white; never negative.
                let mid = (1.0f32 - (2.0 * l - 1.0).abs()).max(0.0);
                // New luminance with the weighted detail added back.
                let new_luma = l + amount * detail * mid;

                if ch >= 3 {
                    // Preserve hue: scale all colour channels by the luma ratio.
                    let scale = if l.abs() > 1e-5 { new_luma / l } else { 1.0 };
                    for c in 0..color_n {
                        px[c] *= scale;
                    }
                    // Alpha (channel 3 for RGBA) is left untouched.
                } else {
                    // Single/luma+alpha image: set channel 0 to the new luma.
                    px[0] = new_luma;
                    // For a 2-channel (luma+alpha) image, alpha in channel 1 is untouched.
                }
            });

        Ok(OperationResponse {
            time: Instant::now().duration_since(start_time),
            responses: vec![
                OutputResponse { value: Value::Image { data: Arc::new(result), change_id: get_id() } },
            ],
        })
    }
}

#[cfg(test)]
#[path = "clarity_tests.rs"]
mod tests;
