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

use crate::float_image::FloatImage;
use crate::get_id;
use crate::value::ToneMapOperator;
use crate::input::{Input, InputSettings};
use crate::node_settings::NodeSettings;
use crate::convert_inputs;
use crate::operations::{OperationResponse, OperationError, OutputResponse, image_input, image_output};
use crate::operations::images::adjustments::common::smoothstep;
use crate::output::Output;
use crate::value::Value;
use rayon::prelude::*;
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
            image_input("image")
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
            image_output("output")
                .with_description("Tone-mapped image, clamped to [0, 1]; alpha preserved."),
        ]
    }

    /// Executes the tone map operation: exposure pre-scale, then the selected operator, then clamp.
    pub async fn run(inputs: &mut [Input]) -> Result<OperationResponse, OperationError> {
        let start_time = Instant::now();

        convert_inputs! { inputs;
            Image(data) = 0,
            ToneMapOperator(operator) = 1,
            Decimal(exposure) = 2,
            Decimal(white_point) = 3,
            Decimal(contrast) = 4,
            Decimal(mid_gray) = 5,
            Decimal(key) = 6,
            Bool(adapt) = 7,
            Decimal(bias) = 8,
        }

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
        // Loop-invariant: the value the Drago curve reaches at the white point.
        let drago_at_white = drago_normalizer(drago_lmax, bias);

        // Which of the three mapping paths this run takes depends only on the
        // operator and the channel count — both fixed for the whole image — so
        // it is decided once here rather than re-matched (with guards) for
        // every pixel. The per-channel path goes further and resolves its
        // 13-way operator match into a single closure up front, so the inner
        // loop is a call, not a dispatch.
        let rgb_direct = matches!(
            operator,
            ToneMapOperator::Agx | ToneMapOperator::PbrNeutral
        ) && color_ch >= 3;
        let luminance_scaled = matches!(
            operator,
            ToneMapOperator::ReinhardLuminance
                | ToneMapOperator::PhotographicReinhard
                | ToneMapOperator::Drago
        ) && color_ch >= 3;

        if rgb_direct {
            // --- RGB / multi-channel operators ---
            let map_rgb: fn([f32; 3]) -> [f32; 3] = match operator {
                ToneMapOperator::Agx => agx_rgb,
                ToneMapOperator::PbrNeutral => pbr_neutral_rgb,
                _ => unreachable!("rgb_direct is only set for Agx and PbrNeutral"),
            };
            result.par_pixels_mut().for_each(|pixel| {
                let rgb = map_rgb([
                    pixel[0] * exposure_gain,
                    pixel[1] * exposure_gain,
                    pixel[2] * exposure_gain,
                ]);
                pixel[0] = rgb[0].clamp(0.0, 1.0);
                pixel[1] = rgb[1].clamp(0.0, 1.0);
                pixel[2] = rgb[2].clamp(0.0, 1.0);
            });
        } else if luminance_scaled {
            // --- Luminance operators (3+ channels) ---
            // The mapping is handed to a generic driver rather than boxed into
            // a `dyn Fn`: each arm monomorphises `map_luminance`, so the curve
            // inlines into the pixel loop instead of costing an indirect call
            // per pixel. The arithmetic is unchanged.
            match operator {
                ToneMapOperator::ReinhardLuminance => {
                    map_luminance(&mut result, exposure_gain, reinhard)
                }
                ToneMapOperator::PhotographicReinhard => map_luminance(
                    &mut result,
                    exposure_gain,
                    |lum| reinhard_extended(lum * photo_scale, white_point),
                ),
                ToneMapOperator::Drago => map_luminance(&mut result, exposure_gain, |lum| {
                    drago_raw(lum, drago_lmax, bias) / drago_at_white
                }),
                _ => unreachable!("luminance_scaled is only set for the three luminance operators"),
            }
        } else {
            // --- Per-channel (and grayscale fallbacks) ---
            // Same monomorphisation as above: one `map_channels` instantiation
            // per operator, no `dyn Fn` call per channel per pixel.
            match operator {
                ToneMapOperator::Linear => map_channels(&mut result, color_ch, exposure_gain, |v| v),
                ToneMapOperator::Reinhard | ToneMapOperator::ReinhardLuminance => {
                    map_channels(&mut result, color_ch, exposure_gain, reinhard)
                }
                ToneMapOperator::ReinhardExtended => {
                    map_channels(&mut result, color_ch, exposure_gain, |v| {
                        reinhard_extended(v, white_point)
                    })
                }
                ToneMapOperator::PhotographicReinhard => {
                    map_channels(&mut result, color_ch, exposure_gain, |v| {
                        reinhard_extended(v * photo_scale, white_point)
                    })
                }
                ToneMapOperator::Aces => map_channels(&mut result, color_ch, exposure_gain, aces),
                ToneMapOperator::HableFilmic => {
                    map_channels(&mut result, color_ch, exposure_gain, |v| {
                        if hable_norm.abs() < 1e-6 {
                            0.0
                        } else {
                            hable_filmic_curve(v) / hable_norm
                        }
                    })
                }
                ToneMapOperator::Hejl => map_channels(&mut result, color_ch, exposure_gain, hejl),
                ToneMapOperator::Gt => map_channels(&mut result, color_ch, exposure_gain, |v| {
                    gt_uchimura(v, contrast)
                }),
                ToneMapOperator::Agx => map_channels(&mut result, color_ch, exposure_gain, |v| {
                    agx_rgb([v, v, v])[0]
                }),
                ToneMapOperator::Sigmoid => {
                    map_channels(&mut result, color_ch, exposure_gain, |v| {
                        sigmoid(v, contrast, mid_gray)
                    })
                }
                ToneMapOperator::Drago => map_channels(&mut result, color_ch, exposure_gain, |v| {
                    drago_raw(v, drago_lmax, bias) / drago_at_white
                }),
                ToneMapOperator::PbrNeutral => {
                    map_channels(&mut result, color_ch, exposure_gain, |v| {
                        pbr_neutral_rgb([v, v, v])[0]
                    })
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

/// Applies `map` to each colour channel (after `exposure_gain`) and clamps to
/// 0..1.
///
/// Generic over the mapping so every operator gets its own inlined copy of the
/// loop; the alternative — one `Box<dyn Fn(f32) -> f32>` shared by all thirteen
/// — put an un-inlinable indirect call in the hottest loop in the node (three
/// or four per pixel, ~72M at 24 MP).
fn map_channels(
    image: &mut FloatImage,
    color_ch: usize,
    exposure_gain: f32,
    map: impl Fn(f32) -> f32 + Sync,
) {
    image.par_pixels_mut().for_each(|pixel| {
        for val in pixel.iter_mut().take(color_ch) {
            *val = map(*val * exposure_gain).clamp(0.0, 1.0);
        }
    });
}

/// Maps Rec. 709 luminance through `map` and rescales RGB by the ratio, so hue
/// and saturation are preserved. Generic for the same reason as
/// [`map_channels`].
fn map_luminance(image: &mut FloatImage, exposure_gain: f32, map: impl Fn(f32) -> f32 + Sync) {
    image.par_pixels_mut().for_each(|pixel| {
        let r = pixel[0] * exposure_gain;
        let g = pixel[1] * exposure_gain;
        let b = pixel[2] * exposure_gain;
        let lum = crate::luma::rec709(r, g, b).max(0.0);
        let lum_mapped = map(lum);
        let scale = if lum > 1e-8 { lum_mapped / lum } else { 0.0 };
        pixel[0] = (r * scale).clamp(0.0, 1.0);
        pixel[1] = (g * scale).clamp(0.0, 1.0);
        pixel[2] = (b * scale).clamp(0.0, 1.0);
    });
}


/// Log-average and max Rec.709 luminance over color channels (post exposure gain).
fn scene_luminance_stats(img: &crate::float_image::FloatImage, color_ch: usize, exposure_gain: f32) -> (f32, f32) {
    let mut sum_log = 0.0f64;
    let mut count = 0u64;
    let mut max_lum = 0.0f32;
    let delta = 1e-6f32;

    for pixel in img.pixels() {
        let lum = if color_ch >= 3 {
            crate::luma::rec709(
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

/// Heuristic log-domain sigmoid.
fn sigmoid(v: f32, contrast: f32, mid_gray: f32) -> f32 {
    let ratio = (v.max(1e-6) / mid_gray.max(1e-6)).max(1e-6);
    1.0 / (1.0 + ratio.powf(-contrast.max(1e-3)))
}

/// The Drago normalizer `drago_raw(lmax, lmax, bias)` — the value the curve
/// reaches at the white point, which every pixel is divided by.
///
/// It depends only on `lmax` and `bias`, both fixed for a whole image, so it is
/// computed once per run and the pixel loops just divide [`drago_raw`] by it.
/// It used to be recomputed once per pixel (and once per channel on the
/// per-channel path), which put a `ln`, two `log10`s and a `powf` of
/// loop-invariant work in the hot loop. `hable_filmic`'s equivalent normalizer
/// was already hoisted this way; Drago's was not.
fn drago_normalizer(lmax: f32, bias: f32) -> f32 {
    drago_raw(lmax, lmax, bias).max(1e-6)
}

/// Drago 2003 adaptive logarithmic map, unnormalized; callers divide by
/// [`drago_normalizer`] so `Lmax → ~1`.
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
