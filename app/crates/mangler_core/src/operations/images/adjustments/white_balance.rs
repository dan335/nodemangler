//! White balance: Kelvin temperature and green/magenta tint correction.
//!
//! The temperature control names the colour of the light the scene is treated
//! as having been shot under: its chromaticity is looked up on the Planckian
//! (black-body) locus with the Kang et al. (2002) cubic approximation, and a
//! Bradford chromatic adaptation transform maps that illuminant's white back to
//! the neutral reference. Declaring a *higher* (bluer) illuminant therefore
//! warms the image and a lower one cools it — the same sense as Lightroom's
//! temperature slider, and the same sign as this node's previous gain model.
//!
//! The whole correction collapses to a single 3x3 matrix built once per run and
//! applied in linear RGB. Grayscale inputs have no chroma to balance and pass
//! through unchanged.

use crate::color::color_spaces::rgb_linear::{linear_to_nonlinear_srgb, nonlinear_to_linear_rgb};
use crate::color::Color;
use crate::color::color_spaces::xyz::{RGB2XYZ_MATRIX, XYZ2RGB_MATRIX};
use crate::get_id;
use crate::value::ValueType;
use crate::input::{Input, InputSettings};
use crate::node_settings::NodeSettings;
use crate::operations::{OperationResponse, OperationError, OutputResponse, default_image, convert_input};
use crate::output::Output;
use crate::value::Value;
use glam::f32::{Mat3, Vec3};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use std::time::Instant;

/// Temperature at which the node is a no-op. The adaptation reference is the
/// Planckian locus point at this Kelvin value (≈ D65, which sits just off the
/// locus), so the default temperature is *exactly* identity and the slider is
/// continuous through it — anchoring on D65 itself would leave a small residual
/// tint at 6500 K.
const NEUTRAL_TEMPERATURE: f32 = 6500.0;

/// How far ±1 tint slides the illuminant's `y` chromaticity. A real tint axis
/// is perpendicular to the locus in a uniform space; this is the simplified
/// magenta–green approximation (raising `y` greens the *light*, so the
/// correction pushes the image magenta).
const TINT_SCALE: f32 = 0.05;

/// Bradford cone response matrix (XYZ → LMS-like sharpened cone space).
const BRADFORD: [[f32; 3]; 3] = [
    [0.8951, 0.2664, -0.1614],
    [-0.7502, 1.7135, 0.0367],
    [0.0389, -0.0685, 1.0296],
];

/// White balance adjustment via a Planckian illuminant and Bradford adaptation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpImageAdjustmentWhiteBalance {}

/// Builds a [`Mat3`] from row-major rows (glam stores columns).
fn mat3_from_rows(rows: [[f32; 3]; 3]) -> Mat3 {
    Mat3::from_cols_array(&[
        rows[0][0], rows[0][1], rows[0][2],
        rows[1][0], rows[1][1], rows[1][2],
        rows[2][0], rows[2][1], rows[2][2],
    ])
    .transpose()
}

/// CIE 1931 xy chromaticity of a black-body radiator at `kelvin`, using the
/// Kang et al. (2002) piecewise cubic approximation to the Planckian locus
/// (valid 1667–25000 K; the input is clamped into that band).
pub(crate) fn planckian_xy(kelvin: f32) -> (f32, f32) {
    let t = kelvin.clamp(1667.0, 25000.0) as f64;
    let (t2, t3) = (t * t, t * t * t);

    let x = if t <= 4000.0 {
        -0.2661239e9 / t3 - 0.2343589e6 / t2 + 0.8776956e3 / t + 0.179910
    } else {
        -3.0258469e9 / t3 + 2.1070379e6 / t2 + 0.2226347e3 / t + 0.240390
    };
    let (x2, x3) = (x * x, x * x * x);

    let y = if t <= 2222.0 {
        -1.1063814 * x3 - 1.34811020 * x2 + 2.18555832 * x - 0.20219683
    } else if t <= 4000.0 {
        -0.9549476 * x3 - 1.37418593 * x2 + 2.09137015 * x - 0.16748867
    } else {
        3.0817580 * x3 - 5.87338670 * x2 + 3.75112997 * x - 0.37001483
    };

    (x as f32, y as f32)
}

/// XYZ tristimulus of an `xy` chromaticity normalised to `Y = 1`.
fn xy_to_xyz(x: f32, y: f32) -> Vec3 {
    let y = if y.abs() < 1e-6 { 1e-6 } else { y };
    Vec3::new(x / y, 1.0, (1.0 - x - y) / y)
}

/// Bradford chromatic adaptation matrix taking XYZ under illuminant `src` to
/// XYZ under illuminant `dst` (`M = B⁻¹ · diag(ρ_dst / ρ_src) · B`).
fn bradford_adaptation(src: (f32, f32), dst: (f32, f32)) -> Mat3 {
    let b = mat3_from_rows(BRADFORD);
    let b_inv = b.inverse();
    let s = b * xy_to_xyz(src.0, src.1);
    let d = b * xy_to_xyz(dst.0, dst.1);
    let ratio = Mat3::from_diagonal(Vec3::new(d.x / s.x, d.y / s.y, d.z / s.z));
    b_inv * ratio * b
}

/// The complete linear-RGB correction matrix for a temperature and tint:
/// `XYZ→RGB · CAT · RGB→XYZ`, adapting *from* the chosen illuminant *to* the
/// neutral reference so that a higher Kelvin warms the image.
pub(crate) fn white_balance_matrix(temperature: f32, tint: f32) -> Mat3 {
    let (x, y) = planckian_xy(temperature);
    let src = (x, y + tint * TINT_SCALE);
    let dst = planckian_xy(NEUTRAL_TEMPERATURE);
    XYZ2RGB_MATRIX * bradford_adaptation(src, dst) * RGB2XYZ_MATRIX
}

/// Below this per-channel spread a reference colour is treated as carrying no
/// chroma.
const ACHROMATIC_EPSILON: f32 = 1e-4;

/// The linear-RGB correction that makes `reference` render neutral.
///
/// The reference's own chromaticity is taken as the scene illuminant and
/// adapted to the neutral reference, so whatever colour the user says *should*
/// have been grey becomes grey. Unlike temperature and tint this is not
/// confined to the Planckian locus, so it can correct fluorescent, LED and
/// mixed lighting that no single temperature describes.
///
/// Returns `None` when the reference carries no chroma — an achromatic
/// reference already is neutral, so there is nothing to correct, and that is
/// what keeps the default (white) an exact no-op.
///
/// Note this adapts to the *working space's* white point, taken from the
/// RGB→XYZ matrix so it cannot drift from the primaries, rather than to the
/// `NEUTRAL_TEMPERATURE` locus point the temperature path uses. The two differ
/// slightly — the Planckian 6500 K point sits just off D65 — and only the true
/// white point makes an adapted colour come out with equal RGB channels, which
/// is the whole contract of this control. The temperature path is unaffected
/// because it adapts locus-to-locus and is exactly identity at its own anchor.
pub(crate) fn neutral_reference_matrix(reference: Color) -> Option<Mat3> {
    if (reference.r - reference.g).abs() < ACHROMATIC_EPSILON
        && (reference.g - reference.b).abs() < ACHROMATIC_EPSILON
    {
        return None;
    }

    let linear = Vec3::new(
        nonlinear_to_linear_rgb(reference.r),
        nonlinear_to_linear_rgb(reference.g),
        nonlinear_to_linear_rgb(reference.b),
    );
    let xyz = RGB2XYZ_MATRIX * linear;
    let sum = xyz.x + xyz.y + xyz.z;
    if sum < 1e-6 {
        // Numerically black: no usable chromaticity to adapt from.
        return None;
    }

    let src = (xyz.x / sum, xyz.y / sum);
    Some(XYZ2RGB_MATRIX * bradford_adaptation(src, working_space_white_xy()) * RGB2XYZ_MATRIX)
}

/// Chromaticity of the working space's white, i.e. the XYZ of linear RGB
/// `(1, 1, 1)`. Derived from the matrix rather than hardcoded so it stays
/// correct if the primaries ever change.
fn working_space_white_xy() -> (f32, f32) {
    let white = RGB2XYZ_MATRIX * Vec3::ONE;
    let sum = white.x + white.y + white.z;
    (white.x / sum, white.y / sum)
}

impl OpImageAdjustmentWhiteBalance {
    /// Returns the node metadata (name and description) for white balance.
    pub fn settings() -> NodeSettings {
        NodeSettings {
            name: "white balance".to_string(),
            description: "Corrects colour temperature (warm/cool) and tint (green/magenta), or neutralises a reference colour.".to_string(),
            help: "White balance via Planckian-locus illuminant (Kang et al. 2002 approximation) and Bradford chromatic adaptation, applied in linear RGB. Tint is a simplified magenta-green chromaticity offset.\n\nTemperature names the colour of the light the shot is treated as having been taken under, in Kelvin. Raising it above the 6500 K neutral warms the image (more orange), lowering it cools the image (more blue) — the same direction as Lightroom's temperature slider. Positive tint pushes toward magenta, negative toward green.\n\n'neutral reference' is the eyedropper: give it a colour that should have been neutral grey and the correction that makes it grey is derived directly, without going through a temperature at all. Wire a 'sample pixel' node pointed at a grey card or a white wall. Because it is not confined to the Planckian locus, this can correct fluorescent, LED and mixed lighting that no single temperature can describe. Any grey reference — including the white default — means no reference correction, so the input is a no-op until you feed it something with colour in it.\n\nThe two work together: the reference sets the base white point and temperature/tint then trim it, so you can pick a grey and still warm the result to taste.\n\nThe correction collapses to one 3x3 matrix built per run: sRGB is linearized, transformed, re-encoded and clamped to 0-1. At exactly 6500 K with no tint and no reference colour the image passes through untouched. Grayscale inputs (1 or 2 channels) carry no chroma and pass through unchanged; alpha is always preserved.".to_string(),
        }
    }

    /// Creates input ports: image, temperature (Kelvin), and tint.
    pub fn create_inputs() -> Vec<Input> {
        vec![
            Input::new("image".to_string(), Value::Image { data: default_image(), change_id: get_id() }, None, None)
                .with_description("Source colour image to white-balance."),
            Input::new("temperature".to_string(), Value::Decimal(NEUTRAL_TEMPERATURE), Some(InputSettings::Slider { range: (2000.0, 12000.0), step_by: Some(50.0), clamp_to_range: true }), None)
                .with_description("Colour temperature in Kelvin; above 6500 warms the image, below cools it."),
            Input::new("tint".to_string(), Value::Decimal(0.0), Some(InputSettings::Slider { range: (-1.0, 1.0), step_by: Some(0.01), clamp_to_range: true }), None)
                .with_description("Green/magenta shift; positive pushes toward magenta."),
            Input::new("neutral reference".to_string(), Value::Color(Color { r: 1.0, g: 1.0, b: 1.0, a: 1.0 }), None, None)
                .with_description("A colour that should have been neutral grey — wire a 'sample pixel' node at a grey card to use it as an eyedropper. Any grey (including the white default) means no reference correction."),
        ]
    }

    /// Creates the output port: the white-balanced image.
    pub fn create_outputs() -> Vec<Output> {
        vec![
            Output::new("output".to_string(), Value::Image { data: default_image(), change_id: get_id() }, None)
                .with_description("Image with the chromatic adaptation applied."),
        ]
    }

    /// Executes the white balance: builds the adaptation matrix once, then
    /// applies it to every pixel in linear RGB.
    pub async fn run(inputs: &mut [Input]) -> Result<OperationResponse, OperationError> {
        let start_time = Instant::now();
        let mut input_errors: Vec<(usize, String)> = vec![];

        let image_converted = convert_input(inputs, 0, ValueType::Image, &mut input_errors);
        let temp_converted = convert_input(inputs, 1, ValueType::Decimal, &mut input_errors);
        let tint_converted = convert_input(inputs, 2, ValueType::Decimal, &mut input_errors);
        let reference_converted = convert_input(inputs, 3, ValueType::Color, &mut input_errors);

        if !input_errors.is_empty() { return Err(OperationError { input_errors, node_error: None }); }

        let Value::Image { data, change_id: _ } = image_converted.unwrap() else { unreachable!() };
        let Value::Decimal(temperature) = temp_converted.unwrap() else { unreachable!() };
        let Value::Decimal(tint) = tint_converted.unwrap() else { unreachable!() };
        let Value::Color(reference) = reference_converted.unwrap() else { unreachable!() };

        let reference_matrix = neutral_reference_matrix(reference);
        let temperature_neutral =
            (temperature - NEUTRAL_TEMPERATURE).abs() < 1e-3 && tint.abs() < 1e-6;

        // Nothing to do, or grayscale (no chroma to balance): pass the original
        // buffer straight through without touching a pixel.
        if (temperature_neutral && reference_matrix.is_none()) || data.channels() < 3 {
            return Ok(OperationResponse {
                time: Instant::now().duration_since(start_time),
                responses: vec![OutputResponse { value: Value::Image { data, change_id: get_id() } }],
            });
        }

        // The reference sets the base white point and temperature/tint trim it
        // afterwards, so an eyedropper pick and the sliders compose instead of
        // overriding each other.
        let mut matrix = reference_matrix.unwrap_or(Mat3::IDENTITY);
        if !temperature_neutral {
            matrix = white_balance_matrix(temperature, tint) * matrix;
        }

        let mut result = (*data).clone();
        for pixel in result.pixels_mut() {
            let linear = Vec3::new(
                nonlinear_to_linear_rgb(pixel[0]),
                nonlinear_to_linear_rgb(pixel[1]),
                nonlinear_to_linear_rgb(pixel[2]),
            );
            let balanced = matrix * linear;
            pixel[0] = linear_to_nonlinear_srgb(balanced.x).clamp(0.0, 1.0);
            pixel[1] = linear_to_nonlinear_srgb(balanced.y).clamp(0.0, 1.0);
            pixel[2] = linear_to_nonlinear_srgb(balanced.z).clamp(0.0, 1.0);
            // alpha (and any further channels) untouched
        }

        Ok(OperationResponse {
            time: Instant::now().duration_since(start_time),
            responses: vec![OutputResponse { value: Value::Image { data: Arc::new(result), change_id: get_id() } }],
        })
    }
}

#[cfg(test)]
#[path = "white_balance_tests.rs"]
mod tests;
