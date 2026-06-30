//! Color balance: independent RGB offsets for shadows, midtones, and highlights.
//!
//! Each pixel's luminance selects a smooth weighting across three tonal bands;
//! the band offsets are summed and added to the colour channels. Grayscale
//! inputs pass through unchanged.

use crate::get_id;
use crate::value::ValueType;
use crate::input::{Input, InputSettings};
use crate::node_settings::NodeSettings;
use crate::operations::{OperationResponse, OperationError, OutputResponse, default_image, convert_input};
use crate::output::Output;
use crate::value::Value;
use super::common::smoothstep;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use std::time::Instant;

/// Maximum channel offset contributed by a band slider at ±1.
const SCALE: f32 = 0.5;

/// A drag-value slider helper for the nine band offsets.
fn offset_input(name: &str, desc: &str) -> Input {
    Input::new(
        name.to_string(),
        Value::Decimal(0.0),
        Some(InputSettings::Slider { range: (-1.0, 1.0), step_by: Some(0.01), clamp_to_range: true }),
        None,
    )
    .with_description(desc.to_string())
}

/// Three-band (shadows / midtones / highlights) RGB colour balance.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpImageAdjustmentColorBalance {}

impl OpImageAdjustmentColorBalance {
    /// Returns the node metadata (name and description) for color balance.
    pub fn settings() -> NodeSettings {
        NodeSettings {
            name: "color balance".to_string(),
            description: "Shifts R/G/B independently in shadows, midtones, and highlights.".to_string(),
            help: "Each pixel's Rec. 709 luminance is split into three smooth tonal weights: shadows (1 at black, fading out by mid-gray), highlights (fading in from mid-gray to white), and midtones (the remainder). The per-band RGB sliders are weighted by those bands and summed, then added to the colour channels scaled by 0.5.\n\nThis lets you, for example, push shadows toward blue while warming highlights. Sliders are normalized to [-1, 1] and the result is not clamped. Grayscale inputs (1 or 2 channels) have no colour to balance and pass through unchanged; alpha is preserved.".to_string(),
        }
    }

    /// Creates input ports: image plus nine per-band RGB offsets.
    pub fn create_inputs() -> Vec<Input> {
        vec![
            Input::new("image".to_string(), Value::Image { data: default_image(), change_id: get_id() }, None, None)
                .with_description("Source colour image to balance."),
            offset_input("shadows r", "Red offset applied to dark tones."),
            offset_input("shadows g", "Green offset applied to dark tones."),
            offset_input("shadows b", "Blue offset applied to dark tones."),
            offset_input("midtones r", "Red offset applied to mid tones."),
            offset_input("midtones g", "Green offset applied to mid tones."),
            offset_input("midtones b", "Blue offset applied to mid tones."),
            offset_input("highlights r", "Red offset applied to bright tones."),
            offset_input("highlights g", "Green offset applied to bright tones."),
            offset_input("highlights b", "Blue offset applied to bright tones."),
        ]
    }

    /// Creates the output port: the colour-balanced image.
    pub fn create_outputs() -> Vec<Output> {
        vec![
            Output::new("output".to_string(), Value::Image { data: default_image(), change_id: get_id() }, None)
                .with_description("Image with per-band RGB offsets applied."),
        ]
    }

    /// Executes the colour balance using per-pixel tonal weighting.
    pub async fn run(inputs: &mut [Input]) -> Result<OperationResponse, OperationError> {
        let start_time = Instant::now();
        let mut input_errors: Vec<(usize, String)> = vec![];

        let image_converted = convert_input(inputs, 0, ValueType::Image, &mut input_errors);
        let mut bands = [0.0f32; 9];
        for (i, slot) in bands.iter_mut().enumerate() {
            if let Some(Value::Decimal(v)) = convert_input(inputs, i + 1, ValueType::Decimal, &mut input_errors) {
                *slot = v;
            }
        }

        if !input_errors.is_empty() { return Err(OperationError { input_errors, node_error: None }); }

        let Value::Image { data, change_id: _ } = image_converted.unwrap() else { unreachable!() };

        let ch = data.channels() as usize;
        if ch < 3 {
            // Grayscale: nothing to balance.
            return Ok(OperationResponse {
                time: Instant::now().duration_since(start_time),
                responses: vec![OutputResponse { value: Value::Image { data, change_id: get_id() } }],
            });
        }

        let sh = [bands[0], bands[1], bands[2]];
        let mid = [bands[3], bands[4], bands[5]];
        let hi = [bands[6], bands[7], bands[8]];

        let mut result = (*data).clone();
        for pixel in result.pixels_mut() {
            let luma = 0.2126 * pixel[0] + 0.7152 * pixel[1] + 0.0722 * pixel[2];
            let w_sh = 1.0 - smoothstep(0.0, 0.5, luma);
            let w_hi = smoothstep(0.5, 1.0, luma);
            let w_mid = (1.0 - w_sh - w_hi).max(0.0);
            for c in 0..3 {
                pixel[c] += SCALE * (w_sh * sh[c] + w_mid * mid[c] + w_hi * hi[c]);
            }
        }

        Ok(OperationResponse {
            time: Instant::now().duration_since(start_time),
            responses: vec![OutputResponse { value: Value::Image { data: Arc::new(result), change_id: get_id() } }],
        })
    }
}

#[cfg(test)]
#[path = "color_balance_tests.rs"]
mod tests;
