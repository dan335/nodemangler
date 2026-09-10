//! Ordered (Bayer) dithering.
//!
//! Converts continuous tones into a quantized palette using a Bayer threshold
//! matrix. For each pixel, the matrix supplies a per-pixel offset that is
//! added before quantization; pixels where the source value is above the
//! matrix threshold round up to the next quantization level, otherwise down.
//!
//! The Bayer matrices are generated recursively from the 2×2 seed:
//!     `B₁ = [[0, 2], [3, 1]]`
//!     `B_{2n}(x, y) = 4·B_n(x, y) + B_2(x/n, y/n)`
//! Values are normalized into (-½, ½] around the center so the dither pattern
//! has zero mean and doesn't shift the overall brightness.
//!
//! Quantization is per-channel. `levels = 2` gives the classic 1-bit look;
//! higher levels produce the retro "reduced palette" aesthetic (EGA/VGA).

use crate::float_image::FloatImage;
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

/// Ordered (Bayer-matrix) dithering filter.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpImageAdjustmentOrderedDither {}

impl OpImageAdjustmentOrderedDither {
    /// Returns the node metadata for the ordered dither filter.
    pub fn settings() -> NodeSettings {
        NodeSettings {
            name: "ordered dither".to_string(),
            description: "Bayer-matrix ordered dither — quantize to N levels using a tiled threshold pattern.".to_string(),
            help: "Tiles a Bayer threshold matrix across the image (sizes 2, 4, or 8), adds a per-pixel offset scaled by the quantization step, and snaps each color channel to the nearest of `levels` palette steps. The matrix is recursively generated from the 2x2 seed and zero-centered so the dither has zero mean and does not shift overall brightness.\n\nUnlike Floyd-Steinberg, the pattern is deterministic and fully parallel — no error propagation. `levels = 2` gives the classic 1-bit look; higher values produce retro EGA/VGA palettes. Alpha is preserved.".to_string(),
        }
    }

    /// Creates input ports: image, matrix size (2/4/8), and quantization levels.
    pub fn create_inputs() -> Vec<Input> {
        vec![
            image_input("image")
                .with_description("Source image to quantize via a tiled Bayer threshold matrix."),
            // Matrix size — only 2, 4, or 8 are meaningful; larger values are clamped
            Input::new("matrix size".to_string(), Value::Integer(4), Some(InputSettings::Slider { range: (2.0, 8.0), step_by: Some(2.0), clamp_to_range: true }), None)
                .with_description("Bayer matrix size; larger matrices give finer dot patterns (snapped to 2, 4, or 8)."),
            // Levels per channel: 2 = 1-bit black/white; higher = retro palette
            Input::new("levels".to_string(), Value::Integer(2), Some(InputSettings::Slider { range: (2.0, 16.0), step_by: Some(1.0), clamp_to_range: true }), None)
                .with_description("Quantization levels per channel; 2 gives 1-bit, higher values give a retro palette."),
        ]
    }

    /// Creates the output port: the dithered image.
    pub fn create_outputs() -> Vec<Output> {
        vec![
            image_output("output")
                .with_description("Ordered-dithered image quantized via the tiled Bayer pattern."),
        ]
    }

    /// Runs the ordered dither filter.
    pub async fn run(inputs: &mut [Input]) -> Result<OperationResponse, OperationError> {
        let start_time = Instant::now();

        convert_inputs! { inputs;
            Image(data) = 0,
            Integer(size) = 1,
            Integer(levels) = 2,
        }

        // Snap matrix size to the next supported value (2, 4, or 8); anything
        // larger collapses to 8 because our recursion generator tops out there
        let matrix_size = match size {
            s if s <= 2 => 2usize,
            s if s <= 4 => 4,
            _ => 8,
        };
        let levels = levels.max(2) as u32;

        // Build the Bayer threshold matrix, zero-centered in (-½, ½]
        let bayer = build_bayer(matrix_size);

        let (width, height) = data.dimensions();
        let ch = data.channels() as usize;
        // Quantization step size per channel (distance between adjacent levels)
        let step = 1.0 / (levels - 1) as f32;
        // Alpha is quantized through untouched dithering but kept in-range
        let color_ch = if ch == 2 || ch == 4 { ch - 1 } else { ch };

        let mut out = FloatImage::new(width, height, ch as u32);
        // Ordered dithering carries no state between pixels (unlike error
        // diffusion): the threshold is a pure function of (x % m, y % m).
        let src_img = &*data;
        out.par_enumerate_pixels_mut().for_each(|(x, y, out_px)| {
            // Sample the tiled Bayer matrix; the offset is scaled by the
            // quantization step so it nudges pixels across level boundaries
            let t = bayer[(y as usize % matrix_size) * matrix_size + (x as usize % matrix_size)];
            let offset = t * step;
            let src = src_img.get_pixel(x, y);
            let mut pixel = [0.0f32; 4];
            for c in 0..color_ch {
                // Add the dither offset, then snap to the nearest quant level
                let v = (src[c] + offset).clamp(0.0, 1.0);
                let q = (v * (levels - 1) as f32).round() / (levels - 1) as f32;
                pixel[c] = q;
            }
            if ch == 2 || ch == 4 { pixel[ch - 1] = src[ch - 1]; }
            out_px.copy_from_slice(&pixel[..ch]);
        });

        Ok(OperationResponse {
            time: Instant::now().duration_since(start_time),
            responses: vec![
                OutputResponse { value: Value::Image { data: Arc::new(out), change_id: get_id() } },
            ],
        })
    }
}

/// Builds a `size × size` Bayer dither matrix recursively, with values
/// mapped into (-½, ½] so the matrix has zero mean.
///
/// Only sizes 2, 4, and 8 are valid — all others are routed here via the
/// caller's clamp. The matrix is returned row-major.
fn build_bayer(size: usize) -> Vec<f32> {
    // Integer matrices from the classic recursion
    let m2: [u32; 4] = [0, 2, 3, 1];

    let ints: Vec<u32> = match size {
        2 => m2.to_vec(),
        4 => {
            // B₄(x, y) = 4·B₂(x mod 2, y mod 2) + B₂(x/2, y/2)
            // with values in [0, 15]
            let mut m = vec![0u32; 16];
            for y in 0..4 {
                for x in 0..4 {
                    let a = m2[(y % 2) * 2 + (x % 2)];
                    let b = m2[(y / 2) * 2 + (x / 2)];
                    m[y * 4 + x] = 4 * a + b;
                }
            }
            m
        }
        _ => {
            // B₈(x, y) = 4·B₄(x mod 4, y mod 4) + B₂(x/4, y/4)
            // Values in [0, 63]
            let m4 = {
                let mut m = vec![0u32; 16];
                for y in 0..4 {
                    for x in 0..4 {
                        let a = m2[(y % 2) * 2 + (x % 2)];
                        let b = m2[(y / 2) * 2 + (x / 2)];
                        m[y * 4 + x] = 4 * a + b;
                    }
                }
                m
            };
            let mut m = vec![0u32; 64];
            for y in 0..8 {
                for x in 0..8 {
                    let a = m4[(y % 4) * 4 + (x % 4)];
                    let b = m2[(y / 4) * 2 + (x / 4)];
                    m[y * 8 + x] = 4 * a + b;
                }
            }
            m
        }
    };

    // Normalize: (i + 0.5) / N² - 0.5 maps integer bins in [0, N²) to
    // continuous offsets in (-½, ½] with zero mean
    let n2 = (size * size) as f32;
    ints.into_iter().map(|v| (v as f32 + 0.5) / n2 - 0.5).collect()
}

#[cfg(test)]
#[path = "ordered_dither_tests.rs"]
mod tests;
