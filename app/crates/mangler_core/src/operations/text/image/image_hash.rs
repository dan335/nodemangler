//! Average-hash (aHash) fingerprint of an image.
//!
//! Shrinks the image to 8×8, thresholds each cell against the mean luminance,
//! and packs the 64 bits into a 16-character hex string. Similar images produce
//! similar hashes (compare with Hamming distance).

use crate::get_id;
use crate::input::Input;
use crate::node_settings::NodeSettings;
use crate::operations::{OperationResponse, OperationError, OutputResponse, default_image, convert_input};
use crate::output::Output;
use crate::value::{Value, ValueType};
use serde::{Deserialize, Serialize};
use std::time::Instant;

use crate::operations::numbers::image::pixel_luma;

/// Operation that computes a 64-bit average hash of an image.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpTextImageHash {}

impl OpTextImageHash {
    /// Returns the node metadata (name, description, help).
    pub fn settings() -> NodeSettings {
        NodeSettings {
            name: "image hash".to_string(),
            description: "Computes an 8×8 average-hash fingerprint as a hex string.".to_string(),
            help: "Shrinks the image to 8×8, computes the mean luminance, and sets one bit per cell for cells at or above the mean — an average hash (aHash) — packed into a 16-character hex string.\n\nSimilar images produce similar hashes, so you can gauge how alike two images are by the Hamming distance (number of differing bits) between their hashes. This is a perceptual fingerprint, not a cryptographic hash.".to_string(),
        }
    }

    /// Creates the input port: a single image.
    pub fn create_inputs() -> Vec<Input> {
        vec![
            Input::new("image".to_string(), Value::Image { data: default_image(), change_id: get_id() }, None, None)
                .with_description("Image to fingerprint."),
        ]
    }

    /// Creates the output port: the hex hash string.
    pub fn create_outputs() -> Vec<Output> {
        vec![
            Output::new("output".to_string(), Value::Text(String::new()), None)
                .with_description("16-character hex of the 64-bit average hash."),
        ]
    }

    /// Executes the average-hash computation.
    pub async fn run(inputs: &mut [Input]) -> Result<OperationResponse, OperationError> {
        let start_time = Instant::now();
        let mut input_errors: Vec<(usize, String)> = vec![];

        let image_converted = convert_input(inputs, 0, ValueType::Image, &mut input_errors);

        if !input_errors.is_empty() { return Err(OperationError { input_errors, node_error: None }); }

        let Value::Image { data, change_id: _ } = image_converted.unwrap() else { unreachable!() };

        let small = data.resize(8, 8);
        let lumas: Vec<f32> = small.pixels().map(pixel_luma).collect();
        let mean = lumas.iter().sum::<f32>() / (lumas.len().max(1) as f32);
        let mut bits: u64 = 0;
        for (i, &l) in lumas.iter().enumerate() {
            if l >= mean {
                bits |= 1u64 << i;
            }
        }
        let hex = format!("{:016x}", bits);

        Ok(OperationResponse {
            time: Instant::now().duration_since(start_time),
            responses: vec![OutputResponse { value: Value::Text(hex) }],
        })
    }
}

#[cfg(test)]
#[path = "image_hash_tests.rs"]
mod tests;
