//! Perceptual-hash (dHash) comparison between two images.
//!
//! Computes a 64-bit difference hash (dHash) for each image and reports the
//! Hamming distance between the two hashes plus a normalized similarity. dHash
//! is robust to scaling, mild blur, and small tonal shifts, making it a cheap
//! near-duplicate detector.

use crate::get_id;
use crate::input::Input;
use crate::node_settings::NodeSettings;
use crate::operations::{OperationResponse, OperationError, OutputResponse, default_image, convert_input};
use crate::output::Output;
use crate::value::{Value, ValueType};
use serde::{Deserialize, Serialize};
use std::time::Instant;

use super::pixel_luma;

/// Operation that compares two images via their dHash perceptual hashes.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpNumberImagePerceptualHash {}

/// Computes the 64-bit difference hash (dHash) of an image.
///
/// The image is resized to 9x8 luminance samples; each of the 8 rows yields 8
/// bits by comparing horizontally adjacent pixels (bit set when the left pixel
/// is darker than its right neighbor).
fn dhash(img: &crate::float_image::FloatImage) -> u64 {
    let r = img.resize(9, 8);
    let mut bits = 0u64;
    let mut idx = 0u32;
    for y in 0..8u32 {
        for x in 0..8u32 {
            let left = pixel_luma(r.get_pixel(x, y));
            let right = pixel_luma(r.get_pixel(x + 1, y));
            if left < right { bits |= 1u64 << idx; }
            idx += 1;
        }
    }
    bits
}

impl OpNumberImagePerceptualHash {
    /// Returns the node metadata (name, description, help).
    pub fn settings() -> NodeSettings {
        NodeSettings {
            name: "perceptual hash".to_string(),
            description: "Compares two images via a difference hash (dHash) Hamming distance.".to_string(),
            help: "Computes a 64-bit difference hash (dHash) for each image: the image is reduced to a 9x8 grayscale grid and each of the 64 bits records whether a pixel is darker than its right-hand neighbor. The two hashes are XORed and the set bits counted to give the Hamming distance (0 = identical hash, 64 = fully opposite), and similarity is 1 - distance / 64.\n\ndHash ignores absolute size and is fairly tolerant of scaling, mild blur, and brightness shifts, so it is a cheap near-duplicate / similarity check. Distances under ~10 usually indicate the same or a lightly edited image; large distances mean the images are structurally different.".to_string(),
        }
    }

    /// Creates the input ports: the two images to hash and compare.
    pub fn create_inputs() -> Vec<Input> {
        vec![
            Input::new("image a".to_string(), Value::Image { data: default_image(), change_id: get_id() }, None, None)
                .with_description("First image to hash."),
            Input::new("image b".to_string(), Value::Image { data: default_image(), change_id: get_id() }, None, None)
                .with_description("Second image to hash and compare against the first."),
        ]
    }

    /// Creates the output ports: Hamming distance and normalized similarity.
    pub fn create_outputs() -> Vec<Output> {
        vec![
            Output::new("distance".to_string(), Value::Integer(0), None)
                .with_description("Hamming distance between the two dHashes (0..64)."),
            Output::new("similarity".to_string(), Value::Decimal(1.0), None)
                .with_description("Normalized similarity, 1 - distance / 64 (1 = identical hash)."),
        ]
    }

    /// Executes the perceptual-hash comparison.
    pub async fn run(inputs: &mut [Input]) -> Result<OperationResponse, OperationError> {
        let start_time = Instant::now();
        let mut input_errors: Vec<(usize, String)> = vec![];

        let a_converted = convert_input(inputs, 0, ValueType::Image, &mut input_errors);
        let b_converted = convert_input(inputs, 1, ValueType::Image, &mut input_errors);

        if !input_errors.is_empty() { return Err(OperationError { input_errors, node_error: None }); }

        let Value::Image { data: a, change_id: _ } = a_converted.unwrap() else { unreachable!() };
        let Value::Image { data: b, change_id: _ } = b_converted.unwrap() else { unreachable!() };

        let ha = dhash(&a);
        let hb = dhash(&b);
        let distance = (ha ^ hb).count_ones() as i32;
        let similarity = 1.0 - distance as f32 / 64.0;

        Ok(OperationResponse {
            time: Instant::now().duration_since(start_time),
            responses: vec![
                OutputResponse { value: Value::Integer(distance) },
                OutputResponse { value: Value::Decimal(similarity) },
            ],
        })
    }
}

#[cfg(test)]
#[path = "perceptual_hash_tests.rs"]
mod tests;
