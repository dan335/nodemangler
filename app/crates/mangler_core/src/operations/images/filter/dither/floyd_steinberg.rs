//! Floyd–Steinberg error-diffusion dithering.
//!
//! For each pixel in scan order:
//!   1. Quantize the current value to the nearest palette level.
//!   2. Compute the quantization error (old - quantized).
//!   3. Distribute that error to the four unprocessed neighbors using the
//!      classic Floyd–Steinberg weights:
//!
//!         ```text
//!                   [ *  7/16 ]
//!         [ 3/16  5/16  1/16 ]
//!         ```
//!
//!      where `*` is the current pixel and the bottom row is the next line.
//!
//! Because each pixel's value depends on the error pushed from earlier
//! pixels, this cannot be trivially parallelized per-pixel. The scan is
//! serial but still fast (single pass, bounded work per pixel).
//!
//! Output is per-channel quantized to `levels` values; the alpha channel is
//! passed through unchanged so dithering doesn't interact with transparency.

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

/// Floyd–Steinberg error-diffusion dithering filter.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpImageAdjustmentFloydSteinberg {}

impl OpImageAdjustmentFloydSteinberg {
    /// Returns the node metadata for the Floyd–Steinberg filter.
    pub fn settings() -> NodeSettings {
        NodeSettings {
            name: "floyd steinberg".to_string(),
            description: "Floyd–Steinberg error-diffusion dithering. Distributes quantization error to unprocessed neighbors.".to_string(),
            help: "In scan order, quantizes each pixel to the nearest of `levels` per-channel palette steps, then pushes the quantization error to the four unprocessed neighbors with weights 7/16 right, 3/16 down-left, 5/16 down, and 1/16 down-right.\n\nConverts banding into a high-frequency dot pattern that looks visually smooth. Inherently serial because each pixel depends on error from earlier pixels, so it runs single-threaded. Alpha is passed through unchanged so transparency doesn't interact with the dither.".to_string(),
        }
    }

    /// Creates input ports: image and the per-channel palette size (levels).
    pub fn create_inputs() -> Vec<Input> {
        vec![
            Input::new("image".to_string(), Value::Image { data: default_image(), change_id: get_id() }, None, None)
                .with_description("Source image to quantize with Floyd-Steinberg error diffusion."),
            // Number of quantization levels per channel; 2 → 1-bit per channel
            Input::new("levels".to_string(), Value::Integer(2), Some(InputSettings::Slider { range: (2.0, 16.0), step_by: Some(1.0), clamp_to_range: true }), None)
                .with_description("Number of quantization steps per channel; 2 yields 1-bit per channel."),
        ]
    }

    /// Creates the output port: the dithered image.
    pub fn create_outputs() -> Vec<Output> {
        vec![
            Output::new("output".to_string(), Value::Image { data: default_image(), change_id: get_id() }, None)
                .with_description("Dithered image with quantization error diffused to neighboring pixels."),
        ]
    }

    /// Runs the Floyd–Steinberg filter.
    pub async fn run(inputs: &mut [Input]) -> Result<OperationResponse, OperationError> {
        let start_time = Instant::now();
        let mut input_errors: Vec<(usize, String)> = vec![];

        let image_converted = convert_input(inputs, 0, ValueType::Image, &mut input_errors);
        let levels_converted = convert_input(inputs, 1, ValueType::Integer, &mut input_errors);

        if !input_errors.is_empty() { return Err(OperationError { input_errors, node_error: None }); }

        let Value::Image { data, change_id: _ } = image_converted.unwrap() else { unreachable!() };
        let Value::Integer(levels) = levels_converted.unwrap() else { unreachable!() };

        let levels = levels.max(2) as u32;
        let q_step = (levels - 1) as f32;

        let (width, height) = data.dimensions();
        let ch = data.channels() as usize;
        // Diffuse only over color channels; alpha is copied verbatim
        let color_ch = if ch == 2 || ch == 4 { ch - 1 } else { ch };

        // Working buffer: flat interleaved f32, same layout as FloatImage
        let mut buf: Vec<f32> = data.as_raw().to_vec();
        let w = width as i32;
        let h = height as i32;
        let wu = width as usize;

        for y in 0..h {
            for x in 0..w {
                let idx = (y as usize * wu + x as usize) * ch;
                for c in 0..color_ch {
                    // Read current (possibly error-biased) value, quantize, write back
                    let old = buf[idx + c];
                    let q = (old.clamp(0.0, 1.0) * q_step).round() / q_step;
                    buf[idx + c] = q;
                    let err = old - q;

                    // Distribute the quantization error to the unprocessed
                    // neighbors. Out-of-bounds neighbors are simply skipped
                    // (their share of the error is discarded).
                    if x + 1 < w {
                        let nidx = (y as usize * wu + (x + 1) as usize) * ch + c;
                        buf[nidx] += err * (7.0 / 16.0);
                    }
                    if y + 1 < h {
                        if x > 0 {
                            let nidx = ((y + 1) as usize * wu + (x - 1) as usize) * ch + c;
                            buf[nidx] += err * (3.0 / 16.0);
                        }
                        let nidx = ((y + 1) as usize * wu + x as usize) * ch + c;
                        buf[nidx] += err * (5.0 / 16.0);
                        if x + 1 < w {
                            let nidx = ((y + 1) as usize * wu + (x + 1) as usize) * ch + c;
                            buf[nidx] += err * (1.0 / 16.0);
                        }
                    }
                }
                // Alpha: preserve exactly (no error diffusion on transparency)
                if ch == 2 || ch == 4 {
                    let src_alpha = data.as_raw()[idx + ch - 1];
                    buf[idx + ch - 1] = src_alpha;
                }
            }
        }

        let out = FloatImage::from_raw(width, height, data.channels(), buf).unwrap();

        Ok(OperationResponse {
            time: Instant::now().duration_since(start_time),
            responses: vec![
                OutputResponse { value: Value::Image { data: Arc::new(out), change_id: get_id() } },
            ],
        })
    }
}

#[cfg(test)]
#[path = "floyd_steinberg_tests.rs"]
mod tests;
