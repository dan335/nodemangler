//! Convolution-based sharpening operation for images.
//!
//! Applies a 3x3 sharpening kernel where the center weight is boosted and
//! edge weights are negative, enhancing local contrast at edges.

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

/// Convolution-based sharpening operation using a 3x3 edge-enhancement kernel.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpImageAdjustmentSharpen {}

impl OpImageAdjustmentSharpen {
    /// Returns the node metadata (name and description) for the sharpen operation.
    pub fn settings() -> NodeSettings {
        NodeSettings {
            name: "sharpen".to_string(),
            description: "Sharpens an image using a convolution kernel.".to_string(),
            help: "Applies a 3x3 discrete-Laplacian sharpening kernel with center weight `1 + 4*intensity` and cardinal neighbors `-intensity` (corners zero). The result emphasizes local contrast at edges: flat regions pass through, but gradient transitions are amplified.\n\nCheap single-pass convolution; unlike `unsharpen`, there is no tunable blur radius, so it only affects the finest detail. Alpha is preserved; boundaries are handled with edge clamping. High intensity values can push output out of range (clamped to [0, 1]) and exaggerate noise.".to_string(),
        }
    }

    /// Creates the input ports: an image and an intensity controlling sharpening strength.
    pub fn create_inputs() -> Vec<Input> {
        vec![
            image_input("image")
                .with_description("Source image to sharpen with a 3x3 convolution kernel."),
            Input::new("intensity".to_string(), Value::Decimal(1.0), Some(InputSettings::Slider { range: (0.0, 10.0), step_by: Some(0.1), clamp_to_range: true }), None)
                .with_description("Sharpening strength; higher values boost local contrast at edges more aggressively."),
        ]
    }

    /// Creates the output port: the sharpened image.
    pub fn create_outputs() -> Vec<Output> {
        vec![
            image_output("output")
                .with_description("Sharpened image with edge contrast enhanced."),
        ]
    }

    /// Executes the sharpening convolution. Uses edge-clamped sampling for border pixels.
    pub async fn run(inputs: &mut [Input]) -> Result<OperationResponse, OperationError> {
        let start_time = Instant::now();

        convert_inputs! { inputs;
            Image(data) = 0,
            Decimal(intensity) = 1,
        }

        // run node — work directly on FloatImage pixels
        let (width, height) = (data.width(), data.height());
        let mut output = (*data).clone();
        let ch = data.channels() as usize;
        let color_ch = if ch == 2 || ch == 4 { ch - 1 } else { ch };

        // Sharpen kernel: center = 1 + 4*intensity, edges = -intensity, corners = 0
        let center = 1.0 + 4.0 * intensity;
        let edge = -intensity;

        let row_len = (width as usize * ch).max(1);
        let src = &*data;
        output.as_raw_mut().par_chunks_mut(row_len).enumerate().for_each(|(y, out_row)| {
            let y = y as u32;
            for x in 0..width {
                let x0 = if x > 0 { x - 1 } else { 0 };
                let x2 = if x + 1 < width { x + 1 } else { width - 1 };
                let y0 = if y > 0 { y - 1 } else { 0 };
                let y2 = if y + 1 < height { y + 1 } else { height - 1 };

                let c_val = src.get_pixel(x, y);
                let top = src.get_pixel(x, y0);
                let bottom = src.get_pixel(x, y2);
                let left = src.get_pixel(x0, y);
                let right = src.get_pixel(x2, y);

                let i = x as usize * ch;
                let pixel = &mut out_row[i..i + ch];
                for c in 0..color_ch {
                    let val = center * c_val[c]
                        + edge * top[c]
                        + edge * bottom[c]
                        + edge * left[c]
                        + edge * right[c];
                    pixel[c] = val.clamp(0.0, 1.0);
                }
                // alpha unchanged
            }
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
#[path = "sharpen_tests.rs"]
mod tests;
