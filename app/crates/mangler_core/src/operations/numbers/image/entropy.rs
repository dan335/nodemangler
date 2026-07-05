//! Shannon entropy of the luminance histogram.
//!
//! Measures how much information (in bits) the image's tonal distribution
//! carries — low for flat or few-tone images, high for busy ones.

use crate::get_id;
use crate::input::Input;
use crate::node_settings::NodeSettings;
use crate::operations::{OperationResponse, OperationError, OutputResponse, default_image, convert_input};
use crate::output::Output;
use crate::value::{Value, ValueType};
use serde::{Deserialize, Serialize};
use std::time::Instant;

/// Operation that computes the Shannon entropy of an image's luminance histogram.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpNumberImageEntropy {}

impl OpNumberImageEntropy {
    /// Returns the node metadata (name, description, help).
    pub fn settings() -> NodeSettings {
        NodeSettings {
            name: "entropy".to_string(),
            description: "Reports the Shannon entropy of the luminance histogram, in bits.".to_string(),
            help: "Bins every pixel's Rec. 601 luminance (0.299 R + 0.587 G + 0.114 B) into a 256-bucket histogram, turns the counts into probabilities p, and computes the Shannon entropy −Σ p·log2(p). The result is in bits and ranges from 0 (a single tone) up to 8 (all 256 levels used equally).\n\nUse it as a texture / information-content measure: a flat or two-tone image reads low, a richly varied photo reads high. An empty image reports 0.".to_string(),
        }
    }

    /// Creates the input port: a single image to measure.
    pub fn create_inputs() -> Vec<Input> {
        vec![
            Input::new("image".to_string(), Value::Image { data: default_image(), change_id: get_id() }, None, None)
                .with_description("Image whose luminance histogram is measured."),
        ]
    }

    /// Creates the output port: the entropy in bits.
    pub fn create_outputs() -> Vec<Output> {
        vec![
            Output::new("entropy".to_string(), Value::Decimal(0.0), None)
                .with_description("Shannon entropy in bits (0–8)."),
        ]
    }

    /// Executes the entropy reduction.
    pub async fn run(inputs: &mut [Input]) -> Result<OperationResponse, OperationError> {
        let start_time = Instant::now();
        let mut input_errors: Vec<(usize, String)> = vec![];

        let image_converted = convert_input(inputs, 0, ValueType::Image, &mut input_errors);

        if !input_errors.is_empty() { return Err(OperationError { input_errors, node_error: None }); }

        let Value::Image { data, change_id: _ } = image_converted.unwrap() else { unreachable!() };

        let v = super::luma_values(&data);
        let entropy = if v.is_empty() {
            0.0f32
        } else {
            let mut hist = [0u64; 256];
            for &x in &v {
                let idx = (x.clamp(0.0, 1.0) * 255.0).round() as usize;
                hist[idx.min(255)] += 1;
            }
            let n = v.len() as f64;
            let mut e = 0.0f64;
            for &count in hist.iter() {
                if count > 0 {
                    let p = count as f64 / n;
                    e -= p * p.log2();
                }
            }
            e as f32
        };

        Ok(OperationResponse {
            time: Instant::now().duration_since(start_time),
            responses: vec![
                OutputResponse { value: Value::Decimal(entropy) },
            ],
        })
    }
}

#[cfg(test)]
#[path = "entropy_tests.rs"]
mod tests;
