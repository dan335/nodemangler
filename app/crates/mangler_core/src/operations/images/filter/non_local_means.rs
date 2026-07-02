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
//! Cost is O(W² · P²) per pixel where W is the search-window size and P is
//! the patch size, so both are capped tightly in the UI.

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
            help: "Buades, Coll and Morel 2005. For each pixel p and every candidate q in a search window, weights q by `exp(-||patch(p) - patch(q)||^2 / h^2)` where patches are small windows around each pixel. The output is the weighted average over q, so repeating textures reinforce each other while noise averages to zero.\n\nSuperior to bilateral/guided at preserving fine repeating detail. Cost is O(W^2 * P^2) per pixel with W = search radius and P = patch radius, so both are capped tightly. Rows run in parallel; smaller `strength` keeps detail, larger strength smooths harder.".to_string(),
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

        let data_ref = &data;
        let pixels: Vec<f32> = (0..h_i).into_par_iter().flat_map_iter(move |y| {
            let mut row = Vec::with_capacity(w as usize * ch);
            for x in 0..w {
                let mut weight_sum = 0.0f32;
                let mut acc = [0.0f32; 4];

                // Iterate over the search window around (x, y)
                for dy in -search_r..=search_r {
                    for dx in -search_r..=search_r {
                        let qx = x + dx;
                        let qy = y + dy;
                        if qx < 0 || qy < 0 || qx >= w || qy >= h_i { continue; }

                        // Compute patch SSD between (x,y) and (qx,qy)
                        let mut ssd = 0.0f32;
                        for py in -patch_r..=patch_r {
                            for px in -patch_r..=patch_r {
                                let sx = (x + px).clamp(0, w - 1) as u32;
                                let sy = (y + py).clamp(0, h_i - 1) as u32;
                                let tx = (qx + px).clamp(0, w - 1) as u32;
                                let ty = (qy + py).clamp(0, h_i - 1) as u32;
                                let sp = data_ref.get_pixel(sx, sy);
                                let tp = data_ref.get_pixel(tx, ty);
                                for c in 0..color_ch {
                                    let d = sp[c] - tp[c];
                                    ssd += d * d;
                                }
                            }
                        }
                        ssd /= patch_area * color_ch as f32;

                        let weight = (-ssd / h2).exp();
                        let qp = data_ref.get_pixel(qx as u32, qy as u32);
                        for c in 0..ch {
                            acc[c] += weight * qp[c];
                        }
                        weight_sum += weight;
                    }
                }

                // Normalize by total weight (guaranteed > 0: weight at q = p is 1)
                for val in acc.iter().take(ch) {
                    row.push(val / weight_sum);
                }
            }
            row
        }).collect();

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
