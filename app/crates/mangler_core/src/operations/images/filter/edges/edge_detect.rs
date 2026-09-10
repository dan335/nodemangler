//! Edge detection operation for images using the Sobel operator.
//!
//! Computes horizontal and vertical gradients using 3x3 Sobel kernels on
//! the Rec. 709 luminance of each pixel, then outputs the gradient magnitude
//! as a grayscale image.

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

/// Edge detection operation using Sobel gradient magnitude on luminance.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpImageAdjustmentEdgeDetect {}

impl OpImageAdjustmentEdgeDetect {
    /// Returns the node metadata (name and description) for the edge detect operation.
    pub fn settings() -> NodeSettings {
        NodeSettings {
            name: "edge detect".to_string(),
            description: "Detects edges using Sobel operator.".to_string(),
            help: "Convolves the Rec. 709 luminance with a 3x3 Sobel Gx and Gy pair, then outputs `sqrt(Gx^2 + Gy^2) * intensity` as a grayscale value on every color channel. Alpha is preserved.\n\nFast single-pass detector; unlike Canny there is no smoothing, non-max suppression, or thresholding, so raw gradients are kept and response scales with noise. Edges are handled by clamping. Intensity acts as a brightness multiplier before clamping to [0, 1].".to_string(),
        }
    }

    /// Creates the input ports: an image and an intensity multiplier for edge strength.
    pub fn create_inputs() -> Vec<Input> {
        vec![
            image_input("image")
                .with_description("Source image whose luminance is analyzed for edge gradients."),
            Input::new("intensity".to_string(), Value::Decimal(1.0), Some(InputSettings::Slider { range: (0.0, 10.0), step_by: Some(0.1), clamp_to_range: true }), None)
                .with_description("Multiplier applied to the Sobel magnitude; higher values make edges brighter."),
        ]
    }

    /// Creates the output port: grayscale edge magnitude image.
    pub fn create_outputs() -> Vec<Output> {
        vec![image_output("output")
            .with_description("Grayscale image of the Sobel gradient magnitude at each pixel.")]
    }

    /// Executes edge detection using Sobel Gx and Gy kernels on Rec. 709 luminance.
    pub async fn run(inputs: &mut [Input]) -> Result<OperationResponse, OperationError> {
        let start_time = Instant::now();

        convert_inputs! { inputs;
            Image(data) = 0,
            Decimal(intensity) = 1,
        }

        // run node — work directly on FloatImage pixels
        let (width, height) = (data.width(), data.height());
        let ch = data.channels() as usize;
        // Only the colour channels are rewritten; alpha is copied from the
        // source, so the output is built fresh rather than cloning the whole
        // image just to preserve one channel.
        let mut output = FloatImage::new(width, height, data.channels());

        // Luminance plane, computed once per pixel: the Sobel stencil reads
        // each neighbour up to twice and eight neighbours per pixel.
        let mut luma_plane = vec![0.0f32; (width as usize) * (height as usize)];
        luma_plane
            .par_chunks_mut((width as usize).max(1))
            .enumerate()
            .for_each(|(y, row)| {
                for (x, v) in row.iter_mut().enumerate() {
                    let p = data.get_pixel(x as u32, y as u32);
                    *v = if ch >= 3 { crate::luma::rec709(p[0], p[1], p[2]) } else { p[0] };
                }
            });
        let lum_at = |px: u32, py: u32| -> f32 {
            luma_plane[py as usize * width as usize + px as usize]
        };

        let row_len = (width as usize * ch).max(1);
        let src = &*data;
        output.as_raw_mut().par_chunks_mut(row_len).enumerate().for_each(|(y, out_row)| {
            let y = y as u32;
            for x in 0..width {
                let x0 = if x > 0 { x - 1 } else { 0 };
                let x2 = if x + 1 < width { x + 1 } else { width - 1 };
                let y0 = if y > 0 { y - 1 } else { 0 };
                let y2 = if y + 1 < height { y + 1 } else { height - 1 };

                // Sobel Gx kernel
                let gx = -lum_at(x0, y0) - 2.0 * lum_at(x0, y) - lum_at(x0, y2)
                        + lum_at(x2, y0) + 2.0 * lum_at(x2, y) + lum_at(x2, y2);
                // Sobel Gy kernel
                let gy = -lum_at(x0, y0) - 2.0 * lum_at(x, y0) - lum_at(x2, y0)
                        + lum_at(x0, y2) + 2.0 * lum_at(x, y2) + lum_at(x2, y2);

                let magnitude = ((gx * gx + gy * gy).sqrt() * intensity).clamp(0.0, 1.0);

                let i = x as usize * ch;
                let pixel = &mut out_row[i..i + ch];
                // Write grayscale magnitude to all color channels
                for c in 0..ch.min(3) {
                    pixel[c] = magnitude;
                }
                // Alpha (channel 1 of 2, or 3 of 4) is carried over untouched.
                if ch == 2 || ch == 4 { pixel[ch - 1] = src.get_pixel(x, y)[ch - 1]; }
            }
        });

        Ok(OperationResponse { 
            time: Instant::now().duration_since(start_time),
            responses: vec![OutputResponse { value: Value::Image { data: Arc::new(output), change_id: get_id() } }],
        })
    }
}

#[cfg(test)]
#[path = "edge_detect_tests.rs"]
mod tests;
