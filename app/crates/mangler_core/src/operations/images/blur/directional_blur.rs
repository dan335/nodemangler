//! Directional blur operation for images.
//!
//! Blurs the image along a specified angle by averaging multiple bilinearly
//! sampled points distributed along a line centered at each pixel.
//! Works directly on [`FloatImage`] f32 data.

use crate::curve::Curve;
use crate::float_image::FloatImage;
use crate::get_id;
use crate::value::ValueType;
use crate::input::{Input, InputSettings};
use crate::node_settings::NodeSettings;
use crate::operations::{OperationResponse, OperationError, OutputResponse, default_image, convert_input, scale_to_resolution};
use crate::output::Output;
use crate::value::Value;
use rayon::prelude::*;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use std::time::Instant;

/// Directional blur operation that smears the image along a specified angle.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpImageAdjustmentDirectionalBlur {}

impl OpImageAdjustmentDirectionalBlur {
    /// Returns the node metadata (name and description) for the directional blur operation.
    pub fn settings() -> NodeSettings {
        NodeSettings {
            name: "directional blur".to_string(),
            description: "Blurs an image along a specified angle.".to_string(),
            help: "Samples the image at equally spaced positions along a line of length 2 * intensity centred on each output pixel, then averages them. The line direction is (cos(angle), sin(angle)) with angle in degrees counter-clockwise from +X, so 0 smears horizontally and 90 smears vertically.\n\nSamples are bilinear so sub-pixel offsets produce smooth results. Higher sample counts yield smoother motion trails but cost linearly more work. Work is parallelised across rows via rayon. Intensity 0 or one sample returns the image unchanged (each tap lands on the centre pixel). Intensity is measured in pixels at a 1024px reference and scales with the image, so the blur looks the same at any resolution.\n\nPath mode: draw a curve on the path input (shown as an overlay in the 2D preview while this node is selected) to smear along that shape instead of a straight line. The taps are spaced equally by arc length along the path, centred on its arc-length midpoint, and the whole path is scaled so its length spans 2 * intensity — the angle input is ignored while a path is drawn. Reset the path input to its default (or collapse it to a zero-length point) to return to angle mode.".to_string(),
        }
    }

    /// Creates the input ports: image, angle (degrees), sample count, intensity
    /// (pixel spread), and an optional spatial path curve for curved smears.
    pub fn create_inputs() -> Vec<Input> {
        vec![
            Input::new("image".to_string(), Value::Image { data: default_image(), change_id: get_id() }, None, None)
                .with_description("Source image to smear along a line."),
            Input::new("angle".to_string(), Value::Decimal(0.0), Some(InputSettings::Slider { range: (0.0, 360.0), step_by: Some(1.0), clamp_to_range: true }), None)
                .with_description("Direction of the blur line in degrees, measured counter-clockwise from +X. Ignored while a path is drawn."),
            Input::new("samples".to_string(), Value::Integer(10), Some(InputSettings::DragValue { speed: None, clamp: Some((1.0, 100.0)) }), None)
                .with_description("Number of taps averaged along the blur line; higher values are smoother but slower."),
            Input::new("intensity".to_string(), Value::Decimal(10.0), Some(InputSettings::Slider { range: (0.0, 100.0), step_by: Some(0.5), clamp_to_range: true }), None)
                .with_description("Half-length of the blur line in pixels at a 1024px reference (scales with image size, so the effect looks the same at any resolution)."),
            Input::new("path".to_string(), Value::Curve(Curve::default()), None, None)
                .with_description("Optional smear path, edited as an overlay in the 2D preview. While drawn (changed from the default arc) the blur follows this shape instead of the angle line; taps span 2 * intensity of arc length centred on the path's midpoint. Reset to default to return to angle mode."),
        ]
    }

    /// Creates the output port: the directionally blurred image.
    pub fn create_outputs() -> Vec<Output> {
        vec![
            Output::new("output".to_string(), Value::Image { data: default_image(), change_id: get_id() }, None)
                .with_description("Image smeared along the configured direction."),
        ]
    }

    /// Executes the directional blur. Samples are distributed symmetrically along
    /// the angle direction — or along the drawn path when one is set — using
    /// FloatImage's bilinear interpolation.
    pub async fn run(inputs: &mut [Input]) -> Result<OperationResponse, OperationError> {
        let start_time = Instant::now();
        let mut input_errors: Vec<(usize, String)> = vec![];

        // convert inputs
        let image_converted = convert_input(inputs, 0, ValueType::Image, &mut input_errors);
        let angle_converted = convert_input(inputs, 1, ValueType::Decimal, &mut input_errors);
        let samples_converted = convert_input(inputs, 2, ValueType::Integer, &mut input_errors);
        let intensity_converted = convert_input(inputs, 3, ValueType::Decimal, &mut input_errors);
        let path_converted = convert_input(inputs, 4, ValueType::Curve, &mut input_errors);

        // return if error
        if !input_errors.is_empty() { return Err(OperationError { input_errors, node_error: None }); }

        // get values
        let Value::Image { data, change_id: _ } = image_converted.unwrap() else { unreachable!() };
        let Value::Decimal(angle) = angle_converted.unwrap() else { unreachable!() };
        let Value::Integer(samples) = samples_converted.unwrap() else { unreachable!() };
        let Value::Decimal(intensity) = intensity_converted.unwrap() else { unreachable!() };
        let Value::Curve(path) = path_converted.unwrap() else { unreachable!() };

        // run node
        let samples = samples.max(1) as u32;
        let (width, height) = data.dimensions();
        // Intensity is authored in reference pixels (at 1024px) and scaled to the
        // actual image, so the same value blurs the same amount relative to the
        // content at any resolution.
        let intensity = scale_to_resolution(intensity.max(0.0), width, height);

        // Per-tap pixel offsets, shared by every output pixel. Path mode kicks
        // in only when the curve has been edited away from its untouched
        // default; a degenerate drawn path (zero arc length) falls back to the
        // straight angle line so the node never errors.
        let offsets: Vec<[f32; 2]> = if path != Curve::default() {
            path_offsets(&path, width, height, samples, intensity)
        } else {
            None
        }
        .unwrap_or_else(|| {
            let angle_rad = angle.to_radians();
            let dx = angle_rad.cos();
            let dy = angle_rad.sin();
            (0..samples)
                .map(|i| {
                    let t = if samples > 1 {
                        (i as f32 / (samples - 1) as f32) * 2.0 - 1.0
                    } else {
                        0.0
                    };
                    let offset = t * intensity;
                    [dx * offset, dy * offset]
                })
                .collect()
        });
        let offsets_ref = &offsets;

        let ch = data.channels() as usize;
        let data_ref = &data;
        let h = height as usize;
        let w = width as usize;

        // Process each row in parallel, accumulating bilinear samples per pixel
        let pixels: Vec<f32> = (0..h).into_par_iter().flat_map_iter(move |y| {
            // Thread-local sample buffer to avoid per-pixel allocation
            let mut sample = vec![0.0f32; ch];
            let mut row_pixels = Vec::with_capacity(w * ch);

            for x in 0..w {
                // channels are always <= 4, so a stack array avoids a
                // per-pixel heap allocation
                let mut sums = [0.0f64; 4];

                // Sample the precomputed tap offsets, centered on this pixel
                for off in offsets_ref {
                    let sx = x as f32 + off[0];
                    let sy = y as f32 + off[1];
                    data_ref.bilinear_sample(sx, sy, &mut sample);
                    for c in 0..ch {
                        sums[c] += sample[c] as f64;
                    }
                }

                // Average across all samples
                let count = offsets_ref.len() as f64;
                for val in sums.iter().take(ch) {
                    row_pixels.push((val / count) as f32);
                }
            }
            row_pixels
        }).collect();

        // Build the output FloatImage from the computed pixel buffer
        let output = FloatImage::from_raw(width, height, data.channels(), pixels).unwrap();

        Ok(OperationResponse { 
            time: Instant::now().duration_since(start_time),
            responses: vec![
                OutputResponse { value: Value::Image { data: Arc::new(output), change_id: get_id() } },
            ],
        })
    }
}

/// Samples per spline segment when flattening the path curve. Matches the
/// standard tolerance used by `Curve`'s own arc-length helpers; far denser
/// than the tap count, so equal-arc-length spacing is accurate.
const PATH_FLATTEN_SAMPLES: usize = 48;

/// Builds the per-tap pixel offsets for path mode: `samples` points spaced
/// equally by arc length along the flattened path (in pixel space), centred on
/// the arc-length midpoint (which maps to a zero offset) and scaled so the
/// path's total length spans `2 * intensity` pixels — the same total span as
/// the straight angle line.
///
/// Returns `None` for a degenerate path (fewer than 2 points after flattening,
/// or ~zero arc length), signalling the caller to fall back to angle mode.
fn path_offsets(
    path: &Curve,
    width: u32,
    height: u32,
    samples: u32,
    intensity: f32,
) -> Option<Vec<[f32; 2]>> {
    // Flatten to pixel space so arc length weights x and y like the image does
    // (a normalized curve on a non-square image is stretched accordingly).
    let poly: Vec<[f32; 2]> = path
        .flatten(PATH_FLATTEN_SAMPLES)
        .iter()
        .map(|p| [p[0] * width as f32, p[1] * height as f32])
        .collect();
    if poly.len() < 2 {
        return None;
    }

    // Cumulative arc length along the polyline.
    let mut cum = Vec::with_capacity(poly.len());
    cum.push(0.0f32);
    for seg in poly.windows(2) {
        let dx = seg[1][0] - seg[0][0];
        let dy = seg[1][1] - seg[0][1];
        let prev = *cum.last().unwrap();
        cum.push(prev + (dx * dx + dy * dy).sqrt());
    }
    let total = *cum.last().unwrap();
    if total <= 1e-6 {
        return None;
    }

    // Point on the polyline at arc length `s` (clamped), by linear scan —
    // both the polyline and the tap count are small, so this stays cheap.
    let point_at = |s: f32| -> [f32; 2] {
        let s = s.clamp(0.0, total);
        for (i, seg) in poly.windows(2).enumerate() {
            if cum[i + 1] >= s {
                let seg_len = cum[i + 1] - cum[i];
                let local = if seg_len > 0.0 { (s - cum[i]) / seg_len } else { 0.0 };
                return [
                    seg[0][0] + local * (seg[1][0] - seg[0][0]),
                    seg[0][1] + local * (seg[1][1] - seg[0][1]),
                ];
            }
        }
        *poly.last().unwrap()
    };

    // Scale the whole tap set so the path's arc length spans 2 * intensity,
    // and centre it on the arc-length midpoint (zero offset there).
    let scale = 2.0 * intensity / total;
    let mid = point_at(total * 0.5);
    let offsets = (0..samples)
        .map(|i| {
            let t = if samples > 1 {
                i as f32 / (samples - 1) as f32
            } else {
                0.5
            };
            let p = point_at(t * total);
            [(p[0] - mid[0]) * scale, (p[1] - mid[1]) * scale]
        })
        .collect();
    Some(offsets)
}

#[cfg(test)]
#[path = "directional_blur_tests.rs"]
mod tests;
