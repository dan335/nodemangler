//! Anisotropic (Perona–Malik) diffusion.
//!
//! Iterative edge-preserving smoothing introduced by Perona & Malik (1990).
//! At each step, every pixel drifts toward its neighbors with a conductance
//! that depends on the local gradient: large gradients (edges) get low
//! conductance and are preserved, small gradients (texture, noise) get high
//! conductance and are smoothed.
//!
//! Update rule per iteration, evaluated in 4-neighborhood (N, S, E, W):
//!     `I_{t+1}(p) = I_t(p) + λ · Σ_d c(∇I_d) · ∇I_d`
//! where the edge-stopping function `c` is the Perona–Malik "quadratic"
//! variant:
//!     `c(x) = 1 / (1 + (x / κ)²)`
//! — this variant is more stable than the exponential form for integer
//! iterations and small κ.

use crate::float_image::FloatImage;
use crate::get_id;
use crate::input::{Input, InputSettings};
use crate::node_settings::NodeSettings;
use crate::convert_inputs;
use crate::operations::{OperationResponse, OperationError, OutputResponse, image_input, image_output};
use crate::output::Output;
use crate::value::Value;
use rayon::prelude::*;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use std::time::Instant;

/// Anisotropic (Perona–Malik) diffusion filter.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpImageAdjustmentAnisotropicDiffusion {}

impl OpImageAdjustmentAnisotropicDiffusion {
    /// Returns the node metadata for the anisotropic diffusion filter.
    pub fn settings() -> NodeSettings {
        NodeSettings {
            name: "anisotropic diffusion".to_string(),
            description: "Perona–Malik iterative edge-preserving smoothing. Flattens texture while preserving edges.".to_string(),
            help: "Iterative PDE-based smoothing from Perona and Malik (1990). Each iteration nudges every pixel toward its 4-neighbors with a conductance `c(x) = 1 / (1 + (x/kappa)^2)` that shrinks near strong gradients, so edges stay crisp while textures and noise diffuse away.\n\nKappa is the edge-stopping threshold: gradients much larger than kappa are preserved. Lambda must stay at or below 0.25 for stability with a 4-point stencil. Alpha is copied through unchanged; edges are clamped.".to_string(),
        }
    }

    /// Creates input ports: image, iteration count, edge-stopping threshold κ,
    /// and per-step rate λ (≤ 0.25 for stability with a 4-neighborhood).
    pub fn create_inputs() -> Vec<Input> {
        vec![
            image_input("image")
                .with_description("Source image to smooth while preserving edges."),
            // number of diffusion iterations — more iterations = more smoothing
            Input::new("iterations".to_string(), Value::Integer(10), Some(InputSettings::Slider { range: (1.0, 100.0), step_by: Some(1.0), clamp_to_range: true }), None)
                .with_description("Number of diffusion steps; more iterations produce stronger smoothing."),
            // κ: gradient magnitudes much larger than κ are preserved as edges
            Input::new("kappa".to_string(), Value::Decimal(0.1), Some(InputSettings::Slider { range: (0.01, 1.0), step_by: Some(0.01), clamp_to_range: true }), None)
                .with_description("Edge-stopping threshold; gradients much larger than this are preserved as edges."),
            // λ: step size, must be ≤ 0.25 for stability with a 4-point stencil
            Input::new("lambda".to_string(), Value::Decimal(0.2), Some(InputSettings::Slider { range: (0.01, 0.25), step_by: Some(0.01), clamp_to_range: true }), None)
                .with_description("Per-iteration step size; must stay at or below 0.25 for stability."),
        ]
    }

    /// Creates the output port: the diffused image.
    pub fn create_outputs() -> Vec<Output> {
        vec![
            image_output("output")
                .with_description("Edge-preserving smoothed image after the diffusion iterations."),
        ]
    }

    /// Runs anisotropic diffusion for the configured number of iterations.
    pub async fn run(inputs: &mut [Input]) -> Result<OperationResponse, OperationError> {
        let start_time = Instant::now();

        convert_inputs! { inputs;
            Image(data) = 0,
            Integer(iterations) = 1,
            Decimal(kappa) = 2,
            Decimal(lambda) = 3,
        }

        let iterations = iterations.max(1) as usize;
        let kappa = kappa.max(1e-6);
        // Clamp λ to the stable 4-neighborhood range [0, 0.25]
        let lambda = lambda.clamp(0.0, 0.25);
        let inv_k2 = 1.0 / (kappa * kappa);

        let (width, height) = data.dimensions();
        let ch = data.channels() as usize;
        let w = width as i32;
        let h = height as i32;
        // Diffuse color channels only; alpha is copied through unchanged
        let color_ch = if ch == 2 || ch == 4 { ch - 1 } else { ch };

        // Working buffers: flat interleaved f32, same layout as FloatImage::as_raw
        let mut buf: Vec<f32> = data.as_raw().to_vec();
        let mut next = buf.clone();

        // Each Jacobi iteration reads only from `buf` and writes only to
        // `next`, so the per-iteration pass is row-parallel; the iteration
        // loop itself must stay sequential.
        let wu = width as usize;
        let row_len = wu * ch;
        for _ in 0..iterations {
            let buf_ref = &buf;
            next.par_chunks_mut(row_len).enumerate().for_each(|(y, next_row)| {
                let y = y as i32;
                for x in 0..w {
                    let idx = (y as usize * wu + x as usize) * ch;
                    // Neighbor indices with edge clamping
                    let xn = (x - 1).max(0) as usize;
                    let xp = (x + 1).min(w - 1) as usize;
                    let yn = (y - 1).max(0) as usize;
                    let yp = (y + 1).min(h - 1) as usize;
                    let idx_n = (yn * wu + x as usize) * ch;
                    let idx_s = (yp * wu + x as usize) * ch;
                    let idx_e = (y as usize * wu + xp) * ch;
                    let idx_we = (y as usize * wu + xn) * ch;
                    let out_idx = x as usize * ch;

                    for c in 0..color_ch {
                        let v = buf_ref[idx + c];
                        let dn = buf_ref[idx_n + c] - v;
                        let ds = buf_ref[idx_s + c] - v;
                        let de = buf_ref[idx_e + c] - v;
                        let dw = buf_ref[idx_we + c] - v;

                        // Perona–Malik "quadratic" edge-stopping function
                        let cn = 1.0 / (1.0 + dn * dn * inv_k2);
                        let cs = 1.0 / (1.0 + ds * ds * inv_k2);
                        let ce = 1.0 / (1.0 + de * de * inv_k2);
                        let cw = 1.0 / (1.0 + dw * dw * inv_k2);

                        next_row[out_idx + c] = v + lambda * (cn * dn + cs * ds + ce * de + cw * dw);
                    }
                    // Copy alpha unchanged (not diffused)
                    if ch == 2 || ch == 4 {
                        next_row[out_idx + ch - 1] = buf_ref[idx + ch - 1];
                    }
                }
            });
            std::mem::swap(&mut buf, &mut next);
        }

        let out = FloatImage::from_raw(width, height, data.channels(), buf).unwrap();

        Ok(OperationResponse {
            time: Instant::now().duration_since(start_time),
            responses: vec![
                OutputResponse { value: Value::Image { data: Arc::new(out), change_id: get_id() } },
            ],
        })
    }
}

#[cfg(test)]
#[path = "anisotropic_diffusion_tests.rs"]
mod tests;
