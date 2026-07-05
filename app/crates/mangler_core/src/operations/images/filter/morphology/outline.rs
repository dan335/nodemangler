//! Mask outline / stroke.
//!
//! Given an input image treated as a mask (luminance is the mask if the
//! image has multiple channels), produces a stroked outline ring of
//! configurable thickness, position (outside, inside, centred), and color.
//! The output is a 4-channel RGBA image suitable for blending over the
//! original via the existing `blend` / `blit` nodes.
//!
//! Internally it builds an exact signed Euclidean distance field to the mask
//! boundary using a Felzenszwalb–Huttenlocher distance transform, then keeps
//! the pixels whose signed distance falls inside a band. Because the field is
//! a true Euclidean distance, the ring has uniform width and stays round —
//! unlike a separable square-kernel dilate/erode, which grows a circle into a
//! rounded square (≈√2 thicker on the diagonals than on the axes).

use crate::color::Color;
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

/// Produces a colored outline ring from a mask image.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpImageAdjustmentOutline {}

impl OpImageAdjustmentOutline {
    pub fn settings() -> NodeSettings {
        NodeSettings {
            name: "outline".to_string(),
            description: "Stroke a mask edge with a configurable colour and thickness.".to_string(),
            help: "Thresholds the input into a binary mask at 0.5, then computes an exact Euclidean distance field to the boundary with a Felzenszwalb–Huttenlocher distance transform. The stroke is the set of pixels whose signed distance to the edge falls inside a band, so the ring has perfectly uniform width and follows curves cleanly — no corner-bulging like a square-kernel dilate/erode (which grows a circle into a rounded square, ≈√2 thicker on the diagonals).\n\nThe mask is taken from the alpha channel when the image has one whose alpha crosses 0.5 (shapes on a transparent background are defined by alpha, and their RGB is often uniformly opaque-looking); otherwise it falls back to Rec.709 luminance. So a shape on a transparent background strokes its silhouette, and an opaque black-on-white image strokes its luminance edge.\n\n`position` places the band relative to the edge: `0` = outer (0…thickness outside), `1` = inner (thickness…0 inside), `2` = centred (±thickness/2 straddling the edge). `thickness` is the total stroke width in pixels. Output is always 4-channel RGBA; alpha carries a 1-pixel antialiased falloff at both edges of the band and is multiplied by the input colour's alpha. Composite over the source with `blit` or `blend`.".to_string(),
        }
    }

    pub fn create_inputs() -> Vec<Input> {
        vec![
            Input::new("image".to_string(), Value::Image { data: default_image(), change_id: get_id() }, None, None)
                .with_description("Source image or mask whose edges to stroke."),
            Input::new("thickness".to_string(), Value::Integer(2), Some(InputSettings::Slider { range: (1.0, 32.0), step_by: Some(1.0), clamp_to_range: true }), None)
                .with_description("Stroke width in pixels."),
            Input::new("position".to_string(), Value::Integer(2), Some(InputSettings::Slider { range: (0.0, 2.0), step_by: Some(1.0), clamp_to_range: true }), None)
                .with_description("0 = outer, 1 = inner, 2 = centred around the edge."),
            Input::new("color".to_string(), Value::Color(Color { r: 0.0, g: 0.0, b: 0.0, a: 1.0 }), None, None)
                .with_description("Color used for the outline strokes."),
        ]
    }

    pub fn create_outputs() -> Vec<Output> {
        vec![
            Output::new("output".to_string(), Value::Image { data: default_image(), change_id: get_id() }, None)
                .with_description("RGBA image containing only the outline; alpha is zero where the source mask is unchanged."),
        ]
    }

    pub async fn run(inputs: &mut [Input]) -> Result<OperationResponse, OperationError> {
        let start_time = Instant::now();
        let mut input_errors: Vec<(usize, String)> = vec![];

        let image_converted = convert_input(inputs, 0, ValueType::Image, &mut input_errors);
        let thickness_converted = convert_input(inputs, 1, ValueType::Integer, &mut input_errors);
        let position_converted = convert_input(inputs, 2, ValueType::Integer, &mut input_errors);
        let color_converted = convert_input(inputs, 3, ValueType::Color, &mut input_errors);

        if !input_errors.is_empty() { return Err(OperationError { input_errors, node_error: None }); }

        let Value::Image { data, change_id: _ } = image_converted.unwrap() else { unreachable!() };
        let Value::Integer(thickness) = thickness_converted.unwrap() else { unreachable!() };
        let Value::Integer(position) = position_converted.unwrap() else { unreachable!() };
        let Value::Color(color) = color_converted.unwrap() else { unreachable!() };

        let thickness = thickness.max(1) as f32;
        let position = position.clamp(0, 2);

        let (width, height) = data.dimensions();
        let w = width as usize;
        let h = height as usize;
        let ch = data.channels() as usize;

        // Pick the channel that defines the shape. Prefer alpha when the image
        // has one AND its alpha actually crosses 0.5 — shapes on a transparent
        // background are defined by alpha, and their RGB is often uniformly
        // opaque-looking, so luminance would see no edge and the stroke would
        // come out empty. Fall back to luminance (Rec.709) for fully-opaque
        // images, or the single channel for grayscale.
        let has_alpha = ch == 2 || ch == 4;
        let use_alpha = has_alpha && {
            let ai = ch - 1;
            let raw = data.as_raw();
            let mut lo = f32::INFINITY;
            let mut hi = f32::NEG_INFINITY;
            for i in 0..(w * h) {
                let a = raw[i * ch + ai];
                lo = lo.min(a);
                hi = hi.max(a);
            }
            lo < 0.5 && hi >= 0.5
        };

        // Threshold the chosen channel into a binary mask. `inside` is the
        // foreground set whose boundary we stroke; `outside` its complement. A
        // hard threshold is what lets the distance field measure one edge.
        let mut inside = vec![false; w * h];
        for y in 0..h {
            for x in 0..w {
                let px = data.get_pixel(x as u32, y as u32);
                let v = if use_alpha {
                    px[ch - 1]
                } else if ch >= 3 {
                    0.2126 * px[0] + 0.7152 * px[1] + 0.0722 * px[2]
                } else {
                    px[0]
                };
                inside[y * w + x] = v >= 0.5;
            }
        }

        // Exact squared Euclidean distance to the nearest pixel of each class.
        //   d_out_sq — distance to the nearest inside pixel  (0 inside,  >0 outside)
        //   d_in_sq  — distance to the nearest outside pixel (0 outside, >0 inside)
        let outside: Vec<bool> = inside.iter().map(|&b| !b).collect();
        let d_out_sq = edt_squared(&inside, w, h);
        let d_in_sq = edt_squared(&outside, w, h);

        // Signed distance to the boundary: negative inside, positive outside,
        // in pixel units. The 0.5 shift recentres the field on the interface
        // that runs midway between adjacent inside/outside pixel centres, so
        // boundary pixels sit at ±0.5 and thin strokes stay symmetric.
        let (lo, hi) = match position {
            0 => (0.0, thickness),                       // outer
            1 => (-thickness, 0.0),                      // inner
            _ => (-0.5 * thickness, 0.5 * thickness),    // centred
        };
        const AA: f32 = 0.5; // half-width of the 1-pixel antialiased edge

        let mut output = FloatImage::new(width, height, 4);
        for y in 0..h {
            for x in 0..w {
                let i = y * w + x;
                let sdf = if inside[i] {
                    0.5 - d_in_sq[i].sqrt()
                } else {
                    d_out_sq[i].sqrt() - 0.5
                };

                // Coverage of the band [lo, hi] with a soft 1-pixel edge on
                // each side: ramp up across the lower edge, down across the upper.
                let lower = smoothstep(lo - AA, lo + AA, sdf);
                let upper = 1.0 - smoothstep(hi - AA, hi + AA, sdf);
                let alpha = (lower * upper).clamp(0.0, 1.0) * color.a;

                output.put_pixel(x as u32, y as u32, &[color.r, color.g, color.b, alpha]);
            }
        }

        Ok(OperationResponse {
            time: Instant::now().duration_since(start_time),
            responses: vec![
                OutputResponse { value: Value::Image { data: Arc::new(output), change_id: get_id() } },
            ],
        })
    }
}

/// Smooth Hermite interpolation between 0 and 1 as `x` crosses `[e0, e1]`.
fn smoothstep(e0: f32, e1: f32, x: f32) -> f32 {
    if e0 == e1 {
        return if x < e0 { 0.0 } else { 1.0 };
    }
    let t = ((x - e0) / (e1 - e0)).clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

/// Exact squared Euclidean distance transform (Felzenszwalb–Huttenlocher, 2012).
///
/// `feature[i]` marks a source pixel (distance 0). Returns, for every pixel in
/// row-major order, the squared Euclidean distance to the nearest source pixel.
/// Runs in O(w·h): a 1D transform down each column, then across each row. If
/// there are no source pixels every result is a large sentinel (`≈1e20`).
fn edt_squared(feature: &[bool], w: usize, h: usize) -> Vec<f32> {
    // A large finite sentinel (not f32::INFINITY): the FH intersection formula
    // subtracts two of these for non-source pixels, and INF−INF would be NaN.
    const INF: f32 = 1e20;
    if w == 0 || h == 0 {
        return Vec::new();
    }

    let grid: Vec<f32> = feature.iter().map(|&f| if f { 0.0 } else { INF }).collect();

    // Column pass: transform each column independently.
    let cols: Vec<Vec<f32>> = (0..w).into_par_iter().map(|x| {
        let f: Vec<f32> = (0..h).map(|y| grid[y * w + x]).collect();
        dt_1d(&f)
    }).collect();

    // Row pass: feed the column results back in, one row at a time.
    let rows: Vec<Vec<f32>> = (0..h).into_par_iter().map(|y| {
        let f: Vec<f32> = (0..w).map(|x| cols[x][y]).collect();
        dt_1d(&f)
    }).collect();

    let mut out = vec![0.0f32; w * h];
    for (y, row) in rows.iter().enumerate() {
        out[y * w..(y + 1) * w].copy_from_slice(row);
    }
    out
}

/// One-dimensional squared distance transform: `d[q] = min_p f[p] + (q − p)²`.
///
/// Computes the lower envelope of the parabolas rooted at each sample, walking
/// left to right. `v` holds the indices of the parabolas currently forming the
/// envelope and `z` the boundaries between them.
fn dt_1d(f: &[f32]) -> Vec<f32> {
    let n = f.len();
    let mut d = vec![0.0f32; n];
    if n == 0 {
        return d;
    }

    let mut v = vec![0usize; n]; // parabola indices in the lower envelope
    let mut z = vec![0.0f32; n + 1]; // breakpoints between consecutive parabolas
    let mut k = 0usize;
    v[0] = 0;
    z[0] = f32::NEG_INFINITY;
    z[1] = f32::INFINITY;

    for q in 1..n {
        // Intersection abscissa of the parabolas from q and from the current
        // top of the envelope; pop parabolas that q's has overtaken.
        loop {
            let p = v[k];
            let s = ((f[q] + (q * q) as f32) - (f[p] + (p * p) as f32)) / (2.0 * (q as f32 - p as f32));
            if s <= z[k] && k > 0 {
                k -= 1;
            } else {
                k += 1;
                v[k] = q;
                z[k] = s;
                z[k + 1] = f32::INFINITY;
                break;
            }
        }
    }

    let mut k = 0usize;
    for q in 0..n {
        while z[k + 1] < q as f32 {
            k += 1;
        }
        let p = v[k];
        let dq = q as f32 - p as f32;
        d[q] = dq * dq + f[p];
    }
    d
}

#[cfg(test)]
#[path = "outline_tests.rs"]
mod tests;
