//! Non-Local Means (NLM) denoising.
//!
//! Proposed by Buades, Coll & Morel (2005). Each pixel is replaced with a
//! weighted average of other pixels in a search window, where the weight is
//! determined by how similar the *patches* around the two pixels are — not
//! just their spatial proximity. This lets NLM preserve repeating texture
//! and subtle details that bilateral/guided filters tend to smear.
//!
//! For pixel p and candidate q within a search window:
//!     `w(p, q) = exp( -||P(p) - P(q)||² / h² )`
//! where `P(p)` is a small patch around p and `h` controls filter strength.
//!
//! Patch SSDs are evaluated with a per-offset integral image of squared
//! differences, so cost is O(W²) per pixel independent of patch size. The
//! search radius is still capped in the UI.

use crate::float_image::FloatImage;
use crate::get_id;
use crate::input::{Input, InputSettings};
use crate::node_settings::NodeSettings;
use crate::operations::{OperationResponse, OperationError, OutputResponse, default_image, convert_input};
use crate::output::Output;
use crate::value::{Value, ValueType};
use rayon::prelude::*;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use std::time::Instant;

/// Non-Local Means denoiser.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpImageAdjustmentNonLocalMeans {}

impl OpImageAdjustmentNonLocalMeans {
    /// Returns the node metadata (name and description) for NLM.
    pub fn settings() -> NodeSettings {
        NodeSettings {
            name: "non local means".to_string(),
            description: "Non-Local Means denoising — weights neighbors by patch similarity rather than spatial distance.".to_string(),
            help: "Buades, Coll and Morel 2005. For each pixel p and every candidate q in a search window, weights q by `exp(-||patch(p) - patch(q)||^2 / h^2)` where patches are small windows around each pixel. The output is the weighted average over q, so repeating textures reinforce each other while noise averages to zero.\n\nSuperior to bilateral/guided at preserving fine repeating detail. Patch SSDs are computed with per-offset integral images, so cost is O(W^2) per pixel with W = search radius, independent of patch size. Rows run in parallel; smaller `strength` keeps detail, larger strength smooths harder.".to_string(),
        }
    }

    /// Creates input ports: image, search-window radius, patch radius, and
    /// filter strength h (larger = more smoothing).
    pub fn create_inputs() -> Vec<Input> {
        vec![
            Input::new("image".to_string(), Value::Image { data: default_image(), change_id: get_id() }, None, None)
                .with_description("Source image to denoise using patch-similarity weighted averaging."),
            // search window radius — how far NLM looks for similar patches
            Input::new("search radius".to_string(), Value::Integer(3), Some(InputSettings::Slider { range: (1.0, 8.0), step_by: Some(1.0), clamp_to_range: true }), None)
                .with_description("Half-size of the search window; larger values consider patches farther away."),
            // patch radius — size of the neighborhood used for similarity
            Input::new("patch radius".to_string(), Value::Integer(1), Some(InputSettings::Slider { range: (0.0, 4.0), step_by: Some(1.0), clamp_to_range: true }), None)
                .with_description("Half-size of the comparison patch; larger values weigh broader context in the match."),
            // filter strength h; small h = sharp but noisy, large h = smooth but blurry
            Input::new("strength".to_string(), Value::Decimal(0.1), Some(InputSettings::Slider { range: (0.001, 1.0), step_by: Some(0.001), clamp_to_range: true }), None)
                .with_description("Denoising strength h; smaller values keep more detail, larger values smooth more."),
        ]
    }

    /// Creates the output port: the denoised image.
    pub fn create_outputs() -> Vec<Output> {
        vec![
            Output::new("output".to_string(), Value::Image { data: default_image(), change_id: get_id() }, None)
                .with_description("Non-Local Means denoised image with repeating texture preserved."),
        ]
    }

    /// Runs the Non-Local Means denoiser.
    pub async fn run(inputs: &mut [Input]) -> Result<OperationResponse, OperationError> {
        let start_time = Instant::now();
        let mut input_errors: Vec<(usize, String)> = vec![];

        let image_converted = convert_input(inputs, 0, ValueType::Image, &mut input_errors);
        let search_converted = convert_input(inputs, 1, ValueType::Integer, &mut input_errors);
        let patch_converted = convert_input(inputs, 2, ValueType::Integer, &mut input_errors);
        let h_converted = convert_input(inputs, 3, ValueType::Decimal, &mut input_errors);

        if !input_errors.is_empty() { return Err(OperationError { input_errors, node_error: None }); }

        let Value::Image { data, change_id: _ } = image_converted.unwrap() else { unreachable!() };
        let Value::Integer(search_r) = search_converted.unwrap() else { unreachable!() };
        let Value::Integer(patch_r) = patch_converted.unwrap() else { unreachable!() };
        let Value::Decimal(h) = h_converted.unwrap() else { unreachable!() };

        let search_r = search_r.max(1);
        let patch_r = patch_r.max(0);
        // Guard against division by zero in the exponent
        let h2 = (h * h).max(1e-8);

        let (width, height) = data.dimensions();
        let ch = data.channels() as usize;
        let w = width as i32;
        let h_i = height as i32;
        // Patch similarity is computed over color channels only, ignoring alpha
        let color_ch = if ch == 2 || ch == 4 { ch - 1 } else { ch };
        // Normalize sum-of-squared-differences by patch area so `h` behaves
        // consistently across different patch sizes
        let patch_area = ((2 * patch_r + 1) * (2 * patch_r + 1)) as f32;
        let inv_norm = 1.0 / (patch_area * color_ch as f32);

        let data_ref = &data;
        let n_pix = width as usize * height as usize;
        let p = patch_r as usize;

        // Weighted-average accumulators, filled one search offset at a time.
        let mut acc = vec![0.0f32; n_pix * ch];
        let mut weight_sum = vec![0.0f32; n_pix];

        // Integral image of squared differences between the image and its
        // (dx, dy)-shifted copy, built over an extended domain that covers
        // every clamped patch sample: u in [-patch_r, w-1+patch_r] (and the
        // same for v). With it, each pixel's patch SSD becomes an O(1) box
        // lookup, making the filter O(search²) per pixel independent of
        // patch size.
        let ew = w as usize + 2 * p;
        let eh = h_i as usize + 2 * p;
        let stride = ew + 1;
        let mut integral = vec![0.0f64; stride * (eh + 1)]; // row 0 / col 0 stay zero

        for dy in -search_r..=search_r {
            for dx in -search_r..=search_r {
                // 1) Row pass (parallel): per-row prefix sums of the squared
                //    color difference, with both samples clamped exactly like
                //    the direct implementation did.
                integral[stride..].par_chunks_mut(stride).enumerate().for_each(|(ev, out_row)| {
                    let v = ev as i32 - patch_r;
                    let sy = v.clamp(0, h_i - 1) as u32;
                    let ty = (v + dy).clamp(0, h_i - 1) as u32;
                    let mut run = 0.0f64;
                    out_row[0] = 0.0;
                    for (eu, out) in out_row.iter_mut().skip(1).enumerate() {
                        let u = eu as i32 - patch_r;
                        let sx = u.clamp(0, w - 1) as u32;
                        let tx = (u + dx).clamp(0, w - 1) as u32;
                        let sp = data_ref.get_pixel(sx, sy);
                        let tp = data_ref.get_pixel(tx, ty);
                        let mut d2 = 0.0f32;
                        for c in 0..color_ch {
                            let d = sp[c] - tp[c];
                            d2 += d * d;
                        }
                        run += d2 as f64;
                        *out = run;
                    }
                });
                // 2) Column pass: accumulate rows top-to-bottom to finish the
                //    integral image (cheap vectorized adds).
                for ev in 1..=eh {
                    let (prev, cur) = integral.split_at_mut(ev * stride);
                    let prev_row = &prev[(ev - 1) * stride..];
                    for (c_val, p_val) in cur[..stride].iter_mut().zip(prev_row.iter()) {
                        *c_val += *p_val;
                    }
                }
                // 3) Accumulate this offset's weighted contribution (parallel
                //    over rows). Candidates outside the image are skipped,
                //    matching the previous search-window bounds check.
                let integral_ref = &integral;
                acc.par_chunks_mut(w as usize * ch)
                    .zip(weight_sum.par_chunks_mut(w as usize))
                    .enumerate()
                    .for_each(|(y, (acc_row, wsum_row))| {
                        let y = y as i32;
                        let qy = y + dy;
                        if qy < 0 || qy >= h_i { return; }
                        // Patch rows for pixel y span extended rows [y, y + 2p]
                        let y0 = y as usize;
                        let y1 = y as usize + 2 * p + 1;
                        for x in 0..w {
                            let qx = x + dx;
                            if qx < 0 || qx >= w { continue; }
                            let x0 = x as usize;
                            let x1 = x as usize + 2 * p + 1;
                            let ssd = (integral_ref[y1 * stride + x1] - integral_ref[y0 * stride + x1]
                                - integral_ref[y1 * stride + x0] + integral_ref[y0 * stride + x0])
                                .max(0.0) as f32 * inv_norm;
                            let weight = (-ssd / h2).exp();
                            let qp = data_ref.get_pixel(qx as u32, qy as u32);
                            let base = x as usize * ch;
                            for c in 0..ch {
                                acc_row[base + c] += weight * qp[c];
                            }
                            wsum_row[x as usize] += weight;
                        }
                    });
            }
        }

        // Normalize by total weight (guaranteed > 0: weight at q = p is 1)
        let mut pixels = acc;
        pixels.par_chunks_mut(ch).zip(weight_sum.par_iter()).for_each(|(px, &ws)| {
            for val in px.iter_mut() {
                *val /= ws;
            }
        });

        let out = FloatImage::from_raw(width, height, data.channels(), pixels).unwrap();

        Ok(OperationResponse {
            time: Instant::now().duration_since(start_time),
            responses: vec![
                OutputResponse { value: Value::Image { data: Arc::new(out), change_id: get_id() } },
            ],
        })
    }
}

#[cfg(test)]
#[path = "non_local_means_tests.rs"]
mod tests;
