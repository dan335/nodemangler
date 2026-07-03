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
use rayon::prelude::*;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use std::time::Instant;

/// Minimum pixel count before the comparison is parallelized over rows.
const PARALLEL_PIXELS: usize = 1 << 16;

/// Reads RGB from an interleaved pixel, broadcasting grayscale sources.
#[inline]
fn rgb_of(px: &[f32], ch: usize) -> (f32, f32, f32) {
    if ch >= 3 { (px[0], px[1], px[2]) } else { (px[0], px[0], px[0]) }
}

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
            help: "For each pixel, computes the mean absolute difference across the RGB channels of A and B (alpha is ignored), multiplies by gain, and clamps to 0-1 to produce a single-channel greyscale output. Identical images yield pure black; maximally different yield white.\n\nOutput size matches image A; pixels outside image B's bounds are treated as black, which inflates the difference in those regions. Gain lets small deviations become visible without changing the comparison math. Useful for visual regression and debugging node outputs: wire two graph branches in and eyeball the diff.".to_string(),
        }
    }

    pub fn create_inputs() -> Vec<Input> {
        vec![
            // First image to compare.
            Input::new("image a".to_string(), Value::Image { data: default_image(), change_id: get_id() }, None, None)
                .with_description("First image; its dimensions determine the output size."),
            // Second image to compare.
            Input::new("image b".to_string(), Value::Image { data: default_image(), change_id: get_id() }, None, None)
                .with_description("Second image; pixels outside its bounds are treated as black."),
            // Multiplier that amplifies small differences (1.0 = raw, higher = more visible).
            Input::new("gain".to_string(), Value::Decimal(1.0), Some(InputSettings::Slider { range: (1.0, 10.0), step_by: Some(0.1), clamp_to_range: true }), None)
                .with_description("Multiplier applied to the per-pixel difference to make small deltas visible."),
        ]
    }

    pub fn create_outputs() -> Vec<Output> {
        vec![Output::new("output".to_string(), Value::Image { data: default_image(), change_id: get_id() }, None)
            .with_description("Grayscale difference map: black where images match, white where they disagree.")]
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
        let (bw, bh) = img_b.dimensions();
        let a_ch = img_a.channels() as usize;
        let b_ch = img_b.channels() as usize;
        let a_raw = img_a.as_raw();
        let b_raw = img_b.as_raw();
        let wu = w as usize;
        let bwu = bw as usize;

        // Output is single-channel greyscale.
        let mut out_data = vec![0.0f32; wu * h as usize];

        // Compare one row of A against the matching row of B (if any); the
        // per-image bounds checks and channel dispatch are hoisted per row.
        let process_row = |(y, dst_row): (usize, &mut [f32])| {
            let a_row = &a_raw[y * wu * a_ch..(y + 1) * wu * a_ch];
            // Columns of this row covered by image B (empty when the row is
            // below image B's height).
            let b_row = if (y as u32) < bh { &b_raw[y * bwu * b_ch..(y + 1) * bwu * b_ch] } else { &[][..] };
            let in_w = if b_row.is_empty() { 0 } else { wu.min(bwu) };

            for (x, dst) in dst_row.iter_mut().enumerate() {
                let (ar, ag, ab) = rgb_of(&a_row[x * a_ch..], a_ch);

                // If pixel is outside image B, treat as black (maximise difference).
                let diff = if x < in_w {
                    let (br, bg, bb) = rgb_of(&b_row[x * b_ch..], b_ch);
                    // Mean absolute difference across RGB.
                    ((ar - br).abs() + (ag - bg).abs() + (ab - bb).abs()) / 3.0
                } else {
                    // Out-of-bounds pixels treated as black.
                    (ar.abs() + ag.abs() + ab.abs()) / 3.0
                };

                // Apply gain and clamp to [0, 1].
                *dst = (diff * gain).clamp(0.0, 1.0);
            }
        };

        if wu > 0 {
            if wu * h as usize >= PARALLEL_PIXELS {
                out_data.par_chunks_exact_mut(wu).enumerate().for_each(process_row);
            } else {
                out_data.chunks_exact_mut(wu).enumerate().for_each(process_row);
            }
        }

        let output = FloatImage::from_raw(w, h, 1, out_data).unwrap();

        Ok(OperationResponse {
            
            time: Instant::now().duration_since(start_time),
            responses: vec![OutputResponse { value: Value::Image { data: Arc::new(output), change_id: get_id() } }],
        })
    }
}

#[cfg(test)]
#[path = "compare_tests.rs"]
mod tests;
