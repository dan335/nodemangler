//! Channel mixer.
//!
//! For each output channel R/G/B, produces a linear combination of the input
//! R, G, B channels plus a constant bias. This is the classic Photoshop
//! "Channel Mixer" adjustment — far more expressive than `channel shuffle`
//! (which picks one source per output) because it can blend channels with
//! arbitrary real coefficients and even invert their sign.
//!
//! Typical uses: custom grayscale via per-channel luminance weights, DirectX
//! ↔ OpenGL normal-map handoffs requiring G-channel inversion, swapping
//! colour spaces via a matrix, or desaturating only part of the spectrum.

use crate::float_image::FloatImage;
use crate::get_id;
use crate::input::{Input, InputSettings};
use crate::node_settings::NodeSettings;
use crate::operations::{OperationResponse, OperationError, OutputResponse, default_image, convert_input};
use crate::output::Output;
use crate::value::{Value, ValueType};
use rayon::prelude::*;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use std::time::Instant;

/// Minimum pixel count before the mix is parallelized over rows.
const PARALLEL_PIXELS: usize = 1 << 16;

/// Per-output-channel linear combinations of the input R/G/B channels plus bias.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpImageChannelMixer {}

impl OpImageChannelMixer {
    pub fn settings() -> NodeSettings {
        NodeSettings {
            name: "channel mixer".to_string(),
            description: "Per-output-channel linear combination of the input R/G/B plus bias.".to_string(),
            help: "Each output channel is computed as `Rout = rR*R + rG*G + rB*B + rBias` (and similarly for G and B). Coefficients are unclamped, so negative weights invert contribution and sums above 1 brighten. After combining, each channel is clamped to [0, 1] so downstream nodes see well-formed data.\n\nDefaults form the identity matrix (rR = gG = bB = 1, everything else 0). Use cases: custom grayscale via `Rout = Gout = Bout = 0.299R + 0.587G + 0.114B`, sepia tone, or subtle colour grading without affecting only one channel. Alpha (when present) is copied through unchanged.".to_string(),
        }
    }

    pub fn create_inputs() -> Vec<Input> {
        // 12 coefficients form a 3x3 mix matrix plus a per-output bias triple.
        fn coef(label: &str, default: f32, desc: &str) -> Input {
            Input::new(label.to_string(), Value::Decimal(default),
                Some(InputSettings::Slider { range: (-2.0, 2.0), step_by: Some(0.01), clamp_to_range: false }),
                None)
                .with_description(desc)
        }
        fn bias(label: &str, desc: &str) -> Input {
            Input::new(label.to_string(), Value::Decimal(0.0),
                Some(InputSettings::Slider { range: (-1.0, 1.0), step_by: Some(0.01), clamp_to_range: false }),
                None)
                .with_description(desc)
        }

        vec![
            Input::new("image".to_string(), Value::Image { data: default_image(), change_id: get_id() }, None, None)
                .with_description("Source image whose channels will be remixed."),
            coef("r from r", 1.0, "Coefficient for red in the output red channel."),
            coef("r from g", 0.0, "Coefficient for green in the output red channel."),
            coef("r from b", 0.0, "Coefficient for blue in the output red channel."),
            bias("r bias", "Constant added to the output red channel."),
            coef("g from r", 0.0, "Coefficient for red in the output green channel."),
            coef("g from g", 1.0, "Coefficient for green in the output green channel."),
            coef("g from b", 0.0, "Coefficient for blue in the output green channel."),
            bias("g bias", "Constant added to the output green channel."),
            coef("b from r", 0.0, "Coefficient for red in the output blue channel."),
            coef("b from g", 0.0, "Coefficient for green in the output blue channel."),
            coef("b from b", 1.0, "Coefficient for blue in the output blue channel."),
            bias("b bias", "Constant added to the output blue channel."),
        ]
    }

    pub fn create_outputs() -> Vec<Output> {
        vec![
            Output::new("output".to_string(), Value::Image { data: default_image(), change_id: get_id() }, None)
                .with_description("Remixed image with the channel linear combinations applied."),
        ]
    }

    pub async fn run(inputs: &mut [Input]) -> Result<OperationResponse, OperationError> {
        let start_time = Instant::now();
        let mut input_errors: Vec<(usize, String)> = vec![];

        // Convert the image, then each of the 12 coefficients in order.
        let image_converted = convert_input(inputs, 0, ValueType::Image, &mut input_errors);
        let mut coeffs = [0.0f32; 12];
        for (i, coeff) in coeffs.iter_mut().enumerate() {
            if let Some(Value::Decimal(v)) = convert_input(inputs, i + 1, ValueType::Decimal, &mut input_errors) {
                *coeff = v;
            }
        }

        if !input_errors.is_empty() { return Err(OperationError { input_errors, node_error: None }); }

        let Value::Image { data, change_id: _ } = image_converted.unwrap() else { unreachable!() };

        let (w, h) = data.dimensions();
        let ch = data.channels() as usize;
        let wu = w as usize;
        let src_data = data.as_raw();

        // If the input is grayscale we mix only via the red row and emit a 1-
        // or 2-channel result matching the input shape.
        let mut out_data = vec![0.0f32; src_data.len()];

        // Mix one row; the channel-count dispatch is hoisted out of the pixel loop.
        let process_row = |(dst_row, src_row): (&mut [f32], &[f32])| {
            let pairs = dst_row.chunks_exact_mut(ch).zip(src_row.chunks_exact(ch));
            if ch >= 3 {
                pairs.for_each(|(dst, src)| {
                    let (sr, sg, sb) = (src[0], src[1], src[2]);
                    // R row = coeffs[0..3], bias = coeffs[3]. Likewise for G and B rows.
                    let r = sr * coeffs[0] + sg * coeffs[1] + sb * coeffs[2] + coeffs[3];
                    let g = sr * coeffs[4] + sg * coeffs[5] + sb * coeffs[6] + coeffs[7];
                    let b = sr * coeffs[8] + sg * coeffs[9] + sb * coeffs[10] + coeffs[11];
                    dst[0] = r.clamp(0.0, 1.0);
                    dst[1] = g.clamp(0.0, 1.0);
                    dst[2] = b.clamp(0.0, 1.0);
                    if ch == 4 { dst[3] = src[3]; }
                });
            } else {
                // Grayscale result: use the R row only (user can still apply a
                // custom luminance weighting via `r from r/g/b`). Treat 1/2
                // channel sources as grayscale (R=G=B=ch0).
                pairs.for_each(|(dst, src)| {
                    let (sr, sg, sb) = (src[0], src[0], src[0]);
                    let r = sr * coeffs[0] + sg * coeffs[1] + sb * coeffs[2] + coeffs[3];
                    dst[0] = r.clamp(0.0, 1.0);
                    if ch == 2 { dst[1] = src[1]; }
                });
            }
        };

        if wu > 0 {
            let row_len = wu * ch;
            if wu * h as usize >= PARALLEL_PIXELS {
                out_data.par_chunks_exact_mut(row_len).zip(src_data.par_chunks_exact(row_len)).for_each(process_row);
            } else {
                out_data.chunks_exact_mut(row_len).zip(src_data.chunks_exact(row_len)).for_each(process_row);
            }
        }

        let output = FloatImage::from_raw(w, h, ch as u32, out_data).unwrap();

        Ok(OperationResponse {
            time: Instant::now().duration_since(start_time),
            responses: vec![
                OutputResponse { value: Value::Image { data: Arc::new(output), change_id: get_id() } },
            ],
        })
    }
}

#[cfg(test)]
#[path = "mixer_tests.rs"]
mod tests;
