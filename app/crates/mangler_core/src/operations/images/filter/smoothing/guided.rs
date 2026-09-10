//! Guided filter operation for images (He et al. 2010).
//!
//! Edge-preserving smoothing whose cost is O(1) per pixel regardless of
//! radius, by expressing the filter as a fixed number of box blurs of the
//! input and intermediate maps. Excellent for large-radius smoothing.
//!
//! This is the *self-guided* form: the input image is its own guide. For
//! each color channel of the input, the filter computes a locally-linear
//! coefficient (`a`, `b`) such that the smoothed output is approximately
//! `a * I + b` where `I` is the guide luminance. The smoothness of `a` and
//! `b` (themselves box-blurred) is what produces the edge-preserving effect:
//! the linear model can change quickly across edges in `I`, so edges aren't
//! averaged across.
//!
//! Aesthetically: smoother and more "denoised photo" than Kuwahara's
//! painterly look — closer to bilateral, but with cost independent of radius.

use crate::operations::images::blur::blur::box_blur_2d;
use crate::float_image::FloatImage;
use crate::get_id;
use crate::input::{Input, InputSettings};
use crate::node_settings::NodeSettings;
use crate::convert_inputs;
use crate::operations::{OperationResponse, OperationError, OutputResponse, scale_to_resolution, image_input, image_output};
use crate::output::Output;
use crate::value::Value;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use std::time::Instant;

/// Guided filter (He et al.) — edge-preserving, O(1) per pixel.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpImageAdjustmentGuided {}

impl OpImageAdjustmentGuided {
    /// Returns the node metadata (name and description) for the guided filter.
    pub fn settings() -> NodeSettings {
        NodeSettings {
            name: "guided filter".to_string(),
            description: "Edge-preserving smoothing (He et al.). Self-guided; O(1) per pixel regardless of radius.".to_string(),
            help: "He et al. 2010 guided filter in self-guided form: luminance acts as its own guide. For each color channel it fits a local linear model `q = a*I + b` whose coefficients are derived from box-blurred statistics (mean, variance, covariance), then smooths `a` and `b` with another box blur before applying.\n\nCost is independent of radius because the whole pipeline reduces to a fixed number of box blurs implemented with prefix sums. Smaller epsilon keeps edges sharper; larger epsilon smooths more aggressively. Alpha is passed through.".to_string(),
        }
    }

    /// Creates the input ports: image, radius, and epsilon (edge sensitivity).
    pub fn create_inputs() -> Vec<Input> {
        vec![
            image_input("image")
                .with_description("Source image to smooth while preserving edges; also acts as its own guide."),
            // box-blur radius — cost is independent of this thanks to prefix sums, so we allow large values
            Input::new("radius".to_string(), Value::Integer(8), Some(InputSettings::Slider { range: (1.0, 64.0), step_by: Some(1.0), clamp_to_range: true }), None)
                .with_description("Window radius for the internal box blurs, in pixels at a 1024px reference (scales with image size); larger values smooth over broader areas."),
            // epsilon controls how aggressively edges are preserved: smaller values keep more detail
            // (sharper edges, less smoothing); larger values smooth more aggressively across edges
            Input::new("epsilon".to_string(), Value::Decimal(0.01), Some(InputSettings::Slider { range: (0.0001, 1.0), step_by: Some(0.001), clamp_to_range: true }), None)
                .with_description("Edge-preservation regularizer; smaller values keep edges sharper."),
        ]
    }

    /// Creates the output port: the guided-filtered image.
    pub fn create_outputs() -> Vec<Output> {
        vec![
            image_output("output")
                .with_description("Edge-preserving guided-filter output."),
        ]
    }

    /// Executes the guided filter.
    pub async fn run(inputs: &mut [Input]) -> Result<OperationResponse, OperationError> {
        let start_time = Instant::now();

        convert_inputs! { inputs;
            Image(data) = 0,
            Integer(radius) = 1,
            Decimal(epsilon) = 2,
        }

        let epsilon = epsilon.max(1e-6);

        // Radius is authored in reference pixels (at 1024px) and scaled to the
        // actual image so the filter looks the same relative size at any resolution.
        let (width, height) = data.dimensions();
        let radius = scale_to_resolution(radius.max(1) as f32, width, height).round().max(1.0) as usize;
        let w = width as usize;
        let h = height as usize;
        let n = w * h;
        let ch = data.channels() as usize;
        let has_alpha = ch == 2 || ch == 4;
        let color_ch = if has_alpha { ch - 1 } else { ch };

        // Pull each channel into a flat f32 buffer for easy SIMD-friendly per-pixel math.
        // channels[c] is the c-th plane of the source image.
        let mut channels: Vec<Vec<f32>> = vec![vec![0.0; n]; ch];
        for (idx, pixel) in data.as_raw().chunks_exact(ch).enumerate() {
            for c in 0..ch {
                channels[c][idx] = pixel[c];
            }
        }

        // Guide is the luminance of the input (Rec. 709 for color, channel 0 for grayscale).
        // Using a scalar guide keeps the math simple and is the standard approach
        // for filtering color images with a self-derived guide.
        let mut guide = vec![0.0f32; n];
        for i in 0..n {
            guide[i] = if color_ch >= 3 {
                crate::luma::rec709(channels[0][i], channels[1][i], channels[2][i])
            } else {
                channels[0][i]
            };
        }

        // Guide statistics (mean_I, var_I) are shared across every channel, so
        // they're computed once here and reused for each plane.
        let stats = guide_stats(&guide, w, h, radius);

        // For each color channel, run the locally-linear fit and produce the
        // filtered output channel q = mean_a * I + mean_b (clamped for display).
        let mut filtered: Vec<Vec<f32>> = vec![vec![0.0; n]; ch];
        for c in 0..color_ch {
            let q = guided_filter_plane_with_stats(&channels[c], &guide, &stats, w, h, radius, epsilon);
            for i in 0..n {
                filtered[c][i] = q[i].clamp(0.0, 1.0);
            }
        }

        // alpha is passed through unchanged
        if has_alpha {
            filtered[ch - 1] = channels[ch - 1].clone();
        }

        // pack channels back into a single interleaved buffer for FloatImage
        let mut pixels = vec![0.0f32; n * ch];
        for i in 0..n {
            for c in 0..ch {
                pixels[i * ch + c] = filtered[c][i];
            }
        }
        let output = FloatImage::from_raw(width, height, data.channels(), pixels).unwrap();

        Ok(OperationResponse {
            time: Instant::now().duration_since(start_time),
            responses: vec![
                OutputResponse { value: Value::Image { data: Arc::new(output), change_id: get_id() } },
            ],
        })
    }
}

/// Box-blurred guide statistics: `mean_I` and `var_I` over the filter window.
///
/// These depend only on the guide, so a caller filtering several planes
/// against the same guide computes them once and reuses them.
pub(crate) struct GuideStats {
    /// Box blur of the guide.
    pub mean_i: Vec<f32>,
    /// Local variance of the guide, `boxblur(I*I) - mean_I^2`, floored at 0.
    pub var_i: Vec<f32>,
}

/// Computes [`GuideStats`] for `guide` at the given window radius.
pub(crate) fn guide_stats(guide: &[f32], width: usize, height: usize, radius: usize) -> GuideStats {
    let n = width * height;
    let mean_i = box_blur_2d(guide, width, height, radius);
    let ii: Vec<f32> = guide.iter().map(|v| v * v).collect();
    let mean_ii = box_blur_2d(&ii, width, height, radius);
    let var_i: Vec<f32> = (0..n).map(|i| (mean_ii[i] - mean_i[i] * mean_i[i]).max(0.0)).collect();
    GuideStats { mean_i, var_i }
}

/// Guided-filter core for a single plane (He et al. 2010).
///
/// Fits the local linear model `q = a*I + b` of plane `p` against scalar guide
/// `guide` (both `width * height` long, row-major), smooths the coefficients
/// with another box blur, and returns `q = mean_a * I + mean_b`. The result is
/// **not** clamped — callers targeting displayable images clamp to `[0,1]`
/// themselves.
///
/// `eps` is the edge-preservation regularizer: smaller keeps edges sharper.
pub(crate) fn guided_filter_plane(
    p: &[f32],
    guide: &[f32],
    width: usize,
    height: usize,
    radius: usize,
    eps: f32,
) -> Vec<f32> {
    let stats = guide_stats(guide, width, height, radius);
    guided_filter_plane_with_stats(p, guide, &stats, width, height, radius, eps)
}

/// [`guided_filter_plane`] with the guide statistics supplied by the caller,
/// so filtering several planes against one guide doesn't recompute them.
pub(crate) fn guided_filter_plane_with_stats(
    p: &[f32],
    guide: &[f32],
    stats: &GuideStats,
    width: usize,
    height: usize,
    radius: usize,
    eps: f32,
) -> Vec<f32> {
    let n = width * height;
    let mean_i = &stats.mean_i;
    let var_i = &stats.var_i;

    // mean_p, mean_Ip, cov_Ip
    let mean_p = box_blur_2d(p, width, height, radius);
    let ip: Vec<f32> = (0..n).map(|i| guide[i] * p[i]).collect();
    let mean_ip = box_blur_2d(&ip, width, height, radius);

    // a = cov_Ip / (var_I + eps), b = mean_p - a * mean_I
    let mut a = vec![0.0f32; n];
    let mut b = vec![0.0f32; n];
    for i in 0..n {
        let cov_ip = mean_ip[i] - mean_i[i] * mean_p[i];
        a[i] = cov_ip / (var_i[i] + eps);
        b[i] = mean_p[i] - a[i] * mean_i[i];
    }

    // smooth a and b before applying — this is what makes the output continuous
    let mean_a = box_blur_2d(&a, width, height, radius);
    let mean_b = box_blur_2d(&b, width, height, radius);

    (0..n).map(|i| mean_a[i] * guide[i] + mean_b[i]).collect()
}


#[cfg(test)]
#[path = "guided_tests.rs"]
mod tests;
