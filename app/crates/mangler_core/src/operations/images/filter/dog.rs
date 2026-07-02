//! Difference of Gaussians (DoG) / Extended DoG (XDoG) stylization filter.
//!
//! Produces stylized line-drawing / ink-sketch output by subtracting two
//! Gaussian-blurred copies of the luminance channel at different scales.
//!
//! - Plain DoG: `D(x) = Gσ(x) - Gk·σ(x)`, optionally thresholded to binary.
//! - XDoG (Winnemöller et al. 2012): adds a soft tanh-based ramp with a
//!   sharpness parameter to produce smoother, more expressive lines:
//!     `T(u) = 1                        if u ≥ ε`
//!     `T(u) = 1 + tanh(φ · (u - ε))    otherwise`
//!   where `u = (1 + p) · Gσ - p · Gk·σ`.
//!
//! The Gaussians are computed directly with separable 1D kernels (truncated at
//! 3σ) rather than the multi-pass box approximation used by the main blur op,
//! because DoG output is sensitive to the exact kernel shape at small sigmas.

use crate::float_image::FloatImage;
use crate::get_id;
use crate::input::{Input, InputSettings};
use crate::node_settings::NodeSettings;
use crate::operations::{OperationResponse, OperationError, OutputResponse, default_image, convert_input};
use crate::output::Output;
use crate::value::{Value, ValueType};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use std::time::Instant;

/// DoG / XDoG stylization filter.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpImageAdjustmentDog {}

impl OpImageAdjustmentDog {
    /// Returns the node metadata (name and description) for the DoG filter.
    pub fn settings() -> NodeSettings {
        NodeSettings {
            name: "difference of gaussians".to_string(),
            description: "Stylized edge / line-drawing filter via DoG or XDoG on luminance.".to_string(),
            help: "Subtracts two Gaussian-blurred copies of the luminance at sigmas sigma and k*sigma. Plain DoG thresholds the difference at zero; XDoG (Winnemoller 2012) replaces the hard threshold with `T(u) = 1 + tanh(phi * (u - eps))` over the blend `u = (1 + p)*G_sigma - p*G_k*sigma`, producing smoother, more expressive strokes.\n\nk is typically 1.6 (Marr-Hildreth). Gaussians use explicit separable 1D kernels truncated at 3*sigma since DoG is sensitive to exact kernel shape at small sigmas.".to_string(),
        }
    }

    /// Creates input ports: image, small sigma, k (large/small sigma ratio),
    /// XDoG parameters (p sharpness, epsilon threshold, phi ramp steepness),
    /// and a toggle between plain DoG and XDoG.
    pub fn create_inputs() -> Vec<Input> {
        vec![
            Input::new("image".to_string(), Value::Image { data: default_image(), change_id: get_id() }, None, None)
                .with_description("Source image whose luminance is stylized into a line drawing."),
            // small sigma (inner Gaussian) — controls line thickness
            Input::new("sigma".to_string(), Value::Decimal(1.0), Some(InputSettings::Slider { range: (0.1, 10.0), step_by: Some(0.1), clamp_to_range: true }), None)
                .with_description("Inner Gaussian standard deviation; controls line thickness."),
            // k ratio (outer sigma = k * sigma); 1.6 is the canonical Marr–Hildreth value
            Input::new("k".to_string(), Value::Decimal(1.6), Some(InputSettings::Slider { range: (1.01, 5.0), step_by: Some(0.01), clamp_to_range: true }), None)
                .with_description("Ratio between outer and inner sigma; 1.6 is the canonical Marr-Hildreth value."),
            // XDoG only: sharpness p — boosts the inner Gaussian in the blend
            Input::new("sharpness".to_string(), Value::Decimal(20.0), Some(InputSettings::Slider { range: (0.0, 200.0), step_by: Some(0.5), clamp_to_range: true }), None)
                .with_description("XDoG sharpness p that boosts the inner Gaussian in the blend."),
            // XDoG only: threshold epsilon — values below become ramped, above clamp to 1
            Input::new("threshold".to_string(), Value::Decimal(0.5), Some(InputSettings::Slider { range: (0.0, 1.0), step_by: Some(0.01), clamp_to_range: true }), None)
                .with_description("XDoG epsilon cutoff; values above are snapped to white, below are ramped."),
            // XDoG only: phi — steepness of the tanh soft threshold
            Input::new("phi".to_string(), Value::Decimal(10.0), Some(InputSettings::Slider { range: (0.1, 100.0), step_by: Some(0.1), clamp_to_range: true }), None)
                .with_description("XDoG phi; steepness of the tanh soft-threshold ramp."),
            // false = plain DoG thresholded at 0; true = XDoG soft-threshold
            Input::new("use xdog".to_string(), Value::Bool(true), None, None)
                .with_description("Enable XDoG soft-threshold output instead of plain binary DoG."),
        ]
    }

    /// Creates the output port: grayscale line-drawing image.
    pub fn create_outputs() -> Vec<Output> {
        vec![
            Output::new("output".to_string(), Value::Image { data: default_image(), change_id: get_id() }, None)
                .with_description("Grayscale line-drawing image produced by the DoG or XDoG stylization."),
        ]
    }

    /// Runs the DoG / XDoG filter on the image's luminance channel.
    pub async fn run(inputs: &mut [Input]) -> Result<OperationResponse, OperationError> {
        let start_time = Instant::now();
        let mut input_errors: Vec<(usize, String)> = vec![];

        // convert inputs
        let image_converted = convert_input(inputs, 0, ValueType::Image, &mut input_errors);
        let sigma_converted = convert_input(inputs, 1, ValueType::Decimal, &mut input_errors);
        let k_converted = convert_input(inputs, 2, ValueType::Decimal, &mut input_errors);
        let p_converted = convert_input(inputs, 3, ValueType::Decimal, &mut input_errors);
        let eps_converted = convert_input(inputs, 4, ValueType::Decimal, &mut input_errors);
        let phi_converted = convert_input(inputs, 5, ValueType::Decimal, &mut input_errors);
        let xdog_converted = convert_input(inputs, 6, ValueType::Bool, &mut input_errors);

        if !input_errors.is_empty() { return Err(OperationError { input_errors, node_error: None }); }

        let Value::Image { data, change_id: _ } = image_converted.unwrap() else { unreachable!() };
        let Value::Decimal(sigma) = sigma_converted.unwrap() else { unreachable!() };
        let Value::Decimal(k) = k_converted.unwrap() else { unreachable!() };
        let Value::Decimal(p) = p_converted.unwrap() else { unreachable!() };
        let Value::Decimal(eps) = eps_converted.unwrap() else { unreachable!() };
        let Value::Decimal(phi) = phi_converted.unwrap() else { unreachable!() };
        let Value::Bool(use_xdog) = xdog_converted.unwrap() else { unreachable!() };

        let sigma = sigma.max(0.1);
        let k = k.max(1.01);

        let (width, height) = data.dimensions();
        let ch = data.channels() as usize;

        // Extract luminance channel (Rec. 709) into a planar buffer
        let mut lum = vec![0.0f32; (width * height) as usize];
        for y in 0..height {
            for x in 0..width {
                let p = data.get_pixel(x, y);
                let l = if ch >= 3 {
                    0.2126 * p[0] + 0.7152 * p[1] + 0.0722 * p[2]
                } else {
                    p[0]
                };
                lum[(y * width + x) as usize] = l;
            }
        }

        // Two separable Gaussian blurs at sigma and k*sigma
        let inner = gaussian_blur_planar(&lum, width, height, sigma);
        let outer = gaussian_blur_planar(&lum, width, height, sigma * k);

        // Build output: either plain DoG > 0 or XDoG soft-threshold
        let mut out = FloatImage::new(width, height, ch as u32);
        for y in 0..height {
            for x in 0..width {
                let idx = (y * width + x) as usize;
                let gi = inner[idx];
                let go = outer[idx];

                let v = if use_xdog {
                    // u = (1 + p) * Gσ - p * Gkσ
                    let u = (1.0 + p) * gi - p * go;
                    if u >= eps {
                        1.0
                    } else {
                        (1.0 + (phi * (u - eps)).tanh()).clamp(0.0, 1.0)
                    }
                } else {
                    // Plain DoG: positive difference → white, negative → black
                    if gi - go > 0.0 { 1.0 } else { 0.0 }
                };

                // Preserve original alpha if present
                let src = data.get_pixel(x, y);
                let mut pixel = [0.0f32; 4];
                for val in pixel.iter_mut().take(ch.min(3)) {
                    *val = v;
                }
                if ch == 2 || ch == 4 { pixel[ch - 1] = src[ch - 1]; }
                out.put_pixel(x, y, &pixel[..ch]);
            }
        }

        Ok(OperationResponse {
            time: Instant::now().duration_since(start_time),
            responses: vec![
                OutputResponse { value: Value::Image { data: Arc::new(out), change_id: get_id() } },
            ],
        })
    }
}

/// Applies a separable Gaussian blur to a single-channel planar buffer.
///
/// The kernel is truncated at radius = ceil(3σ); weights are normalized so
/// that edge clamping does not shift the DC term.
fn gaussian_blur_planar(src: &[f32], width: u32, height: u32, sigma: f32) -> Vec<f32> {
    let sigma = sigma.max(1e-6);
    // Truncate the kernel at 3σ — captures ~99.7% of the Gaussian mass
    let radius = (3.0 * sigma).ceil() as i32;
    let mut kernel = vec![0.0f32; (2 * radius + 1) as usize];
    let two_sigma_sq = 2.0 * sigma * sigma;
    let mut sum = 0.0f32;
    for i in -radius..=radius {
        let w = (-((i * i) as f32) / two_sigma_sq).exp();
        kernel[(i + radius) as usize] = w;
        sum += w;
    }
    // Normalize so the kernel sums to 1
    for w in &mut kernel { *w /= sum; }

    let w = width as i32;
    let h = height as i32;

    // Horizontal pass
    let mut tmp = vec![0.0f32; src.len()];
    for y in 0..h {
        let row = (y * w) as usize;
        for x in 0..w {
            let mut acc = 0.0f32;
            for k in -radius..=radius {
                let sx = (x + k).clamp(0, w - 1) as usize;
                acc += src[row + sx] * kernel[(k + radius) as usize];
            }
            tmp[row + x as usize] = acc;
        }
    }

    // Vertical pass
    let mut out = vec![0.0f32; src.len()];
    for y in 0..h {
        for x in 0..w {
            let mut acc = 0.0f32;
            for k in -radius..=radius {
                let sy = (y + k).clamp(0, h - 1) as usize;
                acc += tmp[sy * w as usize + x as usize] * kernel[(k + radius) as usize];
            }
            out[(y * w + x) as usize] = acc;
        }
    }

    out
}

#[cfg(test)]
#[path = "dog_tests.rs"]
mod tests;
