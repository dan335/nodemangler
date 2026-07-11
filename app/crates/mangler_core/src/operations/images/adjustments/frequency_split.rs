//! Frequency separation: splits an image into low- and high-frequency halves.
//!
//! Gaussian-blurs the input to produce the low-frequency component; the
//! complementary high-frequency component is `source − low + 0.5`, biased so
//! that zero detail sits at mid-grey. Recombining via `low + high − 0.5`
//! reconstructs the original exactly (up to clamping). Standard tool for
//! height-map authoring and retouching pipelines.

use crate::float_image::FloatImage;
use crate::get_id;
use crate::input::{Input, InputSettings};
use crate::node_settings::NodeSettings;
use crate::operations::images::blur::blur::gaussian_blur_image;
use crate::operations::{OperationResponse, OperationError, OutputResponse, default_image, convert_input, scale_to_resolution};
use crate::output::Output;
use crate::value::{Value, ValueType};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use std::time::Instant;

/// Splits an image into low-frequency and high-frequency components.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpImageAdjustmentFrequencySplit {}

impl OpImageAdjustmentFrequencySplit {
    pub fn settings() -> NodeSettings {
        NodeSettings {
            name: "frequency split".to_string(),
            description: "Splits the source into low-frequency (blurred) and high-frequency (detail) components.".to_string(),
            help: "Gaussian-blurs the source at the given sigma to isolate the low-frequency component (output 0); the high-frequency component (output 1) is `source − low + 0.5`, biased to mid-grey so downstream blends stay neutral at zero difference. Reconstructing via `low + (high − 0.5)` recovers the original up to clamping.\n\nTypical workflow: tweak tone and base shape on the low output, tweak details and micro-texture on the high output, then recombine with the arithmetic above (or `blend` in linear-add mode on the shifted high). Alpha, when present, passes through both outputs unchanged. Sigma 0 returns the source and a mid-grey detail image.".to_string(),
        }
    }

    pub fn create_inputs() -> Vec<Input> {
        vec![
            Input::new("image".to_string(), Value::Image { data: default_image(), change_id: get_id() }, None, None)
                .with_description("Source image to decompose."),
            Input::new("sigma".to_string(), Value::Decimal(8.0), Some(InputSettings::DragValue { speed: None, clamp: Some((0.0, 1000.0)) }), None)
                .with_description("Gaussian blur sigma in pixels at a 1024px reference (scales with image size); larger values put more detail in the high output."),
        ]
    }

    pub fn create_outputs() -> Vec<Output> {
        vec![
            Output::new("low".to_string(), Value::Image { data: default_image(), change_id: get_id() }, None)
                .with_description("Low-frequency (blurred) component of the source."),
            Output::new("high".to_string(), Value::Image { data: default_image(), change_id: get_id() }, None)
                .with_description("High-frequency (detail) component, biased so zero detail sits at mid-grey."),
        ]
    }

    pub async fn run(inputs: &mut [Input]) -> Result<OperationResponse, OperationError> {
        let start_time = Instant::now();
        let mut input_errors: Vec<(usize, String)> = vec![];

        let image_converted = convert_input(inputs, 0, ValueType::Image, &mut input_errors);
        let sigma_converted = convert_input(inputs, 1, ValueType::Decimal, &mut input_errors);

        if !input_errors.is_empty() { return Err(OperationError { input_errors, node_error: None }); }

        let Value::Image { data, change_id: _ } = image_converted.unwrap() else { unreachable!() };
        let Value::Decimal(sigma) = sigma_converted.unwrap() else { unreachable!() };

        let (w, h) = data.dimensions();
        let ch = data.channels() as usize;
        let has_alpha = ch == 2 || ch == 4;
        let color_ch = if has_alpha { ch - 1 } else { ch };

        // Sigma is authored in reference pixels (at 1024px) and scaled to the
        // actual image, so the low/high split looks the same at any resolution.
        let sigma = scale_to_resolution(sigma.max(0.0), w, h);

        // Low-frequency = gaussian-blurred source. Reuse the shared helper so we
        // match the standard blur operator exactly.
        let mut low = gaussian_blur_image(&data, sigma);

        // Alpha is not a frequency band; the blur smears it, but the help
        // promises alpha passes through both outputs unchanged (and the high
        // output already copies source alpha). Restore the source alpha on the
        // low output so the two stay consistent and recombine cleanly.
        if has_alpha {
            for y in 0..h {
                for x in 0..w {
                    let a = data.get_pixel(x, y)[ch - 1];
                    low.get_pixel_mut(x, y)[ch - 1] = a;
                }
            }
        }

        // High-frequency = source − low + 0.5 on colour channels. Alpha is
        // copied straight through so a stack of high/low layers still composites
        // correctly.
        let mut high = FloatImage::new(w, h, ch as u32);
        let mut buf = [0.0f32; 4];
        for y in 0..h {
            for x in 0..w {
                let src = data.get_pixel(x, y);
                let blur = low.get_pixel(x, y);
                for c in 0..color_ch {
                    buf[c] = (src[c] - blur[c] + 0.5).clamp(0.0, 1.0);
                }
                if has_alpha {
                    buf[ch - 1] = src[ch - 1];
                }
                high.put_pixel(x, y, &buf[..ch]);
            }
        }

        Ok(OperationResponse {
            time: Instant::now().duration_since(start_time),
            responses: vec![
                OutputResponse { value: Value::Image { data: Arc::new(low), change_id: get_id() } },
                OutputResponse { value: Value::Image { data: Arc::new(high), change_id: get_id() } },
            ],
        })
    }
}

#[cfg(test)]
#[path = "frequency_split_tests.rs"]
mod tests;
