//! Chromatic aberration: per-channel radial scaling (lateral CA).
//!
//! Simulates the fringing a lens produces when it focuses different
//! wavelengths at slightly different magnifications: the red and blue
//! channels are resampled at a small radial scale relative to green, so
//! colour edges separate near the frame's corners while the centre stays
//! aligned. Reuses [`super::transform::sample_bilinear`] for each channel's
//! lookup.

use crate::float_image::FloatImage;
use crate::get_id;
use crate::input::{Input, InputSettings};
use crate::node_settings::NodeSettings;
use crate::operations::images::transform::transform::sample_bilinear;
use crate::operations::{OperationResponse, OperationError, OutputResponse, default_image, convert_input};
use crate::output::Output;
use crate::value::{EdgeMode, Value, ValueType};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use std::time::Instant;

/// Radial scale applied per channel per unit of the `red cyan` / `blue
/// yellow` slider — small enough that the default -1..1 range stays a subtle,
/// physically plausible fringe rather than a cartoonish split.
const CA_STRENGTH: f32 = 0.02;

/// Lateral chromatic aberration via per-channel radial scaling.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpImageTransformChromaticAberration {}

impl OpImageTransformChromaticAberration {
    pub fn settings() -> NodeSettings {
        NodeSettings {
            name: "chromatic aberration".to_string(),
            description: "Per-channel radial fringing (red-cyan / blue-yellow), like a lens's lateral colour separation.".to_string(),
            help: "Lateral chromatic aberration: per-channel radial scaling (red–cyan and blue–yellow fringes); positive values add fringing, negative correct it.\n\nThe red channel is resampled at a uniform scale about the image centre driven by `red cyan` (positive samples further out, spreading red toward the edges relative to green; negative pulls it in). `blue yellow` does the same for the blue channel. Green — and alpha, for images that carry one — are always sampled at their exact, unshifted position, so the centre pixel and any fully aligned (0, 0) case are untouched. `edge` controls what the shifted red/blue samples read once they land outside the source (wrap/extend/mirror, or transparent black in fill mode).\n\nOnly meaningful for images with colour channels: 1- and 2-channel (greyscale, greyscale+alpha) images pass through unchanged since there's no red/blue to separate. 4-channel images are resampled in premultiplied alpha so transparent regions don't bleed hidden colour into the shifted channels — each channel is brought back to straight alpha using the alpha at its own sample position, so soft/partially transparent edges stay in range instead of blowing out.".to_string(),
        }
    }

    pub fn create_inputs() -> Vec<Input> {
        vec![
            Input::new("image".to_string(), Value::Image { data: default_image(), change_id: get_id() }, None, None)
                .with_description("Source image to add or correct fringing on."),
            Input::new("red cyan".to_string(), Value::Decimal(0.0), Some(InputSettings::Slider { range: (-1.0, 1.0), step_by: Some(0.01), clamp_to_range: false }), None)
                .with_description("Radial scale of the red channel; positive spreads red outward (fringe), negative pulls it inward (correction)."),
            Input::new("blue yellow".to_string(), Value::Decimal(0.0), Some(InputSettings::Slider { range: (-1.0, 1.0), step_by: Some(0.01), clamp_to_range: false }), None)
                .with_description("Radial scale of the blue channel; positive spreads blue outward (fringe), negative pulls it inward (correction)."),
            Input::new("edge mode".to_string(), Value::EdgeMode(EdgeMode::Extend), None, None)
                .with_description("How the shifted red/blue samples handle going outside the source: wrap, extend, mirror, or fill (transparent black)."),
        ]
    }

    pub fn create_outputs() -> Vec<Output> {
        vec![
            Output::new("output".to_string(), Value::Image { data: default_image(), change_id: get_id() }, None)
                .with_description("The image with per-channel radial fringing applied, same size and channel count as the input."),
        ]
    }

    pub async fn run(inputs: &mut [Input]) -> Result<OperationResponse, OperationError> {
        let start_time = Instant::now();
        let mut input_errors: Vec<(usize, String)> = vec![];

        let image_converted = convert_input(inputs, 0, ValueType::Image, &mut input_errors);
        let rc_converted = convert_input(inputs, 1, ValueType::Decimal, &mut input_errors);
        let by_converted = convert_input(inputs, 2, ValueType::Decimal, &mut input_errors);
        let edge_converted = convert_input(inputs, 3, ValueType::EdgeMode, &mut input_errors);

        if !input_errors.is_empty() { return Err(OperationError { input_errors, node_error: None }); }

        let Value::Image { data, change_id: _ } = image_converted.unwrap() else { unreachable!() };
        let Value::Decimal(red_cyan) = rc_converted.unwrap() else { unreachable!() };
        let Value::Decimal(blue_yellow) = by_converted.unwrap() else { unreachable!() };
        let Value::EdgeMode(edge) = edge_converted.unwrap() else { unreachable!() };

        let nch = data.channels() as usize;

        // No colour channels to separate, or nothing to do: passthrough.
        if nch < 3 || (red_cyan == 0.0 && blue_yellow == 0.0) {
            return Ok(OperationResponse {
                time: Instant::now().duration_since(start_time),
                responses: vec![OutputResponse { value: Value::Image { data, change_id: get_id() } }],
            });
        }

        let (width, height) = data.dimensions();

        // Premultiply so a shifted red/blue tap near a transparent edge can't
        // pick up hidden colour (no-op for 3-channel: no alpha to carry).
        let premul = data.has_alpha();
        let src = if premul { Arc::new(data.premultiply_alpha()) } else { Arc::clone(&data) };
        // Fixed transparent/black fill: this node exposes no fill colour input.
        let fill_px = vec![0.0f32; nch];

        let cx = width as f32 / 2.0;
        let cy = height as f32 / 2.0;
        let f_r = 1.0 + red_cyan * CA_STRENGTH;
        let f_b = 1.0 + blue_yellow * CA_STRENGTH;

        let mut output = FloatImage::new(width, height, data.channels());
        let mut acc_r = vec![0.0f32; nch];
        let mut acc_g = vec![0.0f32; nch];
        let mut acc_b = vec![0.0f32; nch];
        let mut out_px = [0.0f32; 4];
        for y in 0..height {
            for x in 0..width {
                let dx = x as f32 + 0.5 - cx;
                let dy = y as f32 + 0.5 - cy;

                let sx_r = cx + dx * f_r - 0.5;
                let sy_r = cy + dy * f_r - 0.5;
                // f = 1.0 exactly: unshifted green/alpha sample.
                let sx0 = cx + dx - 0.5;
                let sy0 = cy + dy - 0.5;
                let sx_b = cx + dx * f_b - 0.5;
                let sy_b = cy + dy * f_b - 0.5;

                sample_bilinear(&src, sx_r, sy_r, edge, &fill_px, &mut acc_r);
                sample_bilinear(&src, sx0, sy0, edge, &fill_px, &mut acc_g);
                sample_bilinear(&src, sx_b, sy_b, edge, &fill_px, &mut acc_b);

                if premul {
                    // Each colour channel was interpolated in premultiplied
                    // space *at its own tap*, so it has to be divided by the
                    // alpha interpolated at that same tap. Dividing all three
                    // by the unshifted alpha instead (what a single
                    // `unpremultiply_alpha()` on the result would do) blows
                    // colours up wherever alpha differs between the taps — red
                    // taken from an opaque pixel over an alpha of 0.05 would
                    // land at 20× its true value.
                    out_px[0] = unpremultiply(acc_r[0], acc_r[nch - 1]);
                    out_px[1] = unpremultiply(acc_g[1], acc_g[nch - 1]);
                    out_px[2] = unpremultiply(acc_b[2], acc_b[nch - 1]);
                    // Alpha itself stays at the unshifted position.
                    out_px[3] = acc_g[nch - 1];
                } else {
                    out_px[0] = acc_r[0];
                    out_px[1] = acc_g[1];
                    out_px[2] = acc_b[2];
                }

                output.put_pixel(x, y, &out_px[..nch]);
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

/// Divides a premultiplied colour component by the alpha interpolated at the
/// *same* tap. Mirrors [`FloatImage::unpremultiply_alpha`]'s guard: below the
/// threshold the colour is unrecoverable and zero is the one value that can
/// never bleed (and dividing by a near-zero alpha would amplify interpolation
/// noise into huge colours).
fn unpremultiply(c: f32, a: f32) -> f32 {
    if a > 1e-6 { c / a } else { 0.0 }
}

#[cfg(test)]
#[path = "chromatic_aberration_tests.rs"]
mod tests;
