//! Affine transform: translate, rotate, and scale in one pass.
//!
//! Applies a single affine mapping — scale and rotate about the image centre,
//! then translate by a pixel offset — using inverse mapping with bilinear
//! sampling, so the image is resampled only once (chaining separate nodes
//! would soften it at each step). The `edge` selector decides what fills the
//! space the transform exposes: a fill colour (transparent by default),
//! wrap-around tiling, edge extension, or mirror reflection.
//!
//! Replaces the old translate-only and wrap-only "tiling transform" nodes.

use crate::color::Color;
use crate::float_image::FloatImage;
use crate::get_id;
use crate::input::{Input, InputSettings};
use crate::node_settings::NodeSettings;
use crate::operations::{OperationResponse, OperationError, OutputResponse, default_image, convert_input};
use crate::output::Output;
use crate::value::{EdgeMode, Value, ValueType};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use std::time::Instant;

/// Translates, rotates, and scales an image with a configurable edge fill.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpImageTransformAffine {}

impl OpImageTransformAffine {
    pub fn settings() -> NodeSettings {
        NodeSettings {
            name: "transform".to_string(),
            description: "Translate, rotate, and scale an image, choosing how to fill the exposed space.".to_string(),
            help: "One affine transform applied in a single resample: the image is scaled and rotated about its centre, then translated by `offset x` / `offset y`. Offsets are fractions of the image size (0.25 = a quarter of the way across, positive x right, positive y down), so the same value gives the same shift at any resolution — change the image from 512px to 4096px and you don't touch the offsets. `rotation` is in degrees, positive turns clockwise. `scale x` / `scale y` scale about the centre (1 = no change); the canvas size stays fixed. Doing all three in one node keeps the result sharp — chaining separate translate/rotate/scale nodes resamples the image each time and softens it.\n\n`edge` chooses what appears in the space the transform uncovers:\n• fill — a solid `fill color` (default fully transparent)\n• wrap — the image tiles, so anything that moves off one side reappears on the other (keeps a tileable input tileable)\n• extend — the border pixels are stretched out to the edge\n• mirror — the image is reflected back across each edge\n\nSampling is bilinear, so whole-pixel/whole-degree transforms stay crisp and fractional ones interpolate. Output dimensions and channel count match the input; for images with fewer than 4 channels the fill colour is reduced to match (luminance for 1 channel, RGB for 3).".to_string(),
        }
    }

    pub fn create_inputs() -> Vec<Input> {
        vec![
            Input::new("image".to_string(), Value::Image { data: default_image(), change_id: get_id() }, None, None)
                .with_description("Source image to transform."),
            Input::new("offset x".to_string(), Value::Decimal(0.0), Some(InputSettings::Slider { range: (-1.0, 1.0), step_by: None, clamp_to_range: false }), None)
                .with_description("Horizontal shift as a fraction of image width (0.25 = a quarter across); positive moves content right. Resolution-independent."),
            Input::new("offset y".to_string(), Value::Decimal(0.0), Some(InputSettings::Slider { range: (-1.0, 1.0), step_by: None, clamp_to_range: false }), None)
                .with_description("Vertical shift as a fraction of image height; positive moves content down. Resolution-independent."),
            Input::new("rotation".to_string(), Value::Decimal(0.0), Some(InputSettings::Slider { range: (-360.0, 360.0), step_by: Some(0.1), clamp_to_range: false }), None)
                .with_description("Rotation in degrees about the image centre; positive is clockwise."),
            Input::new("scale x".to_string(), Value::Decimal(1.0), Some(InputSettings::Slider { range: (0.01, 4.0), step_by: Some(0.01), clamp_to_range: false }), None)
                .with_description("Horizontal scale about the centre; 1 = unchanged."),
            Input::new("scale y".to_string(), Value::Decimal(1.0), Some(InputSettings::Slider { range: (0.01, 4.0), step_by: Some(0.01), clamp_to_range: false }), None)
                .with_description("Vertical scale about the centre; 1 = unchanged."),
            Input::new("edge".to_string(), Value::EdgeMode(EdgeMode::Fill), None, None)
                .with_description("What fills the exposed space: fill colour, wrap, extend, or mirror."),
            Input::new("fill color".to_string(), Value::Color(Color { r: 0.0, g: 0.0, b: 0.0, a: 0.0 }), None, None)
                .with_description("Colour used for exposed space when edge = fill (default transparent)."),
        ]
    }

    pub fn create_outputs() -> Vec<Output> {
        vec![
            Output::new("output".to_string(), Value::Image { data: default_image(), change_id: get_id() }, None)
                .with_description("The transformed image, same size and channel count as the input."),
        ]
    }

    pub async fn run(inputs: &mut [Input]) -> Result<OperationResponse, OperationError> {
        let start_time = Instant::now();
        let mut input_errors: Vec<(usize, String)> = vec![];

        let image_converted = convert_input(inputs, 0, ValueType::Image, &mut input_errors);
        let ox_converted = convert_input(inputs, 1, ValueType::Decimal, &mut input_errors);
        let oy_converted = convert_input(inputs, 2, ValueType::Decimal, &mut input_errors);
        let rot_converted = convert_input(inputs, 3, ValueType::Decimal, &mut input_errors);
        let scale_x_converted = convert_input(inputs, 4, ValueType::Decimal, &mut input_errors);
        let scale_y_converted = convert_input(inputs, 5, ValueType::Decimal, &mut input_errors);
        let edge_converted = convert_input(inputs, 6, ValueType::EdgeMode, &mut input_errors);
        let fill_converted = convert_input(inputs, 7, ValueType::Color, &mut input_errors);

        if !input_errors.is_empty() { return Err(OperationError { input_errors, node_error: None }); }

        let Value::Image { data, change_id: _ } = image_converted.unwrap() else { unreachable!() };
        let Value::Decimal(offset_x) = ox_converted.unwrap() else { unreachable!() };
        let Value::Decimal(offset_y) = oy_converted.unwrap() else { unreachable!() };
        let Value::Decimal(rotation) = rot_converted.unwrap() else { unreachable!() };
        let Value::Decimal(scale_x) = scale_x_converted.unwrap() else { unreachable!() };
        let Value::Decimal(scale_y) = scale_y_converted.unwrap() else { unreachable!() };
        let Value::EdgeMode(edge) = edge_converted.unwrap() else { unreachable!() };
        let Value::Color(fill) = fill_converted.unwrap() else { unreachable!() };

        let (width, height) = data.dimensions();
        let nch = data.channels() as usize;

        // The fill colour reduced to the source's channel layout so exposed
        // space matches the image's storage (see `help`).
        let luma = 0.2126 * fill.r + 0.7152 * fill.g + 0.0722 * fill.b;
        let mut fill_px: Vec<f32> = match nch {
            1 => vec![luma],
            2 => vec![luma, fill.a],
            3 => vec![fill.r, fill.g, fill.b],
            _ => vec![fill.r, fill.g, fill.b, fill.a],
        };

        // Premultiply so transparent pixels' hidden colour can't bleed into
        // interpolated edge pixels (white fringe around dark shapes). The fill
        // colour is a source tap too, so it must live in the same premultiplied
        // space (colour components *= fill alpha); 1/3-channel fills have no
        // alpha and are left untouched.
        let premul = data.has_alpha();
        let src = if premul { Arc::new(data.premultiply_alpha()) } else { Arc::clone(&data) };
        if premul {
            let a = *fill_px.last().unwrap();
            for c in &mut fill_px[..nch - 1] { *c *= a; }
        }

        // Guard against a zero scale (division below); keep the sign so the
        // near-degenerate case still degrades gracefully rather than dividing by 0.
        let sx = if scale_x.abs() < 1e-3 { 1e-3 } else { scale_x };
        let sy = if scale_y.abs() < 1e-3 { 1e-3 } else { scale_y };

        let cx = width as f32 / 2.0;
        let cy = height as f32 / 2.0;
        let theta = rotation.to_radians();
        let (cos_t, sin_t) = (theta.cos(), theta.sin());

        // Offsets are fractions of the image size, so the same value produces
        // the same relative shift at any resolution (0.25 = a quarter of the
        // way across, whether the image is 512px or 4096px). Convert to pixels.
        let offset_x_px = offset_x * width as f32;
        let offset_y_px = offset_y * height as f32;

        let mut output = FloatImage::new(width, height, data.channels());
        let mut acc = vec![0.0f32; nch];
        for y in 0..height {
            for x in 0..width {
                // Work in pixel-centre coordinates (pixel (x,y) sits at x+0.5),
                // so the centre pivot and integer offsets are both exact.
                // Inverse map: forward is  o = t + centre + R·S·(s − centre),
                // so  s = centre + Sinv·Rinv·(o − t − centre).
                let dx = x as f32 + 0.5 - offset_x_px - cx;
                let dy = y as f32 + 0.5 - offset_y_px - cy;
                // Rinv = R(−θ) (rotation matrices are orthonormal), then unscale.
                let ux = (cos_t * dx + sin_t * dy) / sx;
                let uy = (-sin_t * dx + cos_t * dy) / sy;
                // Back to a pixel index for sampling (undo the +0.5).
                let src_x = cx + ux - 0.5;
                let src_y = cy + uy - 0.5;

                sample_bilinear(&src, src_x, src_y, edge, &fill_px, &mut acc);
                output.put_pixel(x, y, &acc);
            }
        }

        // Back to straight alpha for downstream nodes / display.
        if premul { output.unpremultiply_alpha(); }

        Ok(OperationResponse {
            time: Instant::now().duration_since(start_time),
            responses: vec![
                OutputResponse { value: Value::Image { data: Arc::new(output), change_id: get_id() } },
            ],
        })
    }
}

/// Remaps a source coordinate onto `[0, n)` per the edge rule, or returns
/// `None` when it falls outside in fill mode (`0`).
fn remap(i: i64, n: i64, edge: EdgeMode) -> Option<i64> {
    match edge {
        EdgeMode::Wrap => Some(((i % n) + n) % n),
        EdgeMode::Extend => Some(i.clamp(0, n - 1)),
        EdgeMode::Mirror => {
            // reflect without repeating the edge pixel
            let p = 2 * n;
            let m = ((i % p) + p) % p;
            Some(if m >= n { p - 1 - m } else { m })
        }
        EdgeMode::Fill => if i < 0 || i >= n { None } else { Some(i) },
    }
}

/// Bilinear sample of `src` at `(sx, sy)` into `out`, applying `edge` handling
/// to each of the four taps. Out-of-range taps in fill mode contribute `fill`.
fn sample_bilinear(src: &FloatImage, sx: f32, sy: f32, edge: EdgeMode, fill: &[f32], out: &mut [f32]) {
    let w = src.width() as i64;
    let h = src.height() as i64;
    let x0f = sx.floor();
    let y0f = sy.floor();
    let fx = sx - x0f;
    let fy = sy - y0f;
    let x0 = x0f as i64;
    let y0 = y0f as i64;

    for v in out.iter_mut() {
        *v = 0.0;
    }

    // (tap x, tap y, weight); zero-weight taps are skipped so exact integer
    // positions read a single pixel and stay crisp (and avoid stray edge reads).
    let taps = [
        (x0, y0, (1.0 - fx) * (1.0 - fy)),
        (x0 + 1, y0, fx * (1.0 - fy)),
        (x0, y0 + 1, (1.0 - fx) * fy),
        (x0 + 1, y0 + 1, fx * fy),
    ];

    for (tx, ty, weight) in taps {
        if weight == 0.0 {
            continue;
        }
        let px = remap(tx, w, edge);
        let py = remap(ty, h, edge);
        let pixel: &[f32] = match (px, py) {
            (Some(cx), Some(cy)) => src.get_pixel(cx as u32, cy as u32),
            _ => fill, // fill mode, tap outside the image
        };
        for (o, s) in out.iter_mut().zip(pixel.iter()) {
            *o += s * weight;
        }
    }
}

#[cfg(test)]
#[path = "transform_tests.rs"]
mod tests;
