//! Bevel from a mask: produce a beveled height (or derived normal map).
//!
//! Converts a binary-ish mask into a ramp that slopes up from the outside of
//! the shape toward its interior, reaching `1.0` once pixels are at least
//! `distance` pixels away from the mask boundary. `smoothing` rounds the
//! ramp; `corner_type` picks between an angular or round falloff profile.
//!
//! Implementation: threshold the mask into inside/outside, then for each
//! inside pixel find the nearest boundary pixel within `distance`, normalise
//! to `[0, 1]`, apply a curve. Output is either the height itself or a
//! Sobel-derived normal map over that height.

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

/// Bevel operation — converts a mask into a beveled height or normal map.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpImagePbrBevel {}

impl OpImagePbrBevel {
    pub fn settings() -> NodeSettings {
        NodeSettings {
            name: "bevel".to_string(),
            description: "Produces a beveled height/normal from a mask using a distance-field ramp.".to_string(),
            help: "Thresholds the input mask into inside/outside pixels, then for each inside pixel finds the Euclidean distance to the nearest outside pixel within the configured search window. That distance is normalised against distance (in pixels), shaped by the corner profile (round sin-curve or angular linear), and blended toward a smoothstep by smoothing.\n\nIn height mode (default) the output is a single-channel height field; in normal mode a Sobel operator is run over the internal height to produce a tangent-space normal map whose apparent strength scales inversely with distance so wider bevels give gentler normals.".to_string(),
        }
    }

    pub fn create_inputs() -> Vec<Input> {
        vec![
            Input::new("mask".to_string(), Value::Image { data: default_image(), change_id: get_id() }, None, None)
                .with_description("Source mask that defines the shape to be beveled."),
            Input::new("distance".to_string(), Value::Decimal(16.0), Some(InputSettings::DragValue { speed: None, clamp: Some((1.0, 256.0)) }), None)
                .with_description("Bevel width in pixels, measured inward from the mask edge."),
            Input::new("smoothing".to_string(), Value::Decimal(0.5), Some(InputSettings::Slider { range: (0.0, 1.0), step_by: Some(0.01), clamp_to_range: true }), None)
                .with_description("Blends the ramp toward a smoothstep curve for softer bevels."),
            // 0 = round (sin curve), 1 = angular (linear)
            Input::new("corner type".to_string(), Value::Integer(0), Some(InputSettings::Slider { range: (0.0, 1.0), step_by: Some(1.0), clamp_to_range: true }), None)
                .with_description("Profile shape: 0 rounded (sin curve), 1 angular (linear)."),
            // 0 = height, 1 = normal
            Input::new("output mode".to_string(), Value::Integer(0), Some(InputSettings::Slider { range: (0.0, 1.0), step_by: Some(1.0), clamp_to_range: true }), None)
                .with_description("Output type: 0 height map, 1 derived normal map."),
            Input::new("threshold".to_string(), Value::Decimal(0.5), Some(InputSettings::Slider { range: (0.0, 1.0), step_by: Some(0.01), clamp_to_range: true }), None)
                .with_description("Luminance cutoff used to split the mask into inside and outside pixels."),
        ]
    }

    pub fn create_outputs() -> Vec<Output> {
        vec![
            Output::new("output".to_string(), Value::Image { data: default_image(), change_id: get_id() }, None)
                .with_description("Beveled height map, or derived normal map when output mode is 1."),
        ]
    }

    pub async fn run(inputs: &mut [Input]) -> Result<OperationResponse, OperationError> {
        let start_time = Instant::now();
        let mut input_errors: Vec<(usize, String)> = vec![];

        let mask_converted = convert_input(inputs, 0, ValueType::Image, &mut input_errors);
        let distance_converted = convert_input(inputs, 1, ValueType::Decimal, &mut input_errors);
        let smoothing_converted = convert_input(inputs, 2, ValueType::Decimal, &mut input_errors);
        let corner_converted = convert_input(inputs, 3, ValueType::Integer, &mut input_errors);
        let mode_converted = convert_input(inputs, 4, ValueType::Integer, &mut input_errors);
        let threshold_converted = convert_input(inputs, 5, ValueType::Decimal, &mut input_errors);

        if !input_errors.is_empty() { return Err(OperationError { input_errors, node_error: None }); }

        let Value::Image { data, change_id: _ } = mask_converted.unwrap() else { unreachable!() };
        let Value::Decimal(distance) = distance_converted.unwrap() else { unreachable!() };
        let Value::Decimal(smoothing) = smoothing_converted.unwrap() else { unreachable!() };
        let Value::Integer(corner) = corner_converted.unwrap() else { unreachable!() };
        let Value::Integer(mode) = mode_converted.unwrap() else { unreachable!() };
        let Value::Decimal(threshold) = threshold_converted.unwrap() else { unreachable!() };

        let distance = distance.max(1.0);
        let smoothing = smoothing.clamp(0.0, 1.0);

        let (width, height) = data.dimensions();
        let w = width as usize;
        let h = height as usize;
        let ch = data.channels() as usize;

        // Threshold mask → inside/outside lookup.
        let inside: Vec<bool> = (0..h).flat_map(|y| {
            let data = &*data;
            (0..w).map(move |x| {
                let p = data.get_pixel(x as u32, y as u32);
                let lum = if ch >= 3 {
                    0.2126 * p[0] + 0.7152 * p[1] + 0.0722 * p[2]
                } else {
                    p[0]
                };
                lum >= threshold
            })
        }).collect();

        // Per-pixel bevel height: 0 outside, ramps up to 1 toward shape interior.
        let inside_ref = &inside;
        let dist_i = distance.ceil() as i32;
        let heights: Vec<f32> = (0..h).into_par_iter().flat_map_iter(move |y| {
            (0..w).map(move |x| {
                let idx = y * w + x;
                if !inside_ref[idx] {
                    return 0.0f32;
                }
                // Find distance to nearest outside pixel within the search window.
                let mut min_d2 = distance * distance;
                let y_start = (y as i32 - dist_i).max(0) as usize;
                let y_end = ((y as i32 + dist_i) as usize).min(h - 1);
                let x_start = (x as i32 - dist_i).max(0) as usize;
                let x_end = ((x as i32 + dist_i) as usize).min(w - 1);
                for sy in y_start..=y_end {
                    for sx in x_start..=x_end {
                        if !inside_ref[sy * w + sx] {
                            let dx = sx as f32 - x as f32;
                            let dy = sy as f32 - y as f32;
                            let d2 = dx * dx + dy * dy;
                            if d2 < min_d2 { min_d2 = d2; }
                        }
                    }
                }
                let d = min_d2.sqrt();
                let t = (d / distance).clamp(0.0, 1.0);
                // Corner profile.
                let shaped = match corner {
                    1 => t, // angular
                    _ => (t * std::f32::consts::FRAC_PI_2).sin(), // round
                };
                // Smoothing nudges the ramp toward a smoothstep.
                let smoothed = shaped * shaped * (3.0 - 2.0 * shaped);
                shaped * (1.0 - smoothing) + smoothed * smoothing
            })
        }).collect();

        let height_img = FloatImage::from_raw(width, height, 1, heights).unwrap();

        if mode != 1 {
            return Ok(OperationResponse {
                time: Instant::now().duration_since(start_time),
                responses: vec![
                    OutputResponse { value: Value::Image { data: Arc::new(height_img), change_id: get_id() } },
                ],
            });
        }

        // Normal mode — Sobel across the height image, packed to RGBA.
        let sample = |x: i32, y: i32| -> f32 {
            let cx = x.clamp(0, width as i32 - 1) as u32;
            let cy = y.clamp(0, height as i32 - 1) as u32;
            height_img.get_pixel(cx, cy)[0]
        };
        let mut normal = FloatImage::new(width, height, 4);
        // Intensity lines the normal up with the chosen bevel distance:
        // larger bevels produce gentler normals.
        let intensity = (4.0 / distance.max(1.0)).max(0.1);
        for y in 0..height as i32 {
            for x in 0..width as i32 {
                let tl = sample(x - 1, y - 1);
                let top = sample(x, y - 1);
                let tr = sample(x + 1, y - 1);
                let left = sample(x - 1, y);
                let right = sample(x + 1, y);
                let bl = sample(x - 1, y + 1);
                let bottom = sample(x, y + 1);
                let br = sample(x + 1, y + 1);
                let dx = ((tr + 2.0 * right + br) - (tl + 2.0 * left + bl)) * intensity;
                let dy = ((bl + 2.0 * bottom + br) - (tl + 2.0 * top + tr)) * intensity;
                let nx = -dx;
                let ny = -dy;
                let nz = 1.0f32;
                let len = (nx * nx + ny * ny + nz * nz).sqrt();
                normal.put_pixel(x as u32, y as u32, &[
                    (nx / len) * 0.5 + 0.5,
                    (ny / len) * 0.5 + 0.5,
                    (nz / len) * 0.5 + 0.5,
                    1.0,
                ]);
            }
        }

        Ok(OperationResponse {
            time: Instant::now().duration_since(start_time),
            responses: vec![
                OutputResponse { value: Value::Image { data: Arc::new(normal), change_id: get_id() } },
            ],
        })
    }
}

#[cfg(test)]
#[path = "bevel_tests.rs"]
mod tests;
