//! Morphological erosion.
//!
//! For each pixel, replaces it with the per-channel minimum over a square
//! neighborhood. Erosion shrinks bright regions and grows dark ones; it is
//! the fundamental morphological operation used for mask cleanup, shape
//! shrinking, and as a building block for opening/closing.
//!
//! The alpha channel is eroded alongside color channels.

use crate::float_image::FloatImage;
use crate::get_id;
use crate::input::{Input, InputSettings};
use crate::node_settings::NodeSettings;
use crate::operations::{OperationResponse, OperationError, OutputResponse, default_image, convert_input, scale_to_resolution};
use crate::output::Output;
use crate::value::{Value, ValueType};
use rayon::prelude::*;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use std::time::Instant;

/// Morphological erosion (per-channel min in a square window).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpImageAdjustmentErode {}

impl OpImageAdjustmentErode {
    /// Returns the node metadata (name and description) for erode.
    pub fn settings() -> NodeSettings {
        NodeSettings {
            name: "erode".to_string(),
            description: "Morphological erosion — per-channel min in a square neighborhood. Shrinks bright regions.".to_string(),
            help: "For each pixel takes the per-channel minimum over a (2r+1) square window. Bright regions shrink by `radius` pixels, thin bright filaments disappear, and dark regions grow.\n\nFundamental morphological primitive; combining with dilation gives open/close. Implemented as separable 1D min passes (horizontal then vertical), so cost is O(r) per pixel rather than O(r^2). Alpha is eroded alongside color channels; edges are handled by clamping.".to_string(),
        }
    }

    /// Creates input ports: image and radius (square window half-size).
    pub fn create_inputs() -> Vec<Input> {
        vec![
            Input::new("image".to_string(), Value::Image { data: default_image(), change_id: get_id() }, None, None)
                .with_description("Source image or mask to erode."),
            // radius of the structuring element (square)
            Input::new("radius".to_string(), Value::Integer(1), Some(InputSettings::Slider { range: (1.0, 16.0), step_by: Some(1.0), clamp_to_range: true }), None)
                .with_description("Half-size of the square window, in pixels at a 1024px reference (scales with image size, so erosion looks the same at any resolution); larger values shrink bright regions more."),
        ]
    }

    /// Creates the output port: the eroded image.
    pub fn create_outputs() -> Vec<Output> {
        vec![
            Output::new("output".to_string(), Value::Image { data: default_image(), change_id: get_id() }, None)
                .with_description("Eroded image where bright regions have shrunk by the chosen radius."),
        ]
    }

    /// Runs the erosion operation.
    pub async fn run(inputs: &mut [Input]) -> Result<OperationResponse, OperationError> {
        let start_time = Instant::now();
        let mut input_errors: Vec<(usize, String)> = vec![];

        let image_converted = convert_input(inputs, 0, ValueType::Image, &mut input_errors);
        let radius_converted = convert_input(inputs, 1, ValueType::Integer, &mut input_errors);

        if !input_errors.is_empty() { return Err(OperationError { input_errors, node_error: None }); }

        let Value::Image { data, change_id: _ } = image_converted.unwrap() else { unreachable!() };
        let Value::Integer(radius) = radius_converted.unwrap() else { unreachable!() };

        // Radius is authored in reference pixels (at 1024px) and scaled to the
        // actual image so the erosion is the same relative size at any resolution.
        let (rw, rh) = data.dimensions();
        let radius = scale_to_resolution(radius.max(1) as f32, rw, rh).round().max(1.0) as i32;

        // Separable min-filter: horizontal pass then vertical pass. A square
        // min kernel factors into 1D min ops, reducing cost from O(r²) to O(r).
        let out = separable_morphology(&data, radius, |a, b| a.min(b));

        Ok(OperationResponse {
            time: Instant::now().duration_since(start_time),
            responses: vec![
                OutputResponse { value: Value::Image { data: Arc::new(out), change_id: get_id() } },
            ],
        })
    }
}

/// Applies a separable 1D morphology operator (min or max) to a square window.
///
/// `op` is a commutative, associative reducer — `f32::min` for erosion,
/// `f32::max` for dilation. Alpha is processed alongside color channels.
///
/// Each 1D pass uses the van Herk–Gil-Werman running min/max algorithm:
/// block-aligned forward and backward scans let every window result be formed
/// from two lookups, so the cost per pixel is O(1) regardless of radius. Edge
/// handling is replicate-clamp, identical to a naive clamped window scan, and
/// results are exact (min/max folds are order-independent).
pub(crate) fn separable_morphology<F>(data: &FloatImage, radius: i32, op: F) -> FloatImage
where
    F: Fn(f32, f32) -> f32 + Sync + Send + Copy,
{
    let (width, height) = data.dimensions();
    let ch = data.channels() as usize;
    let w = width as usize;
    let h = height as usize;
    let r = radius.max(0) as usize;

    if w == 0 || h == 0 {
        return FloatImage::from_raw(width, height, data.channels(), Vec::new()).unwrap();
    }

    let raw = data.as_raw();

    // Horizontal pass → tmp (row-major, same layout as the source)
    let tmp: Vec<f32> = (0..h).into_par_iter().flat_map_iter(|y| {
        let m = w + 2 * r;
        let mut line = vec![0.0f32; m];
        let mut fwd = vec![0.0f32; m];
        let mut bwd = vec![0.0f32; m];
        let mut dst = vec![0.0f32; w];
        let mut out_row = vec![0.0f32; w * ch];
        let row = &raw[y * w * ch..(y + 1) * w * ch];
        for c in 0..ch {
            for (i, v) in line.iter_mut().enumerate() {
                let x = (i as i64 - r as i64).clamp(0, w as i64 - 1) as usize;
                *v = row[x * ch + c];
            }
            van_herk_line(&line, &mut fwd, &mut bwd, &mut dst, r, op);
            for x in 0..w {
                out_row[x * ch + c] = dst[x];
            }
        }
        out_row
    }).collect();

    // Vertical pass reads columns from tmp; the per-column results are
    // gathered back into row-major order afterwards.
    let tmp_ref = &tmp;
    let cols: Vec<Vec<f32>> = (0..w).into_par_iter().map(|x| {
        let m = h + 2 * r;
        let mut line = vec![0.0f32; m];
        let mut fwd = vec![0.0f32; m];
        let mut bwd = vec![0.0f32; m];
        let mut dst = vec![0.0f32; h];
        let mut col = vec![0.0f32; h * ch];
        for c in 0..ch {
            for (i, v) in line.iter_mut().enumerate() {
                let y = (i as i64 - r as i64).clamp(0, h as i64 - 1) as usize;
                *v = tmp_ref[(y * w + x) * ch + c];
            }
            van_herk_line(&line, &mut fwd, &mut bwd, &mut dst, r, op);
            for y in 0..h {
                col[y * ch + c] = dst[y];
            }
        }
        col
    }).collect();

    let cols_ref = &cols;
    let pixels: Vec<f32> = (0..h).into_par_iter().flat_map_iter(move |y| {
        (0..w).flat_map(move |x| {
            let base = y * ch;
            cols_ref[x][base..base + ch].iter().copied()
        })
    }).collect();

    FloatImage::from_raw(width, height, data.channels(), pixels).unwrap()
}

/// One van Herk–Gil-Werman pass over a replicate-padded line.
///
/// `line` holds `dst.len() + 2 * radius` samples: the source values with
/// `radius` edge-replicated samples prepended and appended. `fwd`/`bwd` are
/// scratch buffers of the same length as `line`. For every output index `x`
/// the result is the exact `op`-fold of `line[x..=x + 2 * radius]`, i.e. the
/// clamped window centred on source index `x`.
fn van_herk_line<F>(line: &[f32], fwd: &mut [f32], bwd: &mut [f32], dst: &mut [f32], radius: usize, op: F)
where
    F: Fn(f32, f32) -> f32 + Copy,
{
    let m = line.len();
    let k = 2 * radius + 1;
    // Forward scan restarts at every block boundary; backward scan restarts at
    // every block end. Any window of width k then spans at most two blocks and
    // is covered by one bwd value (its head) and one fwd value (its tail).
    for i in 0..m {
        fwd[i] = if i % k == 0 { line[i] } else { op(fwd[i - 1], line[i]) };
    }
    for i in (0..m).rev() {
        bwd[i] = if i % k == k - 1 || i == m - 1 { line[i] } else { op(bwd[i + 1], line[i]) };
    }
    for (x, d) in dst.iter_mut().enumerate() {
        *d = op(bwd[x], fwd[x + k - 1]);
    }
}

#[cfg(test)]
#[path = "erode_tests.rs"]
mod tests;
