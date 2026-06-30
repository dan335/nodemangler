//! Custom 3x3 convolution filter.
//!
//! Applies a user-supplied 3x3 kernel to each colour channel, then divides by
//! `divisor` and adds `bias`. Edges are handled by clamping. The default kernel
//! is the identity (centre 1, everything else 0). Alpha is preserved.

use crate::get_id;
use crate::value::ValueType;
use crate::input::{Input, InputSettings};
use crate::node_settings::NodeSettings;
use crate::operations::{OperationResponse, OperationError, OutputResponse, default_image, convert_input};
use crate::output::Output;
use crate::value::Value;
use crate::float_image::FloatImage;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use std::time::Instant;

/// A single kernel-weight slider.
fn weight_input(name: &str, default: f32) -> Input {
    Input::new(
        name.to_string(),
        Value::Decimal(default),
        Some(InputSettings::DragValue { clamp: None, speed: Some(0.05) }),
        None,
    )
    .with_description("Kernel weight applied to this neighbour.".to_string())
}

/// Generic 3x3 convolution with divisor and bias.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpImageAdjustmentConvolution {}

impl OpImageAdjustmentConvolution {
    /// Returns the node metadata (name and description) for convolution.
    pub fn settings() -> NodeSettings {
        NodeSettings {
            name: "convolution".to_string(),
            description: "Applies a custom 3x3 convolution kernel with divisor and bias.".to_string(),
            help: "For every pixel the 3x3 neighbourhood is multiplied element-wise by the nine kernel weights and summed per colour channel, then divided by `divisor` and offset by `bias`: out = sum(kernel * neighbours) / divisor + bias. This is the general primitive behind blur, sharpen, edge, and emboss filters — set the weights to build any of them.\n\nThe default kernel is the identity (centre 1, others 0), a no-op. A divisor of 0 is treated as 1 to avoid division by zero; use it to normalize box/blur kernels (e.g. all-ones with divisor 9). Edges clamp to the nearest pixel and alpha is passed through unchanged. Output dimensions and channel count match the input.".to_string(),
        }
    }

    /// Creates input ports: image, nine kernel weights, divisor, and bias.
    pub fn create_inputs() -> Vec<Input> {
        vec![
            Input::new("image".to_string(), Value::Image { data: default_image(), change_id: get_id() }, None, None)
                .with_description("Source image to convolve."),
            weight_input("k00", 0.0), weight_input("k01", 0.0), weight_input("k02", 0.0),
            weight_input("k10", 0.0), weight_input("k11", 1.0), weight_input("k12", 0.0),
            weight_input("k20", 0.0), weight_input("k21", 0.0), weight_input("k22", 0.0),
            Input::new("divisor".to_string(), Value::Decimal(1.0), Some(InputSettings::DragValue { clamp: None, speed: Some(0.1) }), None)
                .with_description("Value the summed result is divided by (0 is treated as 1)."),
            Input::new("bias".to_string(), Value::Decimal(0.0), Some(InputSettings::DragValue { clamp: None, speed: Some(0.01) }), None)
                .with_description("Constant added to every channel after the division."),
        ]
    }

    /// Creates the output port: the convolved image.
    pub fn create_outputs() -> Vec<Output> {
        vec![
            Output::new("output".to_string(), Value::Image { data: default_image(), change_id: get_id() }, None)
                .with_description("Image filtered by the 3x3 kernel."),
        ]
    }

    /// Executes the convolution over the image.
    pub async fn run(inputs: &mut [Input]) -> Result<OperationResponse, OperationError> {
        let start_time = Instant::now();
        let mut input_errors: Vec<(usize, String)> = vec![];

        let image_converted = convert_input(inputs, 0, ValueType::Image, &mut input_errors);
        let mut kernel = [0.0f32; 9];
        for (i, slot) in kernel.iter_mut().enumerate() {
            if let Some(Value::Decimal(v)) = convert_input(inputs, i + 1, ValueType::Decimal, &mut input_errors) {
                *slot = v;
            }
        }
        let divisor_converted = convert_input(inputs, 10, ValueType::Decimal, &mut input_errors);
        let bias_converted = convert_input(inputs, 11, ValueType::Decimal, &mut input_errors);

        if !input_errors.is_empty() { return Err(OperationError { input_errors, node_error: None }); }

        let Value::Image { data, change_id: _ } = image_converted.unwrap() else { unreachable!() };
        let Value::Decimal(divisor) = divisor_converted.unwrap() else { unreachable!() };
        let Value::Decimal(bias) = bias_converted.unwrap() else { unreachable!() };

        let (w, h) = data.dimensions();
        let ch = data.channels() as usize;
        let color_ch = if ch == 2 || ch == 4 { ch - 1 } else { ch };
        let div = if divisor.abs() < 1e-9 { 1.0 } else { divisor };
        let wi = w as i32;
        let hi = h as i32;

        let mut out = FloatImage::new(w, h, data.channels());
        for y in 0..h {
            for x in 0..w {
                let mut acc = [0.0f32; 4];
                for ky in 0..3 {
                    for kx in 0..3 {
                        let k = kernel[ky * 3 + kx];
                        if k == 0.0 { continue; }
                        let sxp = (x as i32 + kx as i32 - 1).clamp(0, wi - 1) as u32;
                        let syp = (y as i32 + ky as i32 - 1).clamp(0, hi - 1) as u32;
                        let p = data.get_pixel(sxp, syp);
                        for c in 0..color_ch {
                            acc[c] += k * p[c];
                        }
                    }
                }
                let mut op = [0.0f32; 4];
                for c in 0..color_ch {
                    op[c] = acc[c] / div + bias;
                }
                // Preserve the alpha channel verbatim, if present.
                if color_ch < ch {
                    op[color_ch] = data.get_pixel(x, y)[color_ch];
                }
                out.put_pixel(x, y, &op[0..ch]);
            }
        }

        Ok(OperationResponse {
            time: Instant::now().duration_since(start_time),
            responses: vec![OutputResponse { value: Value::Image { data: Arc::new(out), change_id: get_id() } }],
        })
    }
}

#[cfg(test)]
#[path = "convolution_tests.rs"]
mod tests;
