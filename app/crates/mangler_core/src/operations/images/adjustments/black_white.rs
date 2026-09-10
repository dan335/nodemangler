//! Photoshop-style Black & White adjustment operation.
//!
//! Converts a color image to monochrome using six per-colour-range weights
//! (reds, yellows, greens, cyans, blues, magentas) that scale how bright each
//! hue becomes, then optionally colorizes the result with a tint colour. This
//! is a faithful approximation of Photoshop's Black & White adjustment layer.

use crate::color::Color;
use crate::float_image::FloatImage;
use crate::get_id;
use crate::input::{Input, InputSettings};
use crate::node_settings::NodeSettings;
use crate::convert_inputs;
use crate::operations::{OperationResponse, OperationError, OutputResponse, image_input, image_output};
use crate::operations::images::adjustments::common::rgb_to_hsl;
use crate::output::Output;
use crate::value::Value;
use rayon::prelude::*;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use std::time::Instant;

/// Photoshop-style Black & White: per-colour-range weighted monochrome + optional tint.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpImageAdjustmentBlackWhite {}

impl OpImageAdjustmentBlackWhite {
    /// Node identity and help text shown in the settings panel.
    pub fn settings() -> NodeSettings {
        NodeSettings {
            name: "black and white".to_string(),
            description: "Photoshop-style monochrome conversion with per-colour-range weights and an optional tint.".to_string(),
            help: "A faithful approximation of Photoshop's Black & White adjustment. Each pixel's hue selects which of six colour-range weights (reds, yellows, greens, cyans, blues, magentas) is applied, interpolating between the two nearest anchors around the colour wheel (0deg red, 60deg yellow, 120deg green, 180deg cyan, 240deg blue, 300deg magenta, wrapping back to red).\n\nThe monochrome value for a pixel is `gray = min(r,g,b) + chroma * weight`, where `chroma = max(r,g,b) - min(r,g,b)`. The achromatic base (min) is preserved and only the colourful part (chroma) is scaled by the hue's weight. Raising a weight above its neighbours brightens the areas of that colour; lowering it darkens them. Fully desaturated pixels (r=g=b) have zero chroma and therefore pass through as the same gray regardless of the weights.\n\nAn optional tint colorizes the grayscale result: the gray value is multiplied by the tint colour, and `tint amount` blends between the pure gray (0) and the fully tinted colour (1), producing classic sepia / duotone looks.\n\nImages with fewer than 3 channels have no hue information and are passed through unchanged. Alpha is preserved.".to_string(),
        }
    }

    /// Input list: the image, six colour-range weights, and the tint controls.
    pub fn create_inputs() -> Vec<Input> {
        // Slider spec shared by all six colour-range weight inputs.
        let weight_slider = || Some(InputSettings::Slider { range: (-2.0, 3.0), step_by: Some(0.01), clamp_to_range: false });
        vec![
            image_input("image")
                .with_description("Source color image to convert to monochrome."),
            Input::new("reds".to_string(), Value::Decimal(0.4), weight_slider(), None)
                .with_description("Brightness weight for red areas (hue ~0deg)."),
            Input::new("yellows".to_string(), Value::Decimal(0.6), weight_slider(), None)
                .with_description("Brightness weight for yellow areas (hue ~60deg)."),
            Input::new("greens".to_string(), Value::Decimal(0.4), weight_slider(), None)
                .with_description("Brightness weight for green areas (hue ~120deg)."),
            Input::new("cyans".to_string(), Value::Decimal(0.6), weight_slider(), None)
                .with_description("Brightness weight for cyan areas (hue ~180deg)."),
            Input::new("blues".to_string(), Value::Decimal(0.2), weight_slider(), None)
                .with_description("Brightness weight for blue areas (hue ~240deg)."),
            Input::new("magentas".to_string(), Value::Decimal(0.8), weight_slider(), None)
                .with_description("Brightness weight for magenta areas (hue ~300deg)."),
            Input::new("tint".to_string(), Value::Color(Color { r: 0.86, g: 0.72, b: 0.50, a: 1.0 }), None, None)
                .with_description("Colour used to tint the monochrome result."),
            Input::new("tint amount".to_string(), Value::Decimal(0.0), Some(InputSettings::Slider { range: (0.0, 1.0), step_by: Some(0.01), clamp_to_range: true }), None)
                .with_description("Blend from pure gray (0) to fully tinted (1)."),
        ]
    }

    /// Single image output.
    pub fn create_outputs() -> Vec<Output> {
        vec![
            image_output("output")
                .with_description("Monochrome (optionally tinted) image."),
        ]
    }

    /// Applies the weighted monochrome conversion and optional tint per pixel.
    pub async fn run(inputs: &mut [Input]) -> Result<OperationResponse, OperationError> {
        let start_time = Instant::now();

        // Convert each input to its expected type, collecting any conversion errors.
        convert_inputs! { inputs;
            Image(data) = 0,
            Decimal(reds) = 1,
            Decimal(yellows) = 2,
            Decimal(greens) = 3,
            Decimal(cyans) = 4,
            Decimal(blues) = 5,
            Decimal(magentas) = 6,
            Color(tint) = 7,
            Decimal(tint_amount) = 8,
        }

        let (w, h) = data.dimensions();
        let ch = data.channels() as usize;

        // Grayscale (or gray+alpha) inputs carry no hue, so there is nothing to
        // weight — pass them straight through unchanged.
        if ch < 3 {
            return Ok(OperationResponse {
                time: Instant::now().duration_since(start_time),
                responses: vec![
                    OutputResponse { value: Value::Image { data, change_id: get_id() } },
                ],
            });
        }

        // The six weights ordered around the colour wheel starting at red.
        let weights = [reds, yellows, greens, cyans, blues, magentas];

        let mut output = FloatImage::new(w, h, ch as u32);
        output
            .par_pixels_mut()
            .zip(data.par_pixels())
            .for_each(|(dst, src)| {
                let mut buf = [0.0f32; 4];
                let (r, g, b) = (src[0], src[1], src[2]);

                // Achromatic base (min) plus chroma-scaled hue weight.
                let mx = r.max(g).max(b);
                let mn = r.min(g).min(b);
                let chroma = mx - mn;
                let hue = rgb_to_hsl(r, g, b).0; // 0..360
                let wgt = interp_hue_weight(hue, weights);
                let gray = mn + chroma * wgt;

                // Optional tint: multiply gray by the tint colour, then blend
                // from pure gray toward the tinted colour by `tint amount`.
                let base = gray;
                let tinted = [base * tint.r, base * tint.g, base * tint.b];
                buf[0] = base + (tinted[0] - base) * tint_amount;
                buf[1] = base + (tinted[1] - base) * tint_amount;
                buf[2] = base + (tinted[2] - base) * tint_amount;

                // Preserve alpha if present.
                if ch == 4 { buf[3] = src[3]; }
                dst.copy_from_slice(&buf[..ch]);
            });

        Ok(OperationResponse {
            time: Instant::now().duration_since(start_time),
            responses: vec![
                OutputResponse { value: Value::Image { data: Arc::new(output), change_id: get_id() } },
            ],
        })
    }
}

/// Interpolates a colour-range weight for a given hue.
///
/// The six weights are treated as anchors placed at 0deg (red), 60deg (yellow),
/// 120deg (green), 180deg (cyan), 240deg (blue) and 300deg (magenta). For a hue
/// in `[0, 360)` this linearly interpolates between the two nearest anchors,
/// wrapping past 300deg back through 360deg to the red anchor at 0deg.
fn interp_hue_weight(hue: f32, weights: [f32; 6]) -> f32 {
    // Normalise the hue into [0, 360).
    let mut h = hue % 360.0;
    if h < 0.0 { h += 360.0; }

    // Each anchor spans 60 degrees; find the lower anchor index and the
    // fractional position between it and the next anchor.
    let seg = h / 60.0;
    let i = seg.floor() as usize; // 0..=5 (6 only if h == 360, guarded above)
    let frac = seg - seg.floor();
    let lo = weights[i % 6];
    let hi = weights[(i + 1) % 6]; // wraps magenta -> red
    lo + (hi - lo) * frac
}

#[cfg(test)]
#[path = "black_white_tests.rs"]
mod tests;
