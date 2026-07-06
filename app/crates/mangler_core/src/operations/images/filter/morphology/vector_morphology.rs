//! Vector morphology on normal-map-style direction fields.
//!
//! Plain per-channel erode/dilate on an RGB normal map produces junk normals
//! (components drift independently and unit length is lost). Vector
//! morphology avoids that by picking a single coherent source vector from
//! the neighborhood — the neighbor with the smallest or largest horizontal
//! tilt — and emitting that vector unchanged. The output always carries
//! real, already-normalised normals from the input, never an interpolation.
//!
//! Mode 0 = erode: pick the flattest neighbour (smallest `nx² + ny²`),
//! so normals converge toward straight-up. Mode 1 = dilate: pick the most
//! tilted neighbour, so normals converge toward the steepest local edge.

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

/// Vector erode/dilate on normal-map-style images.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpImageAdjustmentVectorMorphology {}

impl OpImageAdjustmentVectorMorphology {
    pub fn settings() -> NodeSettings {
        NodeSettings {
            name: "vector morphology".to_string(),
            description: "Erode/dilate a normal map by picking the flattest/steepest neighbouring vector.".to_string(),
            help: "For each pixel, scans a (2r+1)² neighbourhood and unpacks every neighbour's RG encoding into a signed tilt `(nx, ny) ∈ [-1,1]²`. Erode mode selects the neighbour with the smallest horizontal tilt magnitude (`nx² + ny²`) — normals bias toward straight up. Dilate mode selects the largest — normals bias toward the steepest nearby edge.\n\nUnlike per-channel erode/dilate, the chosen neighbour's full RGBA pixel is copied unmodified, so output vectors remain unit length and consistent. For arbitrary colour images (non-normal-map inputs) this still runs but the notion of `tilt` treats R and G as a 2-vector — results may surprise if that's not what the data represents.".to_string(),
        }
    }

    pub fn create_inputs() -> Vec<Input> {
        vec![
            Input::new("image".to_string(), Value::Image { data: default_image(), change_id: get_id() }, None, None)
                .with_description("Normal map (or RG direction field) to morph."),
            Input::new("mode".to_string(), Value::Integer(0), Some(InputSettings::Slider { range: (0.0, 1.0), step_by: Some(1.0), clamp_to_range: true }), None)
                .with_description("0 = erode (flatten), 1 = dilate (sharpen edges)."),
            Input::new("radius".to_string(), Value::Integer(1), Some(InputSettings::Slider { range: (1.0, 8.0), step_by: Some(1.0), clamp_to_range: true }), None)
                .with_description("Half-size of the square window, in pixels at a 1024px reference (scales with image size, so the effect is the same at any resolution)."),
        ]
    }

    pub fn create_outputs() -> Vec<Output> {
        vec![
            Output::new("output".to_string(), Value::Image { data: default_image(), change_id: get_id() }, None)
                .with_description("Per-pixel copy of the most/least tilted neighbour in the radius."),
        ]
    }

    pub async fn run(inputs: &mut [Input]) -> Result<OperationResponse, OperationError> {
        let start_time = Instant::now();
        let mut input_errors: Vec<(usize, String)> = vec![];

        let image_converted = convert_input(inputs, 0, ValueType::Image, &mut input_errors);
        let mode_converted = convert_input(inputs, 1, ValueType::Integer, &mut input_errors);
        let radius_converted = convert_input(inputs, 2, ValueType::Integer, &mut input_errors);

        if !input_errors.is_empty() { return Err(OperationError { input_errors, node_error: None }); }

        let Value::Image { data, change_id: _ } = image_converted.unwrap() else { unreachable!() };
        let Value::Integer(mode) = mode_converted.unwrap() else { unreachable!() };
        let Value::Integer(radius) = radius_converted.unwrap() else { unreachable!() };

        let mode = mode.clamp(0, 1);
        // Radius is authored in reference pixels (at 1024px) and scaled to the
        // actual image so the search window is the same relative size at any resolution.
        let (w, h) = data.dimensions();
        let radius = scale_to_resolution(radius.max(1) as f32, w, h).round().max(1.0) as i32;
        let ch = data.channels() as usize;
        let wu = w as usize;
        let hu = h as usize;
        let r = radius as usize;

        // Precompute the tilt² plane once — one f32 per pixel — instead of
        // re-deriving it for every neighbour of every pixel. Grayscale
        // (ch < 2) has no Y to contribute, so only R acts as tilt X.
        let data_ref = &data;
        let tilt: Vec<f32> = (0..hu).into_par_iter().flat_map_iter(|y| {
            (0..wu).map(move |x| {
                let px = data_ref.get_pixel(x as u32, y as u32);
                let nx = if ch >= 1 { px[0] * 2.0 - 1.0 } else { 0.0 };
                let ny = if ch >= 2 { px[1] * 2.0 - 1.0 } else { 0.0 };
                nx * nx + ny * ny
            })
        }).collect();

        // The square-window argmin/argmax is separable: a horizontal running
        // pass finds each row-window's winner, then a vertical pass reduces
        // those per-column. The winning SCORE is exact (pure comparisons);
        // ties keep the earliest candidate (smallest y, then smallest x),
        // which may differ from the naive scan order but is deterministic
        // and value-equal.
        let better = move |cand: f32, best: f32| if mode == 0 { cand < best } else { cand > best };

        // Horizontal pass: per pixel, (winning tilt², source x) over [x-r, x+r].
        let tilt_ref = &tilt;
        let hwin: Vec<(f32, u32)> = (0..hu).into_par_iter().flat_map_iter(|y| {
            let m = wu + 2 * r;
            let mut line = vec![(0.0f32, 0u32); m];
            let mut fwd = vec![(0.0f32, 0u32); m];
            let mut bwd = vec![(0.0f32, 0u32); m];
            let mut dst = vec![(0.0f32, 0u32); wu];
            for (i, v) in line.iter_mut().enumerate() {
                let x = (i as i64 - r as i64).clamp(0, wu as i64 - 1) as usize;
                *v = (tilt_ref[y * wu + x], x as u32);
            }
            van_herk_argext(&line, &mut fwd, &mut bwd, &mut dst, r, better);
            dst
        }).collect();

        // Vertical pass: reduce the stored per-column row-winners over
        // [y-r, y+r], recovering full (source x, source y) coordinates.
        let hwin_ref = &hwin;
        let winners: Vec<Vec<(u32, u32)>> = (0..wu).into_par_iter().map(|x| {
            let m = hu + 2 * r;
            let mut line = vec![(0.0f32, 0u32); m];
            let mut fwd = vec![(0.0f32, 0u32); m];
            let mut bwd = vec![(0.0f32, 0u32); m];
            let mut dst = vec![(0.0f32, 0u32); hu];
            for (i, v) in line.iter_mut().enumerate() {
                let y = (i as i64 - r as i64).clamp(0, hu as i64 - 1) as usize;
                *v = (hwin_ref[y * wu + x].0, y as u32);
            }
            van_herk_argext(&line, &mut fwd, &mut bwd, &mut dst, r, better);
            dst.iter().map(|&(_, by)| {
                let bx = hwin_ref[by as usize * wu + x].1;
                (bx, by)
            }).collect()
        }).collect();

        // Copy each winner's pixel verbatim so vector length is preserved
        // exactly (no reconstruction or normalisation).
        let winners_ref = &winners;
        let pixels: Vec<f32> = (0..hu).into_par_iter().flat_map_iter(move |y| {
            (0..wu).flat_map(move |x| {
                let (bx, by) = winners_ref[x][y];
                data_ref.get_pixel(bx, by)[..ch].iter().copied()
            })
        }).collect();

        let output = FloatImage::from_raw(w, h, data.channels(), pixels).unwrap();

        Ok(OperationResponse {
            time: Instant::now().duration_since(start_time),
            responses: vec![
                OutputResponse { value: Value::Image { data: Arc::new(output), change_id: get_id() } },
            ],
        })
    }
}

/// One van Herk–Gil-Werman argmin/argmax pass over a replicate-padded line of
/// (score, source-index) pairs.
///
/// `line` holds `dst.len() + 2 * radius` samples: the source pairs with
/// `radius` edge-replicated samples prepended and appended. `fwd`/`bwd` are
/// scratch buffers of the same length as `line`. For every output index `x`
/// the result carries the extremal score over `line[x..=x + 2 * radius]`
/// (exact — only comparisons are performed) and one source index achieving
/// it; on ties the earliest index in the window wins.
fn van_herk_argext<F>(
    line: &[(f32, u32)],
    fwd: &mut [(f32, u32)],
    bwd: &mut [(f32, u32)],
    dst: &mut [(f32, u32)],
    radius: usize,
    better: F,
) where
    F: Fn(f32, f32) -> bool + Copy,
{
    // Keeps `a` unless `b` scores strictly better, so the earlier candidate
    // survives ties in every scan below.
    let sel = |a: (f32, u32), b: (f32, u32)| if better(b.0, a.0) { b } else { a };
    let m = line.len();
    let k = 2 * radius + 1;
    // Forward scan restarts at every block boundary; backward scan restarts
    // at every block end. Any window of width k then spans at most two blocks
    // and is covered by one bwd value (its head) and one fwd value (its tail).
    for i in 0..m {
        fwd[i] = if i % k == 0 { line[i] } else { sel(fwd[i - 1], line[i]) };
    }
    for i in (0..m).rev() {
        bwd[i] = if i % k == k - 1 || i == m - 1 { line[i] } else { sel(line[i], bwd[i + 1]) };
    }
    for (x, d) in dst.iter_mut().enumerate() {
        *d = sel(bwd[x], fwd[x + k - 1]);
    }
}

#[cfg(test)]
#[path = "vector_morphology_tests.rs"]
mod tests;
