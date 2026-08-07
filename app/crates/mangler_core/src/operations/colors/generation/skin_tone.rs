//! Skin tone generation via the Inclusive Color Space.
//!
//! Implements Toney Alexander's Inclusive Color Space for producing diverse,
//! plausible human skin tones. Points live in a TUV sphere that maps to sRGB
//! through a fitted PCA transform; see
//! <https://toneyalexander.github.io/inclusive-color-space/>.
//!
//! Two ways to drive the node:
//! - **Random (default):** uniformly sample the sphere of radius √R² from a seed
//! - **Manual:** set T (deep/fair), U (flushed/ochre), V (cool/warm) directly

use crate::color::Color;
use crate::input::{Input, InputSettings};
use crate::node_settings::NodeSettings;
use crate::operations::{OperationResponse, OperationError, OutputResponse, convert_input};
use crate::output::Output;
use crate::value::{Value, ValueType};
use serde::{Deserialize, Serialize};
use std::f32::consts::TAU;
use std::time::Instant;

/// Map Inclusive Color Space TUV coordinates to sRGB floats in `[0, 1]`.
///
/// Equations from Toney Alexander's Inclusive Color Space — PCA-space
/// intermediate (x, y, z) then the RGB matrix, with channels originally in
/// 0–255 space. Out-of-sphere / high-R² extremes are clamped to the unit cube.
pub(crate) fn tuv_to_srgb(t: f32, u: f32, v: f32) -> (f32, f32, f32) {
    let x = (t - 0.15) / 0.45;
    let y = (v - 1.2 * t * t + 0.2 * t + 0.655) / 1.84;
    let z = u / 3.6;

    let r = 28.774_383_708_54 * x + 36.783_074_455_59 * y - 19.697_669_186_44 * z + 187.143_624_161_1;
    let g = 35.383_273_063_18 * x - 2.009_931_981_182 * y + 47.934_625_631_72 * z + 137.107_382_550_3;
    let b = 36.147_337_179_39 * x - 43.543_469_961_73 * y - 28.508_212_941_35 * z + 108.224_161_073_8;

    (
        (r / 255.0).clamp(0.0, 1.0),
        (g / 255.0).clamp(0.0, 1.0),
        (b / 255.0).clamp(0.0, 1.0),
    )
}

/// Uniformly sample a point inside the sphere of radius `√r_square`.
///
/// Uses the deterministic spherical method from the Inclusive Color Space
/// reference (uniform solid-angle + cube-root radius for volume uniformity).
/// Bit-casts the seed so every `i32` (including 0 and negatives) is a distinct
/// stream, matching the project's other seeded generators.
pub(crate) fn sample_sphere(seed: i32, r_square: f32) -> (f32, f32, f32) {
    let mut rng = fastrand::Rng::with_seed(seed as u64);
    let radius = r_square.max(0.0).sqrt();

    let phi = rng.f32() * TAU;
    let costheta = rng.f32() * 2.0 - 1.0;
    let n = rng.f32();

    // costheta is already cos(θ); avoid acos→cos round-trip.
    let sintheta = (1.0 - costheta * costheta).max(0.0).sqrt();
    let r = radius * n.powf(1.0 / 3.0);

    let t = r * sintheta * phi.cos();
    let u = r * sintheta * phi.sin();
    let v = r * costheta;
    (t, u, v)
}

/// Operation that generates a plausible human skin tone color.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpColorGenerationSkinTone {}

impl OpColorGenerationSkinTone {
    /// Returns the node metadata (name and description) for this operation.
    pub fn settings() -> NodeSettings {
        NodeSettings {
            name: "skin tone".to_string(),
            description: "Generates a diverse, plausible human skin tone via the Inclusive Color Space.".to_string(),
            help: "Uses Toney Alexander's Inclusive Color Space — a PCA-fitted TUV sphere that maps to the range of plausible human skin tones in sRGB (https://toneyalexander.github.io/inclusive-color-space/).\n\nWith random on (the default), the node uniformly samples the sphere of radius √R² from the seed so the same seed always yields the same tone. R² ≈ 1.5 keeps results realistic; 2.0 is the reference broad range (occasional outliers); higher values push into cartoonish/fantasy territory. R² = 0 collapses to the neutral origin tone.\n\nWith random off, set T/U/V directly like a color picker: T is deep↔fair, U is flushed↔ochre, V is cool↔warm. The axes come out of the PCA fit, so they are independent and meaningful rather than being plain HSV. Out-of-sphere extremes are clamped to valid sRGB.\n\nThis is a simplified model of base skin colour only — not subsurface scattering, freckles, conditions, or lighting.".to_string(),
        }
    }

    /// Creates the input definitions: random toggle, seed, R², T/U/V, alpha.
    pub fn create_inputs() -> Vec<Input> {
        vec![
            Input::new("random".to_string(), Value::Bool(true), None, None)
                .with_description("When on, sample a tone from the R² sphere using the seed. When off, use the T/U/V sliders directly."),
            Input::new("seed".to_string(), Value::Integer(1), Some(InputSettings::DragValue { clamp: None, speed: None }), None)
                .with_description("Random seed for sphere sampling (used when random is on). Same seed always yields the same tone."),
            Input::new("r²".to_string(), Value::Decimal(2.0), Some(InputSettings::Slider { range: (0.0, 10.0), step_by: Some(0.1), clamp_to_range: true }), None)
                .with_description("Squared radius of the sampling sphere (used when random is on). 1.5 ≈ realistic variety, 2.0 = reference broad range, higher = more fantastical."),
            Input::new("deep/fair".to_string(), Value::Decimal(0.0), Some(InputSettings::Slider { range: (-2.5, 2.5), step_by: Some(0.01), clamp_to_range: false }), None)
                .with_description("T axis: deeper tones toward negative, fairer tones toward positive (used when random is off)."),
            Input::new("flushed/ochre".to_string(), Value::Decimal(0.0), Some(InputSettings::Slider { range: (-2.5, 2.5), step_by: Some(0.01), clamp_to_range: false }), None)
                .with_description("U axis: flushed/rosy toward one side, ochre/yellowish toward the other (used when random is off)."),
            Input::new("cool/warm".to_string(), Value::Decimal(0.0), Some(InputSettings::Slider { range: (-2.5, 2.5), step_by: Some(0.01), clamp_to_range: false }), None)
                .with_description("V axis: cooler (bluish) toward one side, warmer (orange) toward the other (used when random is off)."),
            Input::new("alpha".to_string(), Value::Decimal(1.0), Some(InputSettings::Slider { range: (0.0, 1.0), step_by: Some(0.01), clamp_to_range: true }), None)
                .with_description("Opacity of the resulting color (0 transparent, 1 opaque)."),
        ]
    }

    /// Creates the outputs: the color plus the T/U/V coordinates that produced it.
    pub fn create_outputs() -> Vec<Output> {
        vec![
            Output::new("color".to_string(), Value::Color(Color::default()), None)
                .with_description("Skin tone as an sRGB color."),
            Output::new("deep/fair".to_string(), Value::Decimal(0.0), None)
                .with_description("T coordinate used for this color (useful when random is on)."),
            Output::new("flushed/ochre".to_string(), Value::Decimal(0.0), None)
                .with_description("U coordinate used for this color (useful when random is on)."),
            Output::new("cool/warm".to_string(), Value::Decimal(0.0), None)
                .with_description("V coordinate used for this color (useful when random is on)."),
        ]
    }

    /// Executes the operation, producing a skin tone color from random sampling or manual TUV.
    pub async fn run(inputs: &mut [Input]) -> Result<OperationResponse, OperationError> {
        let start_time = Instant::now();
        let mut input_errors: Vec<(usize, String)> = vec![];

        let random_converted = convert_input(inputs, 0, ValueType::Bool, &mut input_errors);
        let seed_converted = convert_input(inputs, 1, ValueType::Integer, &mut input_errors);
        let r_square_converted = convert_input(inputs, 2, ValueType::Decimal, &mut input_errors);
        let t_converted = convert_input(inputs, 3, ValueType::Decimal, &mut input_errors);
        let u_converted = convert_input(inputs, 4, ValueType::Decimal, &mut input_errors);
        let v_converted = convert_input(inputs, 5, ValueType::Decimal, &mut input_errors);
        let alpha_converted = convert_input(inputs, 6, ValueType::Decimal, &mut input_errors);

        if !input_errors.is_empty() {
            return Err(OperationError { input_errors, node_error: None });
        }

        let Value::Bool(random) = random_converted.unwrap() else { unreachable!() };
        let Value::Integer(seed) = seed_converted.unwrap() else { unreachable!() };
        let Value::Decimal(r_square) = r_square_converted.unwrap() else { unreachable!() };
        let Value::Decimal(manual_t) = t_converted.unwrap() else { unreachable!() };
        let Value::Decimal(manual_u) = u_converted.unwrap() else { unreachable!() };
        let Value::Decimal(manual_v) = v_converted.unwrap() else { unreachable!() };
        let Value::Decimal(alpha) = alpha_converted.unwrap() else { unreachable!() };

        let (t, u, v) = if random {
            sample_sphere(seed, r_square)
        } else {
            (manual_t, manual_u, manual_v)
        };

        let (r, g, b) = tuv_to_srgb(t, u, v);
        let color = Color::from_srgb_float(r, g, b, alpha);

        Ok(OperationResponse {
            time: Instant::now().duration_since(start_time),
            responses: vec![
                OutputResponse { value: Value::Color(color) },
                OutputResponse { value: Value::Decimal(t) },
                OutputResponse { value: Value::Decimal(u) },
                OutputResponse { value: Value::Decimal(v) },
            ],
        })
    }
}

#[cfg(test)]
#[path = "skin_tone_tests.rs"]
mod tests;
