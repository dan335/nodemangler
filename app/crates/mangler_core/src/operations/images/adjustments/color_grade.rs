//! Color Grade: three-way (shadows/midtones/highlights) hue tint + luminance,
//! Lightroom Color Grading / darktable color balance rgb style.
//!
//! Each tonal range gets its own hue+saturation tint and luminance offset.
//! Per-pixel weights across the three ranges come from a smooth split of the
//! luminance axis into two crossover bands, whose centre (`balance`) and
//! softness (`blending`) are user-controlled — mirroring how Lightroom's
//! Color Grading wheels and darktable's color balance rgb module let you
//! reshape where "shadows" end and "highlights" begin.

use crate::get_id;
use crate::value::ValueType;
use crate::input::{Input, InputSettings};
use crate::node_settings::NodeSettings;
use crate::operations::{OperationResponse, OperationError, OutputResponse, default_image, convert_input};
use super::common::{hsl_to_rgb, smoothstep};
use crate::output::Output;
use crate::value::Value;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use std::time::Instant;

const RANGE_NAMES: [&str; 3] = ["shadows", "midtones", "highlights"];

/// Three-way tonal color grading (shadows / midtones / highlights).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpImageAdjustmentColorGrade {}

impl OpImageAdjustmentColorGrade {
    /// Returns the node metadata (name, description, help).
    pub fn settings() -> NodeSettings {
        NodeSettings {
            name: "color grade".to_string(),
            description: "Three-way shadows/midtones/highlights hue tint and luminance, Lightroom Color Grading style.".to_string(),
            help: "Each of the three tonal ranges (shadows, midtones, highlights) has its own hue, saturation, and luminance sliders — the same shape as Lightroom's Color Grading panel or darktable's color balance rgb. Per pixel, the Rec.709 luminance picks smooth weights across the three ranges: a crossover point `pivot = 0.5 + balance * 0.25` splits low from high, softened by `soft = 0.15 + blending * 0.35`; shadows fade out below `pivot - 0.25` and highlights fade in above `pivot + 0.25`, with midtones filling whatever weight is left.\n\nFor each range, the hue is turned into a saturated tint colour and blended into the pixel scaled by that range's saturation slider and weight (`rgb += (tint - 0.5) * saturation * weight * 0.4`), then the luminance slider applies as a multiplicative gain `rgb *= 2^(luminance * weight * 0.8)`. Output is clamped to [0, 1]; alpha is untouched.\n\nAll three saturation and all three luminance sliders at 0 leaves the image byte-identical (hue and blending/balance have nothing to act on). Grayscale inputs (fewer than 3 channels) have no chroma to tint, so only the per-range luminance offsets apply.".to_string(),
        }
    }

    /// Creates the 12 inputs: image, 3 ranges x (hue, saturation,
    /// luminance), then blending and balance.
    pub fn create_inputs() -> Vec<Input> {
        let mut inputs = vec![
            Input::new("image".to_string(), Value::Image { data: default_image(), change_id: get_id() }, None, None)
                .with_description("Source image to color grade."),
        ];
        for name in RANGE_NAMES {
            inputs.push(
                Input::new(
                    format!("{name} hue"),
                    Value::Decimal(0.0),
                    Some(InputSettings::Slider { range: (0.0, 360.0), step_by: Some(1.0), clamp_to_range: true }),
                    None,
                )
                .with_description(format!("Tint hue in degrees for the {name} range.")),
            );
            inputs.push(
                Input::new(
                    format!("{name} saturation"),
                    Value::Decimal(0.0),
                    Some(InputSettings::Slider { range: (0.0, 1.0), step_by: Some(0.01), clamp_to_range: true }),
                    None,
                )
                .with_description(format!("Tint strength for the {name} range; 0 = no tint.")),
            );
            inputs.push(
                Input::new(
                    format!("{name} luminance"),
                    Value::Decimal(0.0),
                    Some(InputSettings::Slider { range: (-1.0, 1.0), step_by: Some(0.01), clamp_to_range: true }),
                    None,
                )
                .with_description(format!("Brightness offset for the {name} range.")),
            );
        }
        inputs.push(
            Input::new("blending".to_string(), Value::Decimal(0.5), Some(InputSettings::Slider { range: (0.0, 1.0), step_by: Some(0.01), clamp_to_range: true }), None)
                .with_description("Softness of the transition between tonal ranges."),
        );
        inputs.push(
            Input::new("balance".to_string(), Value::Decimal(0.0), Some(InputSettings::Slider { range: (-1.0, 1.0), step_by: Some(0.01), clamp_to_range: true }), None)
                .with_description("Shifts the shadows/highlights crossover point up or down the luminance range."),
        );
        inputs
    }

    /// Creates the output port: the graded image.
    pub fn create_outputs() -> Vec<Output> {
        vec![
            Output::new("output".to_string(), Value::Image { data: default_image(), change_id: get_id() }, None)
                .with_description("Image with the three-way tonal color grade applied."),
        ]
    }

    /// Executes the three-way color grade.
    pub async fn run(inputs: &mut [Input]) -> Result<OperationResponse, OperationError> {
        let start_time = Instant::now();
        let mut input_errors: Vec<(usize, String)> = vec![];

        let image_converted = convert_input(inputs, 0, ValueType::Image, &mut input_errors);
        let mut hue = [0.0f32; 3];
        let mut sat = [0.0f32; 3];
        let mut lum = [0.0f32; 3];
        for i in 0..3 {
            let base = 1 + i * 3;
            if let Some(Value::Decimal(v)) = convert_input(inputs, base, ValueType::Decimal, &mut input_errors) {
                hue[i] = v as f32;
            }
            if let Some(Value::Decimal(v)) = convert_input(inputs, base + 1, ValueType::Decimal, &mut input_errors) {
                sat[i] = v as f32;
            }
            if let Some(Value::Decimal(v)) = convert_input(inputs, base + 2, ValueType::Decimal, &mut input_errors) {
                lum[i] = v as f32;
            }
        }
        let blending_converted = convert_input(inputs, 10, ValueType::Decimal, &mut input_errors);
        let balance_converted = convert_input(inputs, 11, ValueType::Decimal, &mut input_errors);

        if !input_errors.is_empty() { return Err(OperationError { input_errors, node_error: None }); }

        let Value::Image { data, change_id: _ } = image_converted.unwrap() else { unreachable!() };
        let Value::Decimal(blending) = blending_converted.unwrap() else { unreachable!() };
        let Value::Decimal(balance) = balance_converted.unwrap() else { unreachable!() };
        let blending = blending as f32;
        let balance = balance as f32;

        let all_neutral = sat.iter().all(|v| *v == 0.0) && lum.iter().all(|v| *v == 0.0);
        if all_neutral {
            return Ok(OperationResponse {
                time: Instant::now().duration_since(start_time),
                responses: vec![OutputResponse { value: Value::Image { data, change_id: get_id() } }],
            });
        }

        let ch = data.channels() as usize;
        let color = ch >= 3;

        // Fully-saturated, mid-lightness tint colours for the three ranges'
        // hues, precomputed once outside the pixel loop.
        let tints: [(f32, f32, f32); 3] = if color {
            [
                hsl_to_rgb(hue[0], 1.0, 0.5),
                hsl_to_rgb(hue[1], 1.0, 0.5),
                hsl_to_rgb(hue[2], 1.0, 0.5),
            ]
        } else {
            [(0.5, 0.5, 0.5); 3]
        };

        let pivot = 0.5 + balance * 0.25;
        let soft = 0.15 + blending * 0.35;
        let pivot_lo = pivot - 0.25;
        let pivot_hi = pivot + 0.25;

        let mut result = (*data).clone();
        for pixel in result.pixels_mut() {
            let l = if color {
                0.2126 * pixel[0] + 0.7152 * pixel[1] + 0.0722 * pixel[2]
            } else {
                pixel[0]
            };

            let w_sh = 1.0 - smoothstep(pivot_lo - soft, pivot_lo + soft, l);
            let w_hi = smoothstep(pivot_hi - soft, pivot_hi + soft, l);
            let w_mid = (1.0 - w_sh - w_hi).max(0.0);
            let weights = [w_sh, w_mid, w_hi];

            if color {
                let mut rgb = [pixel[0], pixel[1], pixel[2]];
                for r in 0..3 {
                    let w = weights[r];
                    if w <= 0.0 {
                        continue;
                    }
                    let (tr, tg, tb) = tints[r];
                    rgb[0] += (tr - 0.5) * sat[r] * w * 0.4;
                    rgb[1] += (tg - 0.5) * sat[r] * w * 0.4;
                    rgb[2] += (tb - 0.5) * sat[r] * w * 0.4;
                    let factor = 2f32.powf(lum[r] * w * 0.8);
                    rgb[0] *= factor;
                    rgb[1] *= factor;
                    rgb[2] *= factor;
                }
                pixel[0] = rgb[0].clamp(0.0, 1.0);
                pixel[1] = rgb[1].clamp(0.0, 1.0);
                pixel[2] = rgb[2].clamp(0.0, 1.0);
                // Alpha (channel 3 on 4-channel images) is left untouched.
            } else {
                // Grayscale: no chroma to tint, only luminance offsets apply.
                let mut v = pixel[0];
                for r in 0..3 {
                    let w = weights[r];
                    if w <= 0.0 {
                        continue;
                    }
                    v *= 2f32.powf(lum[r] * w * 0.8);
                }
                pixel[0] = v.clamp(0.0, 1.0);
            }
        }

        Ok(OperationResponse {
            time: Instant::now().duration_since(start_time),
            responses: vec![OutputResponse { value: Value::Image { data: Arc::new(result), change_id: get_id() } }],
        })
    }
}

#[cfg(test)]
#[path = "color_grade_tests.rs"]
mod tests;
