//! Compare operation.
//!
//! Compares two images pixel-by-pixel and outputs a greyscale difference map.
//! Black (0.0) means identical pixels, white (1.0) means maximally different,
//! and grey values indicate partial differences proportional to the per-channel
//! distance between the two images.

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

/// Operation that compares two images and outputs a greyscale difference map.
///
/// For each pixel the operation computes the average absolute difference across
/// RGB channels. A `gain` multiplier amplifies small differences so they become
/// visible (defaults to 1.0, range 1–10).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpImageCombineCompare {}

impl OpImageCombineCompare {
    pub fn settings() -> NodeSettings {
        NodeSettings {
            name: "compare".to_string(),
            description: "Compares two images. Black = same, white = different, grey = slightly different.".to_string(),
        }
    }

    pub fn create_inputs() -> Vec<Input> {
        vec![
            // First image to compare.
            Input::new("image a".to_string(), Value::Image { data: default_image(), change_id: get_id() }, None, None),
            // Second image to compare.
            Input::new("image b".to_string(), Value::Image { data: default_image(), change_id: get_id() }, None, None),
            // Multiplier that amplifies small differences (1.0 = raw, higher = more visible).
            Input::new("gain".to_string(), Value::Decimal(1.0), Some(InputSettings::Slider { range: (1.0, 10.0), step_by: Some(0.1), clamp_to_range: true }), None),
        ]
    }

    pub fn create_outputs() -> Vec<Output> {
        vec![Output::new("output".to_string(), Value::Image { data: default_image(), change_id: get_id() }, None)]
    }

    /// Runs the comparison.
    ///
    /// Output size matches image A. Where image B is smaller, missing pixels are
    /// treated as black (0,0,0). Both images are read as RGBA; the per-pixel
    /// difference is `clamp(gain * mean(|Ra-Rb|, |Ga-Gb|, |Ba-Bb|), 0, 1)`.
    pub async fn run(inputs: &mut [Input]) -> Result<OperationResponse, OperationError> {
        let start_time = Instant::now();
        let mut input_errors: Vec<(usize, String)> = vec![];

        let a_converted = convert_input(inputs, 0, ValueType::Image, &mut input_errors);
        let b_converted = convert_input(inputs, 1, ValueType::Image, &mut input_errors);
        let gain_converted = convert_input(inputs, 2, ValueType::Decimal, &mut input_errors);

        if !input_errors.is_empty() {
            return Err(OperationError { input_errors, node_error: None });
        }

        let Value::Image { data: img_a, .. } = a_converted.unwrap() else { unreachable!() };
        let Value::Image { data: img_b, .. } = b_converted.unwrap() else { unreachable!() };
        let Value::Decimal(gain) = gain_converted.unwrap() else { unreachable!() };

        let (w, h) = img_a.dimensions();
        // Output is single-channel greyscale.
        let mut output = FloatImage::new(w, h, 1);

        // Helper: extract RGB from any channel count, defaulting missing channels.
        let get_rgb = |img: &FloatImage, x: u32, y: u32| -> (f32, f32, f32) {
            let px = img.get_pixel(x, y);
            let ch = img.channels() as usize;
            match ch {
                1 => (px[0], px[0], px[0]),
                2 => (px[0], px[0], px[0]),
                3 => (px[0], px[1], px[2]),
                _ => (px[0], px[1], px[2]),
            }
        };

        let (bw, bh) = img_b.dimensions();

        for y in 0..h {
            for x in 0..w {
                let (ar, ag, ab) = get_rgb(&img_a, x, y);

                // If pixel is outside image B, treat as black (maximise difference).
                let diff = if x < bw && y < bh {
                    let (br, bg, bb) = get_rgb(&img_b, x, y);
                    // Mean absolute difference across RGB.
                    ((ar - br).abs() + (ag - bg).abs() + (ab - bb).abs()) / 3.0
                } else {
                    // Out-of-bounds pixels treated as black.
                    (ar.abs() + ag.abs() + ab.abs()) / 3.0
                };

                // Apply gain and clamp to [0, 1].
                let value = (diff * gain).clamp(0.0, 1.0);
                output.put_pixel(x, y, &[value]);
            }
        }

        Ok(OperationResponse {
            ai_cost_usd: None,
            time: Instant::now().duration_since(start_time),
            responses: vec![OutputResponse { value: Value::Image { data: Arc::new(output), change_id: get_id() } }],
        })
    }
}

#[cfg(test)]
#[path = "compare_tests.rs"]
mod tests;
