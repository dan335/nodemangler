//! Tone mapping operation for images.
//!
//! Compresses (typically high-dynamic-range, i.e. values that may exceed 1.0
//! after exposure) pixel values into the displayable `[0, 1]` range using one
//! of several well-known tone mapping curves. Each non-alpha channel value `v`
//! is first scaled by `2^exposure` (photographic stops), then passed through
//! the selected operator, then clamped to `[0, 1]`. Alpha passes through
//! untouched.

use crate::get_id;
use crate::value::{ToneMapOperator, ValueType};
use crate::input::{Input, InputSettings};
use crate::node_settings::NodeSettings;
use crate::operations::{OperationResponse, OperationError, OutputResponse, default_image, convert_input};
use crate::output::Output;
use crate::value::Value;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use std::time::Instant;

/// Log-domain sigmoid contrast constant (see [`apply_sigmoid`]). Tuned so the
/// default output stays close to identity around mid-gray (0.18).
const SIGMOID_CONTRAST: f32 = 1.6;

/// Mid-gray reference used by the sigmoid operator, matching darktable's
/// sigmoid module convention.
const SIGMOID_MID_GRAY: f32 = 0.18;

/// Uncharted 2 filmic curve constants (Hable 2010).
const HABLE_A: f32 = 0.15;
const HABLE_B: f32 = 0.50;
const HABLE_C: f32 = 0.10;
const HABLE_D: f32 = 0.20;
const HABLE_E: f32 = 0.02;
const HABLE_F: f32 = 0.30;

/// Tone mapping: compresses HDR pixel values into `[0, 1]` using a selectable
/// operator (Reinhard, Reinhard Extended, ACES, Hable Filmic, or a heuristic
/// sigmoid), with an exposure pre-scale and white-point normalization.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpImageAdjustmentToneMap {}

impl OpImageAdjustmentToneMap {
    /// Returns the node metadata (name, description, and help) for the tone map operation.
    pub fn settings() -> NodeSettings {
        NodeSettings {
            name: "tone map".to_string(),
            description: "Compress HDR values into displayable range using a selectable tone-mapping curve.".to_string(),
            help: "Compresses (typically high-dynamic-range) pixel values into the displayable \
                   0-1 range. Each non-alpha channel value v is first scaled by 2^exposure \
                   (photographic stops), then passed through the selected operator, then clamped \
                   to [0, 1]. Alpha is always preserved.\n\n\
                   Operators:\n\n\
                   - Reinhard — the simple global operator from Reinhard et al. 2002 \
                   (\"Photographic Tone Reproduction for Digital Images\"): v / (1 + v).\n\n\
                   - Reinhard Extended — the same paper's extended form, which adds a white \
                   point Lwhite so v = Lwhite maps back to (approximately) 1.0 instead of \
                   asymptoting forever: v * (1 + v / white^2) / (1 + v).\n\n\
                   - ACES — Krzysztof Narkowicz's 2015 fast analytic fit to the ACES filmic \
                   reference tonemapper (\"ACES Filmic Tone Mapping Curve\"), a rational \
                   polynomial approximation widely used in real-time rendering.\n\n\
                   - Hable Filmic — John Hable's Uncharted 2 filmic curve (GDC 2010, \"Uncharted \
                   2: HDR Lighting\"), a piecewise-rational curve with shoulder/toe shaping, \
                   normalized by its own value at the white point so white maps to 1.0.\n\n\
                   - Sigmoid — a heuristic log-domain logistic curve centered on 0.18 mid-gray, \
                   inspired by darktable's sigmoid module (not a published reference operator; \
                   it is a smooth, easily-invertible S-curve tuned to look neutral near default \
                   settings).\n\n\
                   1-4 channel images are supported; grayscale/color channels are tone mapped, \
                   alpha (if present) passes through unchanged.".to_string(),
        }
    }

    /// Creates the input ports: the source image, operator, exposure (stops), and white point.
    pub fn create_inputs() -> Vec<Input> {
        vec![
            Input::new("image".to_string(), Value::Image { data: default_image(), change_id: get_id() }, None, None)
                .with_description("Source image to tone map."),
            Input::new("operator".to_string(), Value::ToneMapOperator(ToneMapOperator::Reinhard), None, None)
                .with_description("Tone mapping curve to apply."),
            Input::new("exposure".to_string(), Value::Decimal(0.0), Some(InputSettings::Slider { range: (-5.0, 5.0), step_by: Some(0.01), clamp_to_range: false }), None)
                .with_description("Exposure in stops, applied as a 2^exposure multiplier before tone mapping."),
            Input::new("white point".to_string(), Value::Decimal(4.0), Some(InputSettings::Slider { range: (0.5, 16.0), step_by: Some(0.01), clamp_to_range: true }), None)
                .with_description("Value that maps back to ~1.0. Used by Reinhard Extended and Hable Filmic normalization."),
        ]
    }

    /// Creates the output port: the tone-mapped image.
    pub fn create_outputs() -> Vec<Output> {
        vec![
            Output::new("output".to_string(), Value::Image { data: default_image(), change_id: get_id() }, None)
                .with_description("Tone-mapped image, clamped to [0, 1]; alpha preserved."),
        ]
    }

    /// Executes the tone map operation: exposure pre-scale, then the selected operator, then clamp.
    pub async fn run(inputs: &mut [Input]) -> Result<OperationResponse, OperationError> {
        let start_time = Instant::now();
        let mut input_errors: Vec<(usize, String)> = vec![];

        // convert inputs
        let image_converted      = convert_input(inputs, 0, ValueType::Image, &mut input_errors);
        let operator_converted   = convert_input(inputs, 1, ValueType::ToneMapOperator, &mut input_errors);
        let exposure_converted   = convert_input(inputs, 2, ValueType::Decimal, &mut input_errors);
        let white_point_converted = convert_input(inputs, 3, ValueType::Decimal, &mut input_errors);

        // return if error
        if !input_errors.is_empty() { return Err(OperationError { input_errors, node_error: None }); }

        // get values
        let Value::Image { data, change_id: _ } = image_converted.unwrap() else { unreachable!() };
        let Value::ToneMapOperator(operator) = operator_converted.unwrap() else { unreachable!() };
        let Value::Decimal(exposure) = exposure_converted.unwrap() else { unreachable!() };
        let Value::Decimal(white_point) = white_point_converted.unwrap() else { unreachable!() };

        let exposure_gain = 2f32.powf(exposure);
        // Guard against a degenerate white point (slider clamps to >= 0.5 anyway).
        let white_point = white_point.max(1e-3);
        // Precompute the Hable normalization denominator once per run.
        let hable_norm = hable_filmic_curve(white_point);

        let mut result = (*data).clone();
        let ch = result.channels() as usize;
        // Determine how many color channels to tone map (skip alpha if present)
        let color_ch = if ch == 2 || ch == 4 { ch - 1 } else { ch };

        for pixel in result.pixels_mut() {
            for val in pixel.iter_mut().take(color_ch) {
                let v = *val * exposure_gain;
                let mapped = match operator {
                    ToneMapOperator::Reinhard => reinhard(v),
                    ToneMapOperator::ReinhardExtended => reinhard_extended(v, white_point),
                    ToneMapOperator::Aces => aces(v),
                    ToneMapOperator::HableFilmic => {
                        if hable_norm.abs() < 1e-6 { 0.0 } else { hable_filmic_curve(v) / hable_norm }
                    }
                    ToneMapOperator::Sigmoid => sigmoid(v),
                };
                *val = mapped.clamp(0.0, 1.0);
            }
        }

        Ok(OperationResponse {
            time: Instant::now().duration_since(start_time),
            responses: vec![
                OutputResponse { value: Value::Image { data: Arc::new(result), change_id: get_id() } },
            ],
        })
    }
}

/// Simple Reinhard operator (Reinhard et al. 2002): `v / (1 + v)`.
fn reinhard(v: f32) -> f32 {
    v / (1.0 + v.max(0.0))
}

/// Reinhard extended with a white point: highlights at `white` map back to
/// (approximately) 1.0 instead of asymptoting forever.
fn reinhard_extended(v: f32, white: f32) -> f32 {
    let v = v.max(0.0);
    let w2 = (white * white).max(1e-6);
    v * (1.0 + v / w2) / (1.0 + v)
}

/// Narkowicz 2015 fast analytic fit to the ACES filmic reference tonemapper.
fn aces(v: f32) -> f32 {
    let v = v.max(0.0);
    let a = 2.51;
    let b = 0.03;
    let c = 2.43;
    let d = 0.59;
    let e = 0.14;
    (v * (a * v + b)) / (v * (c * v + d) + e)
}

/// Hable's Uncharted 2 filmic curve (unnormalized). Callers divide by
/// `hable_filmic_curve(white_point)` to bring white back to ~1.0.
fn hable_filmic_curve(x: f32) -> f32 {
    let x = x.max(0.0);
    ((x * (HABLE_A * x + HABLE_C * HABLE_B) + HABLE_D * HABLE_E)
        / (x * (HABLE_A * x + HABLE_B) + HABLE_D * HABLE_F))
        - HABLE_E / HABLE_F
}

/// Heuristic log-domain sigmoid inspired by darktable's sigmoid module: a
/// logistic curve in `v / mid_gray`, tuned around 0.18 mid-gray so the
/// default contrast looks close to neutral.
fn sigmoid(v: f32) -> f32 {
    let ratio = (v.max(1e-6) / SIGMOID_MID_GRAY).max(1e-6);
    1.0 / (1.0 + ratio.powf(-SIGMOID_CONTRAST))
}

#[cfg(test)]
#[path = "tone_map_tests.rs"]
mod tests;
