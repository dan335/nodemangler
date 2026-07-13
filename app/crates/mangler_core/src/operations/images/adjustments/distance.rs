//! Distance field computation for images.
//!
//! Converts a grayscale image into a signed distance field by thresholding
//! pixels into inside/outside regions, then computing the exact Euclidean
//! distance to the nearest pixel of the opposite class with the shared
//! Felzenszwalb-Huttenlocher transform (`simulation::distance_field_labeled`).
//! Output is normalized with 0.5 at the boundary, values above 0.5 for inside
//! regions, and below 0.5 for outside.

use crate::float_image::FloatImage;
use crate::get_id;
use crate::value::ValueType;
use rayon::prelude::*;
use crate::input::{Input, InputSettings};
use crate::node_settings::NodeSettings;
use crate::operations::images::simulation::distance_field_labeled;
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
            help: "Thresholds the input luminance (Rec. 709 for RGB, or single channel for grayscale) into inside/outside regions, then computes the exact Euclidean distance from every pixel to the nearest pixel of the opposite class using a Felzenszwalb-Huttenlocher distance transform (O(1) per pixel, independent of spread).\n\nThe distance is normalised by spread so the boundary becomes 0.5, inside pixels range from 0.5 to 1, and outside pixels from 0 to 0.5; distances beyond spread clamp to the extremes. Output is always a 4-channel grayscale RGBA image. Useful as a basis for glow, outline, or soft-mask effects.".to_string(),
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

    /// Executes the distance field computation using an exact Euclidean distance
    /// transform (two Felzenszwalb-Huttenlocher passes, one per class).
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
        let w = data.width() as usize;
        let h = data.height() as usize;
        let ch = data.channels() as usize;

        // threshold the image: compute binary mask from luminance
        let data_ref = &*data;
        let inside: Vec<bool> = (0..h).into_par_iter().flat_map_iter(|y| {
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

        // Exact Euclidean distance to the nearest opposite-class pixel via two
        // separable transforms: outside pixels read the distance-to-inside
        // field, inside pixels the distance-to-outside field. O(w*h) total,
        // independent of spread; an empty class leaves its field at the DT_INF
        // sentinel, which the spread normalisation clamps to the extreme.
        let outside: Vec<bool> = inside.iter().map(|b| !b).collect();
        let (d2_to_inside, _) = distance_field_labeled(&inside, w, h);
        let (d2_to_outside, _) = distance_field_labeled(&outside, w, h);

        let inside_ref = &inside;
        let d2_in_ref = &d2_to_inside;
        let d2_out_ref = &d2_to_outside;
        let pixels: Vec<f32> = (0..w * h).into_par_iter().flat_map_iter(move |idx| {
            let is_inside = inside_ref[idx];
            let dist_sq = if is_inside { d2_out_ref[idx] } else { d2_in_ref[idx] };
            let dist = (dist_sq as f32).sqrt();
            let normalized_dist = (dist / spread).clamp(0.0, 1.0);
            let result = if is_inside {
                0.5 + normalized_dist / 2.0
            } else {
                0.5 - normalized_dist / 2.0
            }.clamp(0.0, 1.0);

            [result, result, result, 1.0]
        }).collect();

        let output = FloatImage::from_raw(w as u32, h as u32, 4, pixels)
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
