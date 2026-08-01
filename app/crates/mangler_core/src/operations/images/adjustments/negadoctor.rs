//! Negadoctor: simplified film-negative inversion, inspired by darktable's
//! `negadoctor` module.
//!
//! A colour film negative's orange mask (the `film base`) sits over the
//! actual image data; inverting it is not a simple `1 - x` because the mask
//! tints every channel differently. This node approximates the published
//! negadoctor formula: each channel is divided by its film-base component,
//! raised to a `dynamic range` exponent, and scaled by a `brightness` gain,
//! then offset so a pixel exactly matching the film base lands at black.
//!
//! Not a calibrated film model — just a heuristic invert with a
//! believable, tunable response curve.

use crate::color::Color;
use crate::get_id;
use crate::value::ValueType;
use crate::input::{Input, InputSettings};
use crate::node_settings::NodeSettings;
use crate::operations::{OperationResponse, OperationError, OutputResponse, default_image, convert_input};
use crate::output::Output;
use crate::value::Value;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use std::time::Instant;

/// Small epsilon to keep the base/input ratio finite near zero.
const EPS: f32 = 1e-4;

/// Simplified film-negative inversion operation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpImageAdjustmentNegadoctor {}

impl OpImageAdjustmentNegadoctor {
    /// Returns the node metadata (name, description, help).
    pub fn settings() -> NodeSettings {
        NodeSettings {
            name: "negadoctor".to_string(),
            description: "Simplified film-negative inversion inspired by darktable's negadoctor; not a calibrated film model.".to_string(),
            help: "Inverts a scanned colour negative. For each colour channel `c`, `out_c = clamp(((base_c + eps) / (in_c + eps))^dynamic_range * 2^brightness - 1, 0, 1)`, where `base_c` is the corresponding channel of `film base` (the orange mask colour typical of C-41 negatives). A pixel that exactly matches the film base maps to black (out = 0); darker (denser) negative pixels — which correspond to brighter parts of the original scene — map toward white. The mapping is monotonically decreasing in `in_c`.\n\n`dynamic range` is the inversion's gamma/contrast exponent (higher = punchier); `brightness` is an overall exposure-style gain applied before the black-point offset. Grayscale inputs use the film base's Rec.709 luma as `base_c`. Alpha is untouched. This is a simplified, uncalibrated approximation of darktable's negadoctor module, not a physically accurate film model.".to_string(),
        }
    }

    /// Creates the input ports: image, film base colour, dynamic range, brightness.
    pub fn create_inputs() -> Vec<Input> {
        vec![
            Input::new("image".to_string(), Value::Image { data: default_image(), change_id: get_id() }, None, None)
                .with_description("Scanned negative image to invert."),
            // Default = a typical C-41 orange mask colour.
            Input::new("film base".to_string(), Value::Color(Color { r: 1.0, g: 0.55, b: 0.32, a: 1.0 }), None, None)
                .with_description("Colour of the negative's film-base (orange mask); maps to black in the inverted output."),
            Input::new("dynamic range".to_string(), Value::Decimal(1.5), Some(InputSettings::Slider { range: (0.5, 4.0), step_by: Some(0.01), clamp_to_range: true }), None)
                .with_description("Gamma/contrast exponent applied to the base/input ratio; higher values punch up contrast."),
            Input::new("brightness".to_string(), Value::Decimal(0.0), Some(InputSettings::Slider { range: (-1.0, 1.0), step_by: Some(0.01), clamp_to_range: true }), None)
                .with_description("Overall exposure gain applied before the black-point offset."),
        ]
    }

    /// Creates the output port: the inverted (positive) image.
    pub fn create_outputs() -> Vec<Output> {
        vec![
            Output::new("output".to_string(), Value::Image { data: default_image(), change_id: get_id() }, None)
                .with_description("Inverted positive image, alpha preserved."),
        ]
    }

    /// Executes the negative inversion.
    pub async fn run(inputs: &mut [Input]) -> Result<OperationResponse, OperationError> {
        let start_time = Instant::now();
        let mut input_errors: Vec<(usize, String)> = vec![];

        let image_converted = convert_input(inputs, 0, ValueType::Image, &mut input_errors);
        let base_converted = convert_input(inputs, 1, ValueType::Color, &mut input_errors);
        let range_converted = convert_input(inputs, 2, ValueType::Decimal, &mut input_errors);
        let brightness_converted = convert_input(inputs, 3, ValueType::Decimal, &mut input_errors);

        if !input_errors.is_empty() { return Err(OperationError { input_errors, node_error: None }); }

        let Value::Image { data, change_id: _ } = image_converted.unwrap() else { unreachable!() };
        let Value::Color(base) = base_converted.unwrap() else { unreachable!() };
        let Value::Decimal(dynamic_range) = range_converted.unwrap() else { unreachable!() };
        let Value::Decimal(brightness) = brightness_converted.unwrap() else { unreachable!() };

        let gamma = dynamic_range as f32;
        let gain = 2f32.powf(brightness as f32);

        let base_luma = 0.2126 * base.r + 0.7152 * base.g + 0.0722 * base.b;

        let mut result = (*data).clone();
        let ch = result.channels() as usize;
        let color_ch = if ch >= 3 { if ch == 4 { 3 } else { ch } } else { 1 };
        let base_components = [base.r, base.g, base.b];

        for pixel in result.pixels_mut() {
            for c in 0..color_ch {
                let base_c = if ch >= 3 { base_components[c] } else { base_luma };
                let in_c = pixel[c];
                let ratio = (base_c + EPS) / (in_c + EPS);
                let out_c = (ratio.powf(gamma) * gain - 1.0).clamp(0.0, 1.0);
                pixel[c] = out_c;
            }
            // Alpha (last channel on 2/4-channel images) is left untouched.
        }

        Ok(OperationResponse {
            time: Instant::now().duration_since(start_time),
            responses: vec![OutputResponse { value: Value::Image { data: Arc::new(result), change_id: get_id() } }],
        })
    }
}

#[cfg(test)]
#[path = "negadoctor_tests.rs"]
mod tests;
