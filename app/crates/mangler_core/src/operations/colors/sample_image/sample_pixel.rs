//! Sample a pixel (or small disk of pixels) color from an image.
//!
//! Reads the color at a normalized (x, y) position. With `diameter` 1 (the
//! default) this is a single bilinear sample — identical to the original
//! one-pixel behaviour. Larger diameters average every source pixel whose
//! centre falls inside a disk of that diameter, which is the right filter for
//! a multi-pixel eyedropper (suppresses sensor noise / demosaic texture).
//! Lives under `colors` because its headline output is a Color.

use crate::color::Color;
use crate::float_image::FloatImage;
use crate::get_id;
use crate::input::{Input, InputSettings};
use crate::node_settings::NodeSettings;
use crate::operations::{OperationResponse, OperationError, OutputResponse, default_image, convert_input};
use crate::output::Output;
use crate::value::{Value, ValueType};
use serde::{Deserialize, Serialize};
use std::time::Instant;

/// Operation that samples a color from an image at a normalized position.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpColorSampleSamplePixel {}

impl OpColorSampleSamplePixel {
    /// Returns the node metadata (name, description, help).
    pub fn settings() -> NodeSettings {
        NodeSettings {
            name: "sample pixel".to_string(),
            description: "Samples the color at a normalized (x, y) position in an image, optionally averaged over a disk.".to_string(),
            help: "Reads the color at a normalized position, where x and y run from 0 (left/top) to 1 (right/bottom). Emits the combined color plus its red, green, blue, and alpha components.\n\n`diameter` is the sample size in **source-image pixels**. At the default of 1 the node bilinear-samples a single point (sub-pixel accurate) — the original behaviour. At 2 or more it averages every pixel whose centre lies inside a disk of that diameter centred on (x, y), which is what photo eyedroppers use to kill sensor noise and demosaic texture. RGB is averaged premultiplied by alpha so transparent pixels don't bleed hidden colour into the result; alpha is a plain mean. Positions are clamped to [0, 1]; the disk is clipped at the image edge rather than wrapped.\n\nSingle-channel images are broadcast to gray; images without an alpha channel report alpha as 1.".to_string(),
        }
    }

    /// Creates the input ports: image, normalized x/y, and sample diameter.
    pub fn create_inputs() -> Vec<Input> {
        vec![
            Input::new("image".to_string(), Value::Image { data: default_image(), change_id: get_id() }, None, None)
                .with_description("Image to sample a pixel color from."),
            Input::new("x".to_string(), Value::Decimal(0.5), Some(InputSettings::Slider { range: (0.0, 1.0), step_by: None, clamp_to_range: true }), None)
                .with_description("Horizontal position, 0 (left) to 1 (right)."),
            Input::new("y".to_string(), Value::Decimal(0.5), Some(InputSettings::Slider { range: (0.0, 1.0), step_by: None, clamp_to_range: true }), None)
                .with_description("Vertical position, 0 (top) to 1 (bottom)."),
            Input::new("diameter".to_string(), Value::Integer(1), Some(InputSettings::Slider { range: (1.0, 64.0), step_by: Some(1.0), clamp_to_range: true }), None)
                .with_description("Sample disk diameter in source pixels; 1 = single bilinear sample (default), larger values average a circular neighbourhood."),
        ]
    }

    /// Creates the output ports: the sampled color and its RGBA components.
    pub fn create_outputs() -> Vec<Output> {
        vec![
            Output::new("color".to_string(), Value::Color(Color::default()), None)
                .with_description("Sampled color at the given position."),
            Output::new("red".to_string(), Value::Decimal(0.0), None)
                .with_description("Red channel of the sampled color."),
            Output::new("green".to_string(), Value::Decimal(0.0), None)
                .with_description("Green channel of the sampled color."),
            Output::new("blue".to_string(), Value::Decimal(0.0), None)
                .with_description("Blue channel of the sampled color."),
            Output::new("alpha".to_string(), Value::Decimal(1.0), None)
                .with_description("Alpha of the sampled color (1.0 when the image has no alpha channel)."),
        ]
    }

    /// Executes the pixel-sampling operation.
    pub async fn run(inputs: &mut [Input]) -> Result<OperationResponse, OperationError> {
        let start_time = Instant::now();
        let mut input_errors: Vec<(usize, String)> = vec![];

        let image_converted = convert_input(inputs, 0, ValueType::Image, &mut input_errors);
        let x_converted = convert_input(inputs, 1, ValueType::Decimal, &mut input_errors);
        let y_converted = convert_input(inputs, 2, ValueType::Decimal, &mut input_errors);
        let diameter_converted = convert_input(inputs, 3, ValueType::Integer, &mut input_errors);

        if !input_errors.is_empty() { return Err(OperationError { input_errors, node_error: None }); }

        let Value::Image { data, change_id: _ } = image_converted.unwrap() else { unreachable!() };
        let Value::Decimal(x) = x_converted.unwrap() else { unreachable!() };
        let Value::Decimal(y) = y_converted.unwrap() else { unreachable!() };
        let Value::Integer(diameter) = diameter_converted.unwrap() else { unreachable!() };

        let (w, h) = data.dimensions();
        let ch = data.channels() as usize;

        let px = x.clamp(0.0, 1.0) * (w.saturating_sub(1) as f32);
        let py = y.clamp(0.0, 1.0) * (h.saturating_sub(1) as f32);
        // Diameter is in source pixels; 1 keeps the historical single-sample path.
        let diameter = diameter.max(1);

        let (r, g, b, a) = if diameter <= 1 {
            sample_point(&data, px, py, ch)
        } else {
            sample_disk(&data, px, py, ch, diameter as f32)
        };

        let color = Color::from_srgb_float(r, g, b, a);

        Ok(OperationResponse {
            time: Instant::now().duration_since(start_time),
            responses: vec![
                OutputResponse { value: Value::Color(color) },
                OutputResponse { value: Value::Decimal(r) },
                OutputResponse { value: Value::Decimal(g) },
                OutputResponse { value: Value::Decimal(b) },
                OutputResponse { value: Value::Decimal(a) },
            ],
        })
    }
}

/// Single-point bilinear sample, promoted to RGBA (gray broadcast, alpha 1 if missing).
fn sample_point(data: &FloatImage, px: f32, py: f32, ch: usize) -> (f32, f32, f32, f32) {
    let mut buf = [0.0f32; 4];
    data.bilinear_sample(px, py, &mut buf[..ch]);
    match ch {
        1 => (buf[0], buf[0], buf[0], 1.0),
        2 => (buf[0], buf[0], buf[0], buf[1]),
        3 => (buf[0], buf[1], buf[2], 1.0),
        _ => (buf[0], buf[1], buf[2], buf[3]),
    }
}

/// Circular neighbourhood average. Includes every pixel whose centre is within
/// `diameter/2` of `(px, py)`. RGB is accumulated premultiplied so transparent
/// pixels cannot bleed hidden colour; alpha is a plain mean.
fn sample_disk(data: &FloatImage, px: f32, py: f32, ch: usize, diameter: f32) -> (f32, f32, f32, f32) {
    let (w, h) = data.dimensions();
    if w == 0 || h == 0 {
        return (0.0, 0.0, 0.0, 1.0);
    }

    let radius = diameter * 0.5;
    let r2 = radius * radius;
    // Bounding box of candidate pixel centres (integer coords).
    let x0 = ((px - radius).floor() as i32).max(0) as u32;
    let y0 = ((py - radius).floor() as i32).max(0) as u32;
    let x1 = ((px + radius).ceil() as i32).clamp(0, w as i32 - 1) as u32;
    let y1 = ((py + radius).ceil() as i32).clamp(0, h as i32 - 1) as u32;

    let mut sum_r = 0.0f64;
    let mut sum_g = 0.0f64;
    let mut sum_b = 0.0f64;
    let mut sum_a = 0.0f64;
    let mut sum_ra = 0.0f64; // premultiplied accumulators
    let mut sum_ga = 0.0f64;
    let mut sum_ba = 0.0f64;
    let mut count = 0u32;

    for y in y0..=y1 {
        for x in x0..=x1 {
            let dx = x as f32 - px;
            let dy = y as f32 - py;
            if dx * dx + dy * dy > r2 {
                continue;
            }
            let p = data.get_pixel(x, y);
            let (r, g, b, a) = match ch {
                1 => (p[0], p[0], p[0], 1.0),
                2 => (p[0], p[0], p[0], p[1]),
                3 => (p[0], p[1], p[2], 1.0),
                _ => (p[0], p[1], p[2], p[3]),
            };
            sum_ra += (r * a) as f64;
            sum_ga += (g * a) as f64;
            sum_ba += (b * a) as f64;
            sum_a += a as f64;
            // Keep unweighted channel sums only for the zero-alpha edge case
            // where premultiplied recovery is undefined.
            sum_r += r as f64;
            sum_g += g as f64;
            sum_b += b as f64;
            count += 1;
        }
    }

    if count == 0 {
        // Degenerate (e.g. sample between pixels with a tiny diameter that
        // still routed here): fall back to the centre point.
        return sample_point(data, px, py, ch);
    }

    let n = count as f64;
    let a = (sum_a / n) as f32;
    if sum_a > 1e-9 {
        (
            (sum_ra / sum_a) as f32,
            (sum_ga / sum_a) as f32,
            (sum_ba / sum_a) as f32,
            a,
        )
    } else {
        // Fully transparent neighbourhood: report mean of the (hidden) colours
        // with alpha 0 rather than NaN.
        (
            (sum_r / n) as f32,
            (sum_g / n) as f32,
            (sum_b / n) as f32,
            0.0,
        )
    }
}

#[cfg(test)]
#[path = "sample_pixel_tests.rs"]
mod tests;
