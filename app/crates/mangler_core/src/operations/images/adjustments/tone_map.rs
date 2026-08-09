//! Tone mapping operation for images.
//!
//! Compresses (typically high-dynamic-range, i.e. values that may exceed 1.0
//! after exposure) pixel values into the displayable `[0, 1]` range using one
//! of several well-known tone mapping curves. Each non-alpha channel value is
//! first scaled by `2^exposure` (photographic stops), then passed through the
//! selected operator, then clamped to `[0, 1]`. Alpha passes through untouched.
//!
//! Per-operator controls (the settings panel hides the unused ones):
//! - **Linear / Reinhard / Reinhard Luminance / ACES / Hejl / AgX / PBR Neutral** — exposure
//! - **Reinhard Extended / Hable Filmic** — exposure + white point
//! - **Photographic Reinhard** — exposure + white point + key + adapt
//! - **GT** — exposure + contrast
//! - **Sigmoid** — exposure + contrast + mid gray
//! - **Drago** — exposure + white point + bias

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

/// Default log-domain sigmoid contrast.
const DEFAULT_SIGMOID_CONTRAST: f32 = 1.6;

/// Default mid-gray reference for the sigmoid operator.
const DEFAULT_SIGMOID_MID_GRAY: f32 = 0.18;

/// Default photographic key (Reinhard 2002 mid-gray).
const DEFAULT_KEY: f32 = 0.18;

/// Default Drago bias (paper recommends ~0.85).
const DEFAULT_DRAGO_BIAS: f32 = 0.85;

/// Rec.709 luminance weights (linear light).
const LUMA_R: f32 = 0.2126;
const LUMA_G: f32 = 0.7152;
const LUMA_B: f32 = 0.0722;

/// Uncharted 2 filmic curve constants (Hable 2010).
const HABLE_A: f32 = 0.15;
const HABLE_B: f32 = 0.50;
const HABLE_C: f32 = 0.10;
const HABLE_D: f32 = 0.20;
const HABLE_E: f32 = 0.02;
const HABLE_F: f32 = 0.30;

/// Tone mapping: compresses HDR pixel values into `[0, 1]` using a selectable
/// operator with an exposure pre-scale and operator-specific controls.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpImageAdjustmentToneMap {}

impl OpImageAdjustmentToneMap {
    /// Returns the node metadata (name, description, and help) for the tone map operation.
    pub fn settings() -> NodeSettings {
        NodeSettings {
            name: "tone map".to_string(),
            description: "Compress HDR values into displayable range using a selectable tone-mapping curve.".to_string(),
            help: "Compresses (typically high-dynamic-range) pixel values into the displayable \
                   0-1 range. Each non-alpha channel is first scaled by 2^exposure \
                   (photographic stops), then passed through the selected operator, then clamped \
                   to [0, 1]. Alpha is always preserved.\n\n\
                   The settings panel only shows controls used by the selected operator \
                   (connected/exposed inputs always stay visible).\n\n\
                   Operators:\n\n\
                   - Linear — exposure then hard clamp; baseline with no curve.\n\n\
                   - Reinhard — simple global per-channel v/(1+v) (Reinhard et al. 2002).\n\n\
                   - Reinhard Luminance — same curve on Rec.709 luminance, chrominance restored; \
                   preferred for photography over per-channel Reinhard.\n\n\
                   - Reinhard Extended — adds a white point so that value maps back to ~1.0.\n\n\
                   - Photographic Reinhard — full 2002 photographic form: key scales mid-gray, \
                   optional scene log-average adaptation (turn adapt off for video stability), \
                   then extended Reinhard with white point.\n\n\
                   - ACES — Narkowicz 2015 fast analytic fit to the ACES filmic tonemapper.\n\n\
                   - Hable Filmic — Uncharted 2 filmic curve (Hable GDC 2010), normalized by \
                   white point.\n\n\
                   - Hejl — Hejl–Burgess–Dawson filmic approximation (cheap shoulder/toe).\n\n\
                   - GT — Uchimura Gran Turismo tonemapper (piecewise toe/linear/shoulder); \
                   contrast controls the linear-section slope.\n\n\
                   - AgX — Blender AgX (minimal analytic inset/log/sigmoid/outset implementation).\n\n\
                   - Sigmoid — heuristic log-domain logistic centered on mid gray (darktable-inspired).\n\n\
                   - Drago — Drago 2003 adaptive logarithmic map; bias controls contrast \
                   compression (typical ~0.85); white point is the scene max luminance used \
                   for normalization.\n\n\
                   - PBR Neutral — Khronos PBR Neutral: preserves base colors under grayscale \
                   light with smooth highlight compression (product/PBR previews).\n\n\
                   1-4 channel images are supported; grayscale/color channels are tone mapped, \
                   alpha (if present) passes through unchanged. Operators that are natively RGB \
                   (AgX, PBR Neutral) treat 1-channel images as grayscale.".to_string(),
        }
    }

    /// Creates the input ports. Operator-specific controls always exist so
    /// values persist across operator switches; the GUI hides unused ones.
    pub fn create_inputs() -> Vec<Input> {
        vec![
            Input::new("image".to_string(), Value::Image { data: default_image(), change_id: get_id() }, None, None)
                .with_description("Source image to tone map."),
            Input::new("operator".to_string(), Value::ToneMapOperator(ToneMapOperator::Reinhard), None, None)
                .with_description("Tone mapping curve to apply."),
            Input::new("exposure".to_string(), Value::Decimal(0.0), Some(InputSettings::Slider { range: (-5.0, 5.0), step_by: Some(0.01), clamp_to_range: false }), None)
                .with_description("Exposure in stops, applied as a 2^exposure multiplier before tone mapping. Used by all operators."),
            Input::new("white point".to_string(), Value::Decimal(4.0), Some(InputSettings::Slider { range: (0.5, 16.0), step_by: Some(0.01), clamp_to_range: true }), None)
                .with_description("Scene value that maps back to ~1.0 (Reinhard Extended, Hable, Photographic Reinhard) or Lmax for Drago."),
            Input::new("contrast".to_string(), Value::Decimal(DEFAULT_SIGMOID_CONTRAST), Some(InputSettings::Slider { range: (0.5, 4.0), step_by: Some(0.01), clamp_to_range: true }), None)
                .with_description("Sigmoid S-curve steepness, or GT linear-section contrast. Used by Sigmoid and GT."),
            Input::new("mid gray".to_string(), Value::Decimal(DEFAULT_SIGMOID_MID_GRAY), Some(InputSettings::Slider { range: (0.01, 1.0), step_by: Some(0.001), clamp_to_range: true }), None)
                .with_description("Sigmoid pivot: the input value that maps to 0.5 (default 0.18). Used by Sigmoid only."),
            Input::new("key".to_string(), Value::Decimal(DEFAULT_KEY), Some(InputSettings::Slider { range: (0.05, 1.0), step_by: Some(0.01), clamp_to_range: true }), None)
                .with_description("Photographic key / mid-gray target (Reinhard 2002, default 0.18). Used by Photographic Reinhard."),
            Input::new("adapt".to_string(), Value::Bool(true), None, None)
                .with_description("When on, Photographic Reinhard scales by the image log-average luminance. Turn off for video to avoid flicker."),
            Input::new("bias".to_string(), Value::Decimal(DEFAULT_DRAGO_BIAS), Some(InputSettings::Slider { range: (0.5, 1.0), step_by: Some(0.01), clamp_to_range: true }), None)
                .with_description("Drago bias: lower = more contrast compression in bright areas (paper default ~0.85). Used by Drago only."),
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

        let image_converted       = convert_input(inputs, 0, ValueType::Image, &mut input_errors);
        let operator_converted    = convert_input(inputs, 1, ValueType::ToneMapOperator, &mut input_errors);
        let exposure_converted    = convert_input(inputs, 2, ValueType::Decimal, &mut input_errors);
        let white_point_converted = convert_input(inputs, 3, ValueType::Decimal, &mut input_errors);
        let contrast_converted    = convert_input(inputs, 4, ValueType::Decimal, &mut input_errors);
        let mid_gray_converted    = convert_input(inputs, 5, ValueType::Decimal, &mut input_errors);
        let key_converted         = convert_input(inputs, 6, ValueType::Decimal, &mut input_errors);
        let adapt_converted       = convert_input(inputs, 7, ValueType::Bool, &mut input_errors);
        let bias_converted        = convert_input(inputs, 8, ValueType::Decimal, &mut input_errors);

        if !input_errors.is_empty() {
            return Err(OperationError { input_errors, node_error: None });
        }

        let Value::Image { data, change_id: _ } = image_converted.unwrap() else { unreachable!() };
        let Value::ToneMapOperator(operator) = operator_converted.unwrap() else { unreachable!() };
        let Value::Decimal(exposure) = exposure_converted.unwrap() else { unreachable!() };
        let Value::Decimal(white_point) = white_point_converted.unwrap() else { unreachable!() };
        let Value::Decimal(contrast) = contrast_converted.unwrap() else { unreachable!() };
        let Value::Decimal(mid_gray) = mid_gray_converted.unwrap() else { unreachable!() };
        let Value::Decimal(key) = key_converted.unwrap() else { unreachable!() };
        let Value::Bool(adapt) = adapt_converted.unwrap() else { unreachable!() };
        let Value::Decimal(bias) = bias_converted.unwrap() else { unreachable!() };

        let exposure_gain = 2f32.powf(exposure);
        let white_point = white_point.max(1e-3);
        let contrast = contrast.max(1e-3);
        let mid_gray = mid_gray.max(1e-6);
        let key = key.max(1e-4);
        let bias = bias.clamp(0.5, 1.0);
        let hable_norm = hable_filmic_curve(white_point);

        let mut result = (*data).clone();
        let ch = result.channels() as usize;
        let color_ch = if ch == 2 || ch == 4 { ch - 1 } else { ch };

        // Scene stats for adaptive operators (post-exposure luminance).
        let (log_avg, _max_lum) = if matches!(
            operator,
            ToneMapOperator::PhotographicReinhard | ToneMapOperator::Drago
        ) {
            scene_luminance_stats(&result, color_ch, exposure_gain)
        } else {
            (DEFAULT_KEY, white_point)
        };

        let photo_scale = if matches!(operator, ToneMapOperator::PhotographicReinhard) {
            if adapt {
                key / log_avg.max(1e-6)
            } else {
                // Relative to default mid-gray without measuring the frame.
                key / DEFAULT_KEY
            }
        } else {
            1.0
        };

        // Drago Lmax: user white point after exposure scaling context.
        let drago_lmax = white_point.max(1e-3);

        for pixel in result.pixels_mut() {
            match operator {
                // --- RGB / multi-channel operators ---
                ToneMapOperator::Agx | ToneMapOperator::PbrNeutral if color_ch >= 3 => {
                    let mut rgb = [
                        pixel[0] * exposure_gain,
                        pixel[1] * exposure_gain,
                        pixel[2] * exposure_gain,
                    ];
                    rgb = match operator {
                        ToneMapOperator::Agx => agx_rgb(rgb),
                        ToneMapOperator::PbrNeutral => pbr_neutral_rgb(rgb),
                        _ => unreachable!(),
                    };
                    pixel[0] = rgb[0].clamp(0.0, 1.0);
                    pixel[1] = rgb[1].clamp(0.0, 1.0);
                    pixel[2] = rgb[2].clamp(0.0, 1.0);
                }

                // --- Luminance operators (3+ channels) ---
                ToneMapOperator::ReinhardLuminance
                | ToneMapOperator::PhotographicReinhard
                | ToneMapOperator::Drago
                    if color_ch >= 3 =>
                {
                    let r = pixel[0] * exposure_gain;
                    let g = pixel[1] * exposure_gain;
                    let b = pixel[2] * exposure_gain;
                    let lum = luminance(r, g, b).max(0.0);
                    let lum_mapped = match operator {
                        ToneMapOperator::ReinhardLuminance => reinhard(lum),
                        ToneMapOperator::PhotographicReinhard => {
                            let scaled = lum * photo_scale;
                            reinhard_extended(scaled, white_point)
                        }
                        ToneMapOperator::Drago => drago_normalized(lum, drago_lmax, bias),
                        _ => unreachable!(),
                    };
                    let scale = if lum > 1e-8 { lum_mapped / lum } else { 0.0 };
                    pixel[0] = (r * scale).clamp(0.0, 1.0);
                    pixel[1] = (g * scale).clamp(0.0, 1.0);
                    pixel[2] = (b * scale).clamp(0.0, 1.0);
                }

                // --- Per-channel (and grayscale fallbacks) ---
                _ => {
                    for val in pixel.iter_mut().take(color_ch) {
                        let v = *val * exposure_gain;
                        let mapped = match operator {
                            ToneMapOperator::Linear => v,
                            ToneMapOperator::Reinhard => reinhard(v),
                            ToneMapOperator::ReinhardLuminance => reinhard(v),
                            ToneMapOperator::ReinhardExtended => reinhard_extended(v, white_point),
                            ToneMapOperator::PhotographicReinhard => {
                                reinhard_extended(v * photo_scale, white_point)
                            }
                            ToneMapOperator::Aces => aces(v),
                            ToneMapOperator::HableFilmic => {
                                if hable_norm.abs() < 1e-6 {
                                    0.0
                                } else {
                                    hable_filmic_curve(v) / hable_norm
                                }
                            }
                            ToneMapOperator::Hejl => hejl(v),
                            ToneMapOperator::Gt => gt_uchimura(v, contrast),
                            ToneMapOperator::Agx => {
                                let rgb = agx_rgb([v, v, v]);
                                rgb[0]
                            }
                            ToneMapOperator::Sigmoid => sigmoid(v, contrast, mid_gray),
                            ToneMapOperator::Drago => drago_normalized(v, drago_lmax, bias),
                            ToneMapOperator::PbrNeutral => {
                                let rgb = pbr_neutral_rgb([v, v, v]);
                                rgb[0]
                            }
                        };
                        *val = mapped.clamp(0.0, 1.0);
                    }
                }
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

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn luminance(r: f32, g: f32, b: f32) -> f32 {
    LUMA_R * r + LUMA_G * g + LUMA_B * b
}

/// Log-average and max Rec.709 luminance over color channels (post exposure gain).
fn scene_luminance_stats(img: &crate::float_image::FloatImage, color_ch: usize, exposure_gain: f32) -> (f32, f32) {
    let mut sum_log = 0.0f64;
    let mut count = 0u64;
    let mut max_lum = 0.0f32;
    let delta = 1e-6f32;

    for pixel in img.pixels() {
        let lum = if color_ch >= 3 {
            luminance(
                pixel[0] * exposure_gain,
                pixel[1] * exposure_gain,
                pixel[2] * exposure_gain,
            )
        } else {
            pixel[0] * exposure_gain
        }
        .max(0.0);
        max_lum = max_lum.max(lum);
        sum_log += (lum + delta).ln() as f64;
        count += 1;
    }

    let log_avg = if count > 0 {
        (sum_log / count as f64).exp() as f32
    } else {
        DEFAULT_KEY
    };
    (log_avg.max(1e-6), max_lum.max(1e-6))
}

/// Simple Reinhard operator (Reinhard et al. 2002): `v / (1 + v)`.
fn reinhard(v: f32) -> f32 {
    v / (1.0 + v.max(0.0))
}

/// Reinhard extended with a white point.
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

/// Hable's Uncharted 2 filmic curve (unnormalized).
fn hable_filmic_curve(x: f32) -> f32 {
    let x = x.max(0.0);
    ((x * (HABLE_A * x + HABLE_C * HABLE_B) + HABLE_D * HABLE_E)
        / (x * (HABLE_A * x + HABLE_B) + HABLE_D * HABLE_F))
        - HABLE_E / HABLE_F
}

/// Hejl–Burgess–Dawson filmic. Output is roughly display-referred (includes the
/// built-in gamma-ish shoulder of the original fit); clamped by the caller.
fn hejl(v: f32) -> f32 {
    let x = (v - 0.004).max(0.0);
    (x * (6.2 * x + 0.5)) / (x * (6.2 * x + 1.7) + 0.06)
}

/// Uchimura / Gran Turismo tonemapper with configurable contrast (`a`).
/// Other shape params match the published defaults.
fn gt_uchimura(x: f32, contrast: f32) -> f32 {
    let p = 1.0f32; // max display brightness
    let a = contrast.max(0.1); // contrast
    let m = 0.22f32; // linear section start
    let l = 0.4f32; // linear section length
    let c = 1.33f32; // black
    let b = 0.0f32; // pedestal

    let x = x.max(0.0);
    let l0 = ((p - m) * l) / a;
    let s0 = m + l0;
    let s1 = m + a * l0;
    let c2 = (a * p) / (p - s1).max(1e-6);
    let cp = -c2 / p;

    let w0 = 1.0 - smoothstep(0.0, m, x);
    let w2 = if x >= m + l0 { 1.0 } else { 0.0 };
    let w1 = 1.0 - w0 - w2;

    let t = m * (x / m.max(1e-6)).powf(c) + b;
    let s = p - (p - s1) * (cp * (x - s0)).exp();
    let lin = m + a * (x - m);

    t * w0 + lin * w1 + s * w2
}

fn smoothstep(edge0: f32, edge1: f32, x: f32) -> f32 {
    let t = ((x - edge0) / (edge1 - edge0).max(1e-6)).clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

/// Heuristic log-domain sigmoid.
fn sigmoid(v: f32, contrast: f32, mid_gray: f32) -> f32 {
    let ratio = (v.max(1e-6) / mid_gray.max(1e-6)).max(1e-6);
    1.0 / (1.0 + ratio.powf(-contrast.max(1e-3)))
}

/// Drago 2003 adaptive logarithmic map, normalized so `Lmax → ~1`.
fn drago_normalized(lum: f32, lmax: f32, bias: f32) -> f32 {
    let raw = drago_raw(lum, lmax, bias);
    let at_white = drago_raw(lmax, lmax, bias).max(1e-6);
    raw / at_white
}

fn drago_raw(lum: f32, lmax: f32, bias: f32) -> f32 {
    let lum = lum.max(0.0);
    let lmax = lmax.max(1e-6);
    // Classic form with Ldmax=100 → 0.01 factor for a [0,1]-ish range before norm.
    let c1 = 0.01 / (lmax + 1.0).log10().max(1e-6);
    let bias_p = (bias.max(1e-4).ln() / 0.5f32.ln()).clamp(0.01, 10.0);
    let p = (lum / lmax).clamp(0.0, 1.0).powf(bias_p);
    let denom = (2.0 + 8.0 * p).log10().max(1e-6);
    c1 * (lum + 1.0).ln() / denom
}

/// AgX via the three.js / Filament analytic path (linear sRGB in/out).
fn agx_rgb(color: [f32; 3]) -> [f32; 3] {
    let mut v = [
        color[0].max(0.0),
        color[1].max(0.0),
        color[2].max(0.0),
    ];
    // Working space is linear sRGB / Rec.709; AgX is defined on Rec.2020.
    v = mat3_mul(LINEAR_SRGB_TO_LINEAR_REC2020, v);
    v = mat3_mul(AGX_INSET, v);
    v = [v[0].max(1e-10), v[1].max(1e-10), v[2].max(1e-10)];

    const AGX_MIN_EV: f32 = -12.47393;
    const AGX_MAX_EV: f32 = 4.026069;

    for c in &mut v {
        let lg = c.log2();
        *c = ((lg - AGX_MIN_EV) / (AGX_MAX_EV - AGX_MIN_EV)).clamp(0.0, 1.0);
    }

    // 6th-order contrast approx (three.js / Blender mean-error fit).
    for c in &mut v {
        let x = *c;
        let x2 = x * x;
        let x4 = x2 * x2;
        *c = 15.5 * x4 * x2
            - 40.14 * x4 * x
            + 31.96 * x4
            - 6.868 * x2 * x
            + 0.4298 * x2
            + 0.1191 * x
            - 0.00232;
    }

    v = mat3_mul(AGX_OUTSET, v);
    // Linearize the display-encoded sigmoid output, then back to linear sRGB.
    v = [
        v[0].max(0.0).powf(2.2),
        v[1].max(0.0).powf(2.2),
        v[2].max(0.0).powf(2.2),
    ];
    v = mat3_mul(LINEAR_REC2020_TO_LINEAR_SRGB, v);
    [
        v[0].clamp(0.0, 1.0),
        v[1].clamp(0.0, 1.0),
        v[2].clamp(0.0, 1.0),
    ]
}

// Row-major matrices: `out[i] = sum_j M[i][j] * v[j]`.
// Sourced from three.js tonemapping_pars_fragment (Filament/Blender AgX).
// three.js mat3(column0, column1, column2) rewritten as row-major.
const LINEAR_SRGB_TO_LINEAR_REC2020: [[f32; 3]; 3] = [
    [0.6274, 0.3293, 0.0433],
    [0.0691, 0.9195, 0.0113],
    [0.0164, 0.0880, 0.8956],
];
const LINEAR_REC2020_TO_LINEAR_SRGB: [[f32; 3]; 3] = [
    [1.6605, -0.5876, -0.0728],
    [-0.1246, 1.1329, -0.0083],
    [-0.0182, -0.1006, 1.1187],
];
const AGX_INSET: [[f32; 3]; 3] = [
    [0.856627153315983, 0.0951212405381588, 0.0482516061458583],
    [0.137318972929847, 0.761241990602591, 0.101439036467562],
    [0.11189821299995, 0.0767994186031903, 0.811302368396859],
];
const AGX_OUTSET: [[f32; 3]; 3] = [
    [1.1271005818144368, -0.11060664309660323, -0.016493938717834573],
    [-0.1413297634984383, 1.157823702216272, -0.016493938717834257],
    [-0.14132976349843826, -0.11060664309660294, 1.2519364065950405],
];

fn mat3_mul(m: [[f32; 3]; 3], v: [f32; 3]) -> [f32; 3] {
    [
        m[0][0] * v[0] + m[0][1] * v[1] + m[0][2] * v[2],
        m[1][0] * v[0] + m[1][1] * v[1] + m[1][2] * v[2],
        m[2][0] * v[0] + m[2][1] * v[1] + m[2][2] * v[2],
    ]
}

/// Khronos PBR Neutral (Emmett Lalish / model-viewer).
fn pbr_neutral_rgb(mut color: [f32; 3]) -> [f32; 3] {
    const START_COMPRESSION: f32 = 0.8 - 0.04; // 0.76
    const DESATURATION: f32 = 0.15;

    let x = color[0].min(color[1]).min(color[2]);
    let offset = if x < 0.08 {
        x - 6.25 * x * x
    } else {
        0.04
    };
    color[0] -= offset;
    color[1] -= offset;
    color[2] -= offset;

    let peak = color[0].max(color[1]).max(color[2]);
    if peak < START_COMPRESSION {
        return color;
    }

    let d = 1.0 - START_COMPRESSION;
    let new_peak = 1.0 - d * d / (peak + d - START_COMPRESSION);
    let scale = new_peak / peak.max(1e-6);
    color[0] *= scale;
    color[1] *= scale;
    color[2] *= scale;

    let g = 1.0 - 1.0 / (DESATURATION * (peak - new_peak) + 1.0);
    [
        color[0] + (new_peak - color[0]) * g,
        color[1] + (new_peak - color[1]) * g,
        color[2] + (new_peak - color[2]) * g,
    ]
}

#[cfg(test)]
#[path = "tone_map_tests.rs"]
mod tests;
