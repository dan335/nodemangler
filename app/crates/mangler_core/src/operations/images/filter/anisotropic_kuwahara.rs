//! Anisotropic Kuwahara filter (Kyprianidis et al. 2009).
//!
//! A more sophisticated relative of the classic Kuwahara filter. Instead of
//! sampling four axis-aligned square quadrants per pixel, this version:
//!
//! 1. Estimates the local image structure tensor (smoothed outer product of
//!    luminance gradients) and derives, per pixel, a dominant orientation
//!    `phi` and an anisotropy `A ∈ [0, 1]` (1 = a strong directional edge,
//!    0 = locally isotropic).
//! 2. Samples an **ellipse** oriented along `phi`, stretched perpendicular to
//!    the edge direction so the filter "follows" edges instead of cutting
//!    across them.
//! 3. Splits the ellipse into N=8 angular sectors with smooth Gaussian-wedge
//!    weights, computes a weighted mean and luminance variance for each, and
//!    blends the sector means with weights `1 / (variance^q + eps)` so the
//!    flattest sector dominates.
//!
//! The result is a smoother, more painterly stylization than the classic
//! Kuwahara — diagonal edges no longer staircase, and brush-stroke shapes
//! follow object contours.
//!
//! Cost: heavier than classic Kuwahara — needs a structure tensor pass over
//! the whole image plus N×(2r+1)² weighted samples per pixel — but rows are
//! processed in parallel via rayon.

use crate::float_image::FloatImage;
use crate::get_id;
use crate::value::ValueType;
use crate::input::{Input, InputSettings};
use crate::node_settings::NodeSettings;
use crate::operations::{OperationResponse, OperationError, OutputResponse, default_image, convert_input};
use crate::output::Output;
use crate::value::Value;
use rayon::prelude::*;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use std::time::Instant;

/// Number of angular sectors. 8 is the standard from Kyprianidis 2009 — gives
/// smooth coverage of all directions without making the per-pixel inner loop
/// prohibitively expensive.
const SECTORS: usize = 8;

/// Anisotropic Kuwahara edge-preserving smoothing filter (Kyprianidis 2009).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpImageAdjustmentAnisotropicKuwahara {}

impl OpImageAdjustmentAnisotropicKuwahara {
    /// Returns the node metadata (name and description).
    pub fn settings() -> NodeSettings {
        NodeSettings {
            name: "anisotropic kuwahara".to_string(),
            description: "Edge-following painterly smoothing (Kyprianidis 2009). Slower but smoother than classic Kuwahara.".to_string(),
            help: "Kyprianidis 2009 variant of the Kuwahara filter. A smoothed structure tensor yields a per-pixel edge orientation and anisotropy; sampling happens along an ellipse oriented along the edge, split into 8 Gaussian angular sectors, and sector means are blended with weights `1 / (variance^q + eps)` so the flattest sector dominates.\n\nBrush strokes follow object contours instead of staircasing on diagonals. Sharpness (q) posterizes flat regions, alpha stretches strokes along edges. Heavier than classic Kuwahara but parallelized over rows.".to_string(),
        }
    }

    /// Creates the input ports.
    pub fn create_inputs() -> Vec<Input> {
        vec![
            Input::new("image".to_string(), Value::Image { data: default_image(), change_id: get_id() }, None, None)
                .with_description("Source image to stylize with edge-following painterly smoothing."),
            // base sampling radius — neighborhood is (2r+1) x (2r+1) before the elliptical warp
            Input::new("radius".to_string(), Value::Integer(4), Some(InputSettings::Slider { range: (2.0, 16.0), step_by: Some(1.0), clamp_to_range: true }), None)
                .with_description("Base sampling radius in pixels; larger values give bigger brush strokes."),
            // sharpness q: exponent on per-sector variance when computing blend weights.
            // Higher = sharper edges and flatter regions (more posterised);
            // lower = softer transitions (more averaged).
            Input::new("sharpness".to_string(), Value::Decimal(8.0), Some(InputSettings::Slider { range: (1.0, 20.0), step_by: Some(0.5), clamp_to_range: true }), None)
                .with_description("Variance exponent controlling sector blending; higher values posterize flatter regions."),
            // alpha: how much to stretch the ellipse along the edge direction.
            // 0 → almost circular (≈ classic Kuwahara behavior at strong edges);
            // higher → ellipse follows edges more aggressively for longer brush strokes.
            Input::new("alpha".to_string(), Value::Decimal(1.0), Some(InputSettings::Slider { range: (0.1, 2.0), step_by: Some(0.05), clamp_to_range: true }), None)
                .with_description("Ellipse elongation along edges; higher values produce longer edge-following strokes."),
        ]
    }

    /// Creates the output port.
    pub fn create_outputs() -> Vec<Output> {
        vec![
            Output::new("output".to_string(), Value::Image { data: default_image(), change_id: get_id() }, None)
                .with_description("Painterly anisotropic-Kuwahara smoothed image that follows edge contours."),
        ]
    }

    /// Executes the anisotropic Kuwahara filter.
    pub async fn run(inputs: &mut [Input]) -> Result<OperationResponse, OperationError> {
        let start_time = Instant::now();
        let mut input_errors: Vec<(usize, String)> = vec![];

        // convert inputs
        let image_converted = convert_input(inputs, 0, ValueType::Image, &mut input_errors);
        let radius_converted = convert_input(inputs, 1, ValueType::Integer, &mut input_errors);
        let sharpness_converted = convert_input(inputs, 2, ValueType::Decimal, &mut input_errors);
        let alpha_converted = convert_input(inputs, 3, ValueType::Decimal, &mut input_errors);

        // return if error
        if !input_errors.is_empty() { return Err(OperationError { input_errors, node_error: None }); }

        // get values
        let Value::Image { data, change_id: _ } = image_converted.unwrap() else { unreachable!() };
        let Value::Integer(radius) = radius_converted.unwrap() else { unreachable!() };
        let Value::Decimal(sharpness) = sharpness_converted.unwrap() else { unreachable!() };
        let Value::Decimal(alpha) = alpha_converted.unwrap() else { unreachable!() };

        let radius = radius.max(2);
        let q = sharpness.max(1.0);
        let alpha = alpha.max(0.1);

        let (width, height) = data.dimensions();
        let w = width as usize;
        let h = height as usize;
        let n = w * h;
        let ch = data.channels() as usize;
        let has_alpha = ch == 2 || ch == 4;
        let color_ch = if has_alpha { ch - 1 } else { ch };

        // ---- Step 1: luminance buffer ----
        // Per-row in parallel: cheap operation but every later step depends on
        // it, so getting it off the main thread shaves real time on big images.
        let data_ref = &data;
        let luminance: Vec<f32> = (0..h).into_par_iter().flat_map_iter(|y| {
            (0..w).map(move |x| {
                let p = data_ref.get_pixel(x as u32, y as u32);
                if color_ch >= 3 {
                    0.2126 * p[0] + 0.7152 * p[1] + 0.0722 * p[2]
                } else {
                    p[0]
                }
            })
        }).collect();

        // ---- Step 2: structure tensor ----
        // Form per-pixel J = [[Gx², GxGy], [GxGy, Gy²]] using Sobel gradients on
        // luminance, then smooth the three components with a small box blur.
        // The smoothing is what turns per-pixel noise into a useful local
        // orientation estimate.
        let mut jxx = vec![0.0f32; n];
        let mut jyy = vec![0.0f32; n];
        let mut jxy = vec![0.0f32; n];
        let lum_ref = &luminance;
        // Per-row parallel Sobel — clamp samples at row/column edges by hand
        // (cheap branch, no lambda capture across threads needed).
        jxx.par_chunks_mut(w)
            .zip(jyy.par_chunks_mut(w))
            .zip(jxy.par_chunks_mut(w))
            .enumerate()
            .for_each(|(y, ((jxx_row, jyy_row), jxy_row))| {
                let y_i = y as i32;
                let h_i = h as i32;
                let w_i = w as i32;
                let row_up = y_i.saturating_sub(1).clamp(0, h_i - 1) as usize;
                let row_dn = (y_i + 1).clamp(0, h_i - 1) as usize;
                for x in 0..w {
                    let x_i = x as i32;
                    let cx_l = x_i.saturating_sub(1).clamp(0, w_i - 1) as usize;
                    let cx_r = (x_i + 1).clamp(0, w_i - 1) as usize;
                    let l_up = lum_ref[row_up * w + cx_l];
                    let c_up = lum_ref[row_up * w + x];
                    let r_up = lum_ref[row_up * w + cx_r];
                    let l_md = lum_ref[y * w + cx_l];
                    let r_md = lum_ref[y * w + cx_r];
                    let l_dn = lum_ref[row_dn * w + cx_l];
                    let c_dn = lum_ref[row_dn * w + x];
                    let r_dn = lum_ref[row_dn * w + cx_r];
                    let gx = -l_up - 2.0 * l_md - l_dn + r_up + 2.0 * r_md + r_dn;
                    let gy = -l_up - 2.0 * c_up - r_up + l_dn + 2.0 * c_dn + r_dn;
                    jxx_row[x] = gx * gx;
                    jyy_row[x] = gy * gy;
                    jxy_row[x] = gx * gy;
                }
            });
        // smooth tensor components with a small box blur (radius 2 is plenty)
        let tensor_smooth_r = 2usize;
        let jxx_s = box_blur_2d(&jxx, w, h, tensor_smooth_r);
        let jyy_s = box_blur_2d(&jyy, w, h, tensor_smooth_r);
        let jxy_s = box_blur_2d(&jxy, w, h, tensor_smooth_r);

        // ---- Step 3: per-pixel orientation phi and anisotropy A ----
        // For a 2x2 symmetric matrix [[a, b], [b, c]] the eigenvalues are
        // (a+c ± sqrt((a-c)² + 4b²)) / 2 and the major-eigenvector orientation
        // is 0.5 * atan2(2b, a-c). Anisotropy is (lambda1 - lambda2) /
        // (lambda1 + lambda2), normalised to [0, 1].
        let (phi, anis): (Vec<f32>, Vec<f32>) = (0..n).into_par_iter().map(|i| {
            let a = jxx_s[i];
            let b = jxy_s[i];
            let c = jyy_s[i];
            let trace = a + c;
            let disc = ((a - c) * (a - c) + 4.0 * b * b).sqrt();
            let l1 = 0.5 * (trace + disc);
            let l2 = 0.5 * (trace - disc);
            // Edge direction is perpendicular to the dominant gradient — we add
            // π/2 so phi runs ALONG the edge (which is what the elliptical
            // sampling expects to stretch into).
            let phi_i = 0.5 * (2.0 * b).atan2(a - c) + std::f32::consts::FRAC_PI_2;
            let denom = l1 + l2;
            let anis_i = if denom > 1e-8 { ((l1 - l2) / denom).clamp(0.0, 1.0) } else { 0.0 };
            (phi_i, anis_i)
        }).unzip();

        // ---- Step 4: precompute sector weight LUT ----
        // K_i(dx, dy) = G(dx, dy) * w_i(theta(dx, dy))
        //   G is a Gaussian envelope in the canonical (pre-warp) frame
        //   w_i is a smooth angular wedge centered on sector i
        // Stored as Vec<Vec<f32>> indexed [sector][offset_index].
        let diameter = (2 * radius + 1) as usize;
        let kernel_n = diameter * diameter;
        let sigma = radius as f32 * 0.5;
        let two_sigma_sq = 2.0 * sigma * sigma;
        let mut sector_weights: Vec<Vec<f32>> = vec![vec![0.0f32; kernel_n]; SECTORS];
        for (s, weights) in sector_weights.iter_mut().enumerate().take(SECTORS) {
            let center_angle = (s as f32 + 0.5) * (std::f32::consts::TAU / SECTORS as f32);
            for dy in -radius..=radius {
                for dx in -radius..=radius {
                    let off = ((dy + radius) as usize) * diameter + (dx + radius) as usize;
                    let dist2 = (dx * dx + dy * dy) as f32;
                    let radial = (-dist2 / two_sigma_sq).exp();
                    if dx == 0 && dy == 0 {
                        // center pixel contributes a small uniform weight to every
                        // sector so it's never starved of samples
                        weights[off] = radial / SECTORS as f32;
                        continue;
                    }
                    let theta = (dy as f32).atan2(dx as f32);
                    // smooth wedge: cos²(N/2 * Δθ) clamped to positive lobe.
                    // adjacent sectors overlap so the total weight per offset
                    // sums to a smooth function of angle, avoiding seams.
                    let mut delta = theta - center_angle;
                    while delta > std::f32::consts::PI { delta -= std::f32::consts::TAU; }
                    while delta < -std::f32::consts::PI { delta += std::f32::consts::TAU; }
                    // Use N/4 so adjacent wedges are 90° out-of-phase in cos²
                    // — that gives a partition of unity (cos² + sin² = 1) and
                    // the total angular weight is constant for every direction.
                    // N/2 would give non-overlapping wedges and visible
                    // rotation-dependent banding in flat regions.
                    let arg = (SECTORS as f32) * 0.25 * delta;
                    let wedge = if arg.abs() < std::f32::consts::FRAC_PI_2 {
                        let c = arg.cos();
                        c * c
                    } else {
                        0.0
                    };
                    weights[off] = radial * wedge;
                }
            }
        }

        // ---- Step 5: per-pixel filter ----
        // For each output pixel: build the elliptical sampling frame from this
        // pixel's phi/anisotropy, then iterate the canonical (dx, dy) grid,
        // bilinear-sample the rotated/scaled position, and accumulate per-sector
        // weighted sums (per-channel mean) plus weighted squared-luminance
        // sums (for variance).
        let data_ref = &data;
        let phi_ref = &phi;
        let anis_ref = &anis;
        let kernel_ref = &sector_weights;

        let pixels: Vec<f32> = (0..h as i32).into_par_iter().flat_map_iter(move |y| {
            let mut row_pixels = Vec::with_capacity(w * ch);
            // per-thread sample buffer to avoid allocations inside the hot loop
            let mut sample = vec![0.0f32; ch];
            // per-sector accumulators
            let mut sums = vec![0.0f64; SECTORS * ch];
            let mut sum_lum = [0.0f64; SECTORS];
            let mut sumsq_lum = [0.0f64; SECTORS];
            let mut wsum = [0.0f64; SECTORS];

            for x in 0..w as i32 {
                let i = y as usize * w + x as usize;

                // elliptical scale factors. Along the edge direction we stretch
                // ((alpha + A) / alpha) so the filter samples further along the
                // edge; perpendicular we squeeze (alpha / (alpha + A)) so it
                // doesn't reach across the edge. When A → 0, both factors → 1
                // (circle); when A → 1 the ellipse is markedly elongated along
                // the edge — exactly what makes the filter "follow" edges.
                let a = anis_ref[i];
                let scale_along = (alpha + a) / alpha;
                let scale_perp = alpha / (alpha + a);
                let cos_p = phi_ref[i].cos();
                let sin_p = phi_ref[i].sin();

                // reset accumulators
                for v in sums.iter_mut() { *v = 0.0; }
                for v in sum_lum.iter_mut() { *v = 0.0; }
                for v in sumsq_lum.iter_mut() { *v = 0.0; }
                for v in wsum.iter_mut() { *v = 0.0; }

                for dy in -radius..=radius {
                    for dx in -radius..=radius {
                        let off = ((dy + radius) as usize) * diameter + (dx + radius) as usize;

                        // canonical (dx, dy) → image-space sample position via
                        // (rotate by phi) ∘ (scale by ellipse axes).
                        // scale_along sits on the edge direction (cos_p, sin_p);
                        // scale_perp sits on the perpendicular (-sin_p, cos_p).
                        let along_x = dx as f32 * scale_along;
                        let perp_y = dy as f32 * scale_perp;
                        let sx = x as f32 + along_x * cos_p - perp_y * sin_p;
                        let sy = y as f32 + along_x * sin_p + perp_y * cos_p;
                        data_ref.bilinear_sample(sx, sy, &mut sample);

                        // luminance of the sampled pixel for variance bookkeeping
                        let s_lum = if color_ch >= 3 {
                            0.2126 * sample[0] + 0.7152 * sample[1] + 0.0722 * sample[2]
                        } else {
                            sample[0]
                        };

                        // accumulate this sample into every sector with its
                        // precomputed weight
                        for s in 0..SECTORS {
                            let kw = kernel_ref[s][off] as f64;
                            if kw == 0.0 { continue; }
                            for c in 0..ch {
                                sums[s * ch + c] += sample[c] as f64 * kw;
                            }
                            sum_lum[s] += s_lum as f64 * kw;
                            sumsq_lum[s] += (s_lum as f64).powi(2) * kw;
                            wsum[s] += kw;
                        }
                    }
                }

                // combine sector means by 1 / (variance^q + eps) weights — the
                // sector with the lowest luminance variance dominates.
                let eps = 1e-8f64;
                let mut numer = vec![0.0f64; ch];
                let mut denom = 0.0f64;
                for s in 0..SECTORS {
                    if wsum[s] < 1e-12 { continue; }
                    let inv_w = 1.0 / wsum[s];
                    let mean_lum = sum_lum[s] * inv_w;
                    let var_lum = (sumsq_lum[s] * inv_w - mean_lum * mean_lum).max(0.0);
                    let blend_w = 1.0 / (var_lum.powf(q as f64) + eps);
                    for c in 0..ch {
                        numer[c] += sums[s * ch + c] * inv_w * blend_w;
                    }
                    denom += blend_w;
                }
                let inv_d = if denom > 0.0 { 1.0 / denom } else { 0.0 };
                for val in numer.iter().take(ch) {
                    row_pixels.push((val * inv_d).clamp(0.0, 1.0) as f32);
                }
            }
            row_pixels
        }).collect();

        let output = FloatImage::from_raw(width, height, data.channels(), pixels).unwrap();

        Ok(OperationResponse {
            time: Instant::now().duration_since(start_time),
            responses: vec![
                OutputResponse { value: Value::Image { data: Arc::new(output), change_id: get_id() } },
            ],
        })
    }
}

/// Separable 2D box blur with edge clamping. O(1) per pixel via 1D prefix sums.
/// (Same primitive used in `guided.rs` and `toon.rs` — duplicated here to keep
/// each filter self-contained; extract to a shared module if a fourth filter
/// wants it.)
fn box_blur_2d(input: &[f32], width: usize, height: usize, radius: usize) -> Vec<f32> {
    if width == 0 || height == 0 { return Vec::new(); }

    let mut h_pass = vec![0.0f32; input.len()];
    let mut prefix = vec![0.0f64; width + 1];
    for y in 0..height {
        let row_start = y * width;
        prefix[0] = 0.0;
        for x in 0..width {
            prefix[x + 1] = prefix[x] + input[row_start + x] as f64;
        }
        for x in 0..width {
            let lo = x.saturating_sub(radius);
            let hi = (x + radius + 1).min(width);
            let cnt = (hi - lo) as f64;
            h_pass[row_start + x] = ((prefix[hi] - prefix[lo]) / cnt) as f32;
        }
    }

    let mut out = vec![0.0f32; input.len()];
    let mut col_prefix = vec![0.0f64; height + 1];
    for x in 0..width {
        col_prefix[0] = 0.0;
        for y in 0..height {
            col_prefix[y + 1] = col_prefix[y] + h_pass[y * width + x] as f64;
        }
        for y in 0..height {
            let lo = y.saturating_sub(radius);
            let hi = (y + radius + 1).min(height);
            let cnt = (hi - lo) as f64;
            out[y * width + x] = ((col_prefix[hi] - col_prefix[lo]) / cnt) as f32;
        }
    }
    out
}

#[cfg(test)]
#[path = "anisotropic_kuwahara_tests.rs"]
mod tests;
