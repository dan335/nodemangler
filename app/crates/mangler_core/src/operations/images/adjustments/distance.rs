//! Distance field computation for images.
//!
//! Converts a grayscale image into a signed distance field by thresholding
//! pixels into inside/outside regions, then computing the minimum Euclidean
//! distance to the nearest boundary pixel. Output is normalized with 0.5 at
//! the boundary, values above 0.5 for inside regions, and below 0.5 for outside.

use crate::float_image::FloatImage;
use crate::get_id;
use crate::value::ValueType;
use rayon::prelude::*;
use crate::input::{Input, InputSettings};
use crate::node_settings::NodeSettings;
use crate::operations::{OperationResponse, OperationError, OutputResponse, default_image, convert_input, scale_to_resolution};
use crate::output::Output;
use crate::value::Value;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use std::time::Instant;

/// Distance field operation that computes a signed distance from a binary threshold.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpImageAdjustmentDistance{}

impl OpImageAdjustmentDistance {
    /// Returns the node metadata (name and description) for the distance operation.
    pub fn settings() -> NodeSettings {
        NodeSettings {
            name: "distance field".to_string(),
            description: "Computes a signed distance field from a binary (black/white) image.".to_string(),
            help: "Thresholds the input luminance (Rec. 709 for RGB, or single channel for grayscale) into inside/outside regions, then brute-force searches a square window of radius spread to find the nearest pixel of the opposite class.\n\nThe Euclidean distance is normalised by spread so the boundary becomes 0.5, inside pixels range from 0.5 to 1, and outside pixels from 0 to 0.5. Larger spread values produce smoother, softer fields but cost O(spread^2) per pixel; the loop short-circuits when it finds a neighbour at distance less than 1. Output is always a 4-channel grayscale RGBA image. Useful as a basis for glow, outline, or soft-mask effects.".to_string(),
        }
    }

    /// Creates the input ports: image, luminance threshold for the binary mask, and spread
    /// (maximum search radius in pixels).
    pub fn create_inputs() -> Vec<Input> {
        vec![
            Input::new("image".to_string(),  Value::Image { data:default_image(), change_id:get_id() }, None, None)
                .with_description("Source image; its luminance is thresholded into inside/outside regions."),
            Input::new("threshold".to_string(), Value::Decimal(0.5), Some(InputSettings::Slider { range: (0.0, 1.0), step_by: Some(0.01), clamp_to_range: true }), None)
                .with_description("Luminance cutoff separating inside (above) from outside (below)."),
            Input::new("spread".to_string(), Value::Decimal(32.0), Some(InputSettings::DragValue { speed: None, clamp: Some((1.0, 256.0)) }), None)
                .with_description("Maximum search radius in pixels at a 1024px reference (scales with image size); caps how far the distance field extends."),
        ]
    }

    /// Creates the output port: the distance field image.
    pub fn create_outputs() -> Vec<Output> {
        vec![
            Output::new("output".to_string(), Value::Image { data:default_image(), change_id:get_id()}, None)
                .with_description("Signed distance field centred on 0.5 at the boundary, >0.5 inside, <0.5 outside."),
        ]
    }

    /// Executes the distance field computation using brute-force nearest-boundary search
    /// within the spread radius.
    pub async fn run(inputs: &mut [Input]) -> Result<OperationResponse, OperationError> {
        let start_time = Instant::now();
        let mut input_errors: Vec<(usize, String)> = vec![];

        // convert inputs
        let image_converted = convert_input(inputs, 0, ValueType::Image, &mut input_errors);
        let threshold_converted = convert_input(inputs, 1, ValueType::Decimal, &mut input_errors);
        let spread_converted = convert_input(inputs, 2, ValueType::Decimal, &mut input_errors);

        // return if error
        if !input_errors.is_empty() { return Err(OperationError { input_errors, node_error: None }); }

        // get values
        let Value::Image{data, change_id:_} = image_converted.unwrap() else { unreachable!() };
        let Value::Decimal(threshold) = threshold_converted.unwrap() else { unreachable!() };
        let Value::Decimal(spread) = spread_converted.unwrap() else { unreachable!() };

        // run node — work directly on FloatImage data
        // Spread is authored in reference pixels (at 1024px) and scaled to the
        // actual image, so the field extends the same relative distance at any
        // resolution.
        let spread = scale_to_resolution(spread.max(0.0), data.width(), data.height()).max(1.0);
        let width = data.width() as i32;
        let height = data.height() as i32;
        let w = width as usize;
        let h = height as usize;
        let spread_i = spread.ceil() as i32;
        let ch = data.channels() as usize;

        // threshold the image: compute binary mask from luminance
        let data_ref = &*data;
        let inside: Vec<bool> = (0..h).flat_map(|y| {
            (0..w).map(move |x| {
                let px = data_ref.get_pixel(x as u32, y as u32);
                let lum = if ch >= 3 {
                    0.2126 * px[0] + 0.7152 * px[1] + 0.0722 * px[2]
                } else {
                    px[0]
                };
                lum >= threshold
            })
        }).collect();

        // Compute distance transform and output image in parallel
        let inside_ref = &inside;

        let pixels: Vec<f32> = (0..h).into_par_iter().flat_map_iter(move |y| {
            (0..w).flat_map(move |x| {
                let idx = y * w + x;
                let is_inside = inside_ref[idx];
                let mut min_dist_sq = spread * spread;

                let y_start = (y as i32 - spread_i).max(0) as usize;
                let y_end = ((y as i32 + spread_i).min(height - 1)) as usize;
                let x_start = (x as i32 - spread_i).max(0) as usize;
                let x_end = ((x as i32 + spread_i).min(width - 1)) as usize;

                'outer: for sy in y_start..=y_end {
                    for sx in x_start..=x_end {
                        let sidx = sy * w + sx;
                        if inside_ref[sidx] != is_inside {
                            let ddx = (sx as f32) - (x as f32);
                            let ddy = (sy as f32) - (y as f32);
                            let dist_sq = ddx * ddx + ddy * ddy;
                            if dist_sq < min_dist_sq {
                                min_dist_sq = dist_sq;
                                if dist_sq <= 1.0 { break 'outer; }
                            }
                        }
                    }
                }

                let dist = min_dist_sq.sqrt();
                let normalized_dist = (dist / spread).clamp(0.0, 1.0);
                let result = if is_inside {
                    0.5 + normalized_dist / 2.0
                } else {
                    0.5 - normalized_dist / 2.0
                }.clamp(0.0, 1.0);

                [result, result, result, 1.0]
            })
        }).collect();

        let output = FloatImage::from_raw(width as u32, height as u32, 4, pixels)
            .expect("distance field pixel count mismatch");

        Ok(OperationResponse { 
            time: Instant::now().duration_since(start_time),
            responses: vec![
                OutputResponse {value: Value::Image { data:Arc::new(output), change_id:get_id() }},
            ],
        })
    }
}

#[cfg(test)]
#[path = "distance_tests.rs"]
mod tests;
