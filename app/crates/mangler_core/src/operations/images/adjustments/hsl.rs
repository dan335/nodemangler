//! Image-wide HSL adjustment.
//!
//! Applies hue, saturation, and lightness deltas to every RGB pixel by
//! converting to HSL, adjusting, and converting back. Unlike `hue shift`
//! (hue only) and the single-color `adjust hsv` node, this operates on
//! entire images and covers all three HSL components at once.

use crate::color::Color;
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

/// Image HSL adjustment operation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpImageAdjustmentHsl {}

impl OpImageAdjustmentHsl {
    pub fn settings() -> NodeSettings {
        NodeSettings {
            name: "hsl".to_string(),
            description: "Shifts hue, scales saturation, and offsets lightness across the whole image.".to_string(),
            help: "For every RGB pixel, converts to HSL, applies `hue += hue_shift*360°` (wrapping), `saturation *= saturation`, `lightness += lightness`, clamps saturation/lightness to [0, 1], and converts back. All three adjustments are applied in one pass, so this is cheaper than chaining hue-rotate with separate saturation and lightness nodes.\n\n1- and 2-channel (grayscale) inputs have no hue or saturation; lightness is mapped onto their intensity channel instead. Alpha (when present) is preserved unchanged.".to_string(),
        }
    }

    pub fn create_inputs() -> Vec<Input> {
        vec![
            Input::new("image".to_string(), Value::Image { data: default_image(), change_id: get_id() }, None, None)
                .with_description("Source image whose HSL components will be adjusted."),
            Input::new("hue".to_string(), Value::Decimal(0.0), Some(InputSettings::Slider { range: (-1.0, 1.0), step_by: Some(0.001), clamp_to_range: false }), None)
                .with_description("Hue rotation normalised to [-1, 1], mapped to -360° to +360°."),
            Input::new("saturation".to_string(), Value::Decimal(1.0), Some(InputSettings::Slider { range: (0.0, 2.0), step_by: Some(0.01), clamp_to_range: false }), None)
                .with_description("Saturation multiplier; 1 = unchanged, 0 = desaturated, >1 = boosted."),
            Input::new("lightness".to_string(), Value::Decimal(0.0), Some(InputSettings::Slider { range: (-1.0, 1.0), step_by: Some(0.01), clamp_to_range: false }), None)
                .with_description("Lightness offset added after saturation; clamped to [0, 1]."),
        ]
    }

    pub fn create_outputs() -> Vec<Output> {
        vec![
            Output::new("output".to_string(), Value::Image { data: default_image(), change_id: get_id() }, None)
                .with_description("Image with the HSL adjustments applied."),
        ]
    }

    pub async fn run(inputs: &mut [Input]) -> Result<OperationResponse, OperationError> {
        let start_time = Instant::now();
        let mut input_errors: Vec<(usize, String)> = vec![];

        let image_converted = convert_input(inputs, 0, ValueType::Image, &mut input_errors);
        let hue_converted = convert_input(inputs, 1, ValueType::Decimal, &mut input_errors);
        let sat_converted = convert_input(inputs, 2, ValueType::Decimal, &mut input_errors);
        let light_converted = convert_input(inputs, 3, ValueType::Decimal, &mut input_errors);

        if !input_errors.is_empty() { return Err(OperationError { input_errors, node_error: None }); }

        let Value::Image { data, change_id: _ } = image_converted.unwrap() else { unreachable!() };
        let Value::Decimal(hue_shift) = hue_converted.unwrap() else { unreachable!() };
        let Value::Decimal(sat_mult) = sat_converted.unwrap() else { unreachable!() };
        let Value::Decimal(light_shift) = light_converted.unwrap() else { unreachable!() };

        let degrees = hue_shift * 360.0;
        let (w, h) = data.dimensions();
        let ch = data.channels() as usize;
        let mut output = FloatImage::new(w, h, ch as u32);

        let mut buf = [0.0f32; 4];
        for y in 0..h {
            for x in 0..w {
                let src = data.get_pixel(x, y);
                if ch >= 3 {
                    // Go through HSL: adjust, clamp, come back.
                    let c = Color { r: src[0], g: src[1], b: src[2], a: 1.0 };
                    let (mut hue, sat, light, _) = c.to_hsl();
                    hue = (hue + degrees).rem_euclid(360.0);
                    let sat = (sat * sat_mult).clamp(0.0, 1.0);
                    let light = (light + light_shift).clamp(0.0, 1.0);
                    let out_color = Color::from_hsl(hue, sat, light, 1.0);
                    buf[0] = out_color.r;
                    buf[1] = out_color.g;
                    buf[2] = out_color.b;
                    if ch == 4 { buf[3] = src[3]; }
                } else {
                    // Grayscale: only lightness shift makes sense; hue/sat are undefined.
                    buf[0] = (src[0] + light_shift).clamp(0.0, 1.0);
                    if ch == 2 { buf[1] = src[1]; }
                }
                output.put_pixel(x, y, &buf[..ch]);
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

#[cfg(test)]
#[path = "hsl_tests.rs"]
mod tests;
