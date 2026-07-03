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
            Input::new("image".to_string(), Value::Image { data: default_image(), change_id: get_id() }, None, None)
                .with_description("Source image to smooth while preserving edges; also acts as its own guide."),
            // box-blur radius — cost is independent of this thanks to prefix sums, so we allow large values
            Input::new("radius".to_string(), Value::Integer(8), Some(InputSettings::Slider { range: (1.0, 64.0), step_by: Some(1.0), clamp_to_range: true }), None)
                .with_description("Window radius for the internal box blurs; larger values smooth over broader areas."),
            // epsilon controls how aggressively edges are preserved: smaller values keep more detail
            // (sharper edges, less smoothing); larger values smooth more aggressively across edges
            Input::new("epsilon".to_string(), Value::Decimal(0.01), Some(InputSettings::Slider { range: (0.0001, 1.0), step_by: Some(0.001), clamp_to_range: true }), None)
                .with_description("Edge-preservation regularizer; smaller values keep edges sharper."),
        ]
    }

    /// Creates the output port: the guided-filtered image.
    pub fn create_outputs() -> Vec<Output> {
        vec![
            Output::new("output".to_string(), Value::Image { data: default_image(), change_id: get_id() }, None)
                .with_description("Edge-preserving guided-filter output."),
        ]
    }

    /// Executes the guided filter.
    pub async fn run(inputs: &mut [Input]) -> Result<OperationResponse, OperationError> {
        let start_time = Instant::now();
        let mut input_errors: Vec<(usize, String)> = vec![];

        // convert inputs
        let image_converted = convert_input(inputs, 0, ValueType::Image, &mut input_errors);
        let radius_converted = convert_input(inputs, 1, ValueType::Integer, &mut input_errors);
        let epsilon_converted = convert_input(inputs, 2, ValueType::Decimal, &mut input_errors);

        // return if error
        if !input_errors.is_empty() { return Err(OperationError { input_errors, node_error: None }); }

        // get values
        let Value::Image { data, change_id: _ } = image_converted.unwrap() else { unreachable!() };
        let Value::Integer(radius) = radius_converted.unwrap() else { unreachable!() };
        let Value::Decimal(epsilon) = epsilon_converted.unwrap() else { unreachable!() };

        let radius = radius.max(1) as usize;
        let epsilon = epsilon.max(1e-6);

        let (width, height) = data.dimensions();
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
                0.2126 * channels[0][i] + 0.7152 * channels[1][i] + 0.0722 * channels[2][i]
            } else {
                channels[0][i]
            };
        }

        // Precompute guide statistics that are shared across all channels:
        //   mean_I = boxblur(I)
        //   mean_II = boxblur(I * I)
        //   var_I = mean_II - mean_I^2
        let mean_i = box_blur_2d(&guide, w, h, radius);
        let ii: Vec<f32> = guide.iter().map(|v| v * v).collect();
        let mean_ii = box_blur_2d(&ii, w, h, radius);
        let var_i: Vec<f32> = (0..n).map(|i| (mean_ii[i] - mean_i[i] * mean_i[i]).max(0.0)).collect();

        // For each color channel, compute the locally-linear coefficients (a, b)
        // and produce the filtered output channel q = mean_a * I + mean_b.
        let mut filtered: Vec<Vec<f32>> = vec![vec![0.0; n]; ch];
        for c in 0..color_ch {
            let p = &channels[c];
            // mean_p, mean_Ip, cov_Ip
            let mean_p = box_blur_2d(p, w, h, radius);
            let ip: Vec<f32> = (0..n).map(|i| guide[i] * p[i]).collect();
            let mean_ip = box_blur_2d(&ip, w, h, radius);

            // a = cov_Ip / (var_I + eps), b = mean_p - a * mean_I
            let mut a = vec![0.0f32; n];
            let mut b = vec![0.0f32; n];
            for i in 0..n {
                let cov_ip = mean_ip[i] - mean_i[i] * mean_p[i];
                a[i] = cov_ip / (var_i[i] + epsilon);
                b[i] = mean_p[i] - a[i] * mean_i[i];
            }

            // smooth a and b before applying — this is what makes the output continuous
            let mean_a = box_blur_2d(&a, w, h, radius);
            let mean_b = box_blur_2d(&b, w, h, radius);

            for i in 0..n {
                filtered[c][i] = (mean_a[i] * guide[i] + mean_b[i]).clamp(0.0, 1.0);
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

/// Separable 2D box blur with edge clamping.
///
/// Uses 1D prefix sums per row then per column, giving O(1) work per pixel
/// regardless of radius. This is what makes the guided filter cheap at
/// arbitrary radii. Rows (and columns) are independent, so both passes are
/// rayon-parallel; the per-row/per-column arithmetic is unchanged, so results
/// are bit-identical to the serial version.
fn box_blur_2d(input: &[f32], width: usize, height: usize, radius: usize) -> Vec<f32> {
    if width == 0 || height == 0 {
        return Vec::new();
    }

    // horizontal pass: for each row, build a prefix sum then read out
    // the mean over [x-r, x+r] (clamped to row bounds) at each x.
    let mut h_pass = vec![0.0f32; input.len()];
    h_pass.par_chunks_mut(width).enumerate().for_each(|(y, out_row)| {
        let row_start = y * width;
        let mut prefix = vec![0.0f64; width + 1];
        for x in 0..width {
            prefix[x + 1] = prefix[x] + input[row_start + x] as f64;
        }
        for x in 0..width {
            let lo = x.saturating_sub(radius);
            let hi = (x + radius + 1).min(width);
            let cnt = (hi - lo) as f64;
            out_row[x] = ((prefix[hi] - prefix[lo]) / cnt) as f32;
        }
    });

    // vertical pass: same idea over columns, computed in parallel into a
    // column-major buffer then gathered back to row-major.
    let columns: Vec<f32> = (0..width).into_par_iter().flat_map_iter(|x| {
        let mut col_prefix = vec![0.0f64; height + 1];
        for y in 0..height {
            col_prefix[y + 1] = col_prefix[y] + h_pass[y * width + x] as f64;
        }
        (0..height).map(move |y| {
            let lo = y.saturating_sub(radius);
            let hi = (y + radius + 1).min(height);
            let cnt = (hi - lo) as f64;
            ((col_prefix[hi] - col_prefix[lo]) / cnt) as f32
        })
    }).collect();

    let mut out = vec![0.0f32; input.len()];
    out.par_chunks_mut(width).enumerate().for_each(|(y, out_row)| {
        for x in 0..width {
            out_row[x] = columns[x * height + y];
        }
    });

    out
}

#[cfg(test)]
#[path = "guided_tests.rs"]
mod tests;
