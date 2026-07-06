//! Morphological closing: dilation followed by erosion.
//!
//! Fills small dark gaps and thin dark cracks while preserving the overall
//! size of bright regions. Pairs with `open` for mask cleanup.

use crate::get_id;
use crate::input::{Input, InputSettings};
use crate::node_settings::NodeSettings;
use crate::operations::images::filter::morphology::erode::separable_morphology;
use crate::operations::{OperationResponse, OperationError, OutputResponse, default_image, convert_input, scale_to_resolution};
use crate::output::Output;
use crate::value::{Value, ValueType};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use std::time::Instant;

/// Morphological closing (dilate then erode).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpImageAdjustmentClose {}

impl OpImageAdjustmentClose {
    pub fn settings() -> NodeSettings {
        NodeSettings {
            name: "close".to_string(),
            description: "Morphological closing — dilate then erode. Fills small dark gaps.".to_string(),
            help: "Runs a dilation (per-channel max in a square window) immediately followed by an erosion (per-channel min) using the same radius. Fills pinholes, seals narrow dark cracks, and joins nearby bright blobs without growing the overall footprint of bright regions.\n\nPairs naturally with `open` for mask cleanup. Uses the separable morphology primitive from `erode.rs`, so cost is O(r) per pixel rather than O(r^2).".to_string(),
        }
    }

    pub fn create_inputs() -> Vec<Input> {
        vec![
            Input::new("image".to_string(), Value::Image { data: default_image(), change_id: get_id() }, None, None)
                .with_description("Source image or mask to close."),
            Input::new("radius".to_string(), Value::Integer(1), Some(InputSettings::Slider { range: (1.0, 16.0), step_by: Some(1.0), clamp_to_range: true }), None)
                .with_description("Structuring element radius, in pixels at a 1024px reference (scales with image size, so the effect is the same at any resolution); larger values fill bigger dark gaps."),
        ]
    }

    pub fn create_outputs() -> Vec<Output> {
        vec![
            Output::new("output".to_string(), Value::Image { data: default_image(), change_id: get_id() }, None)
                .with_description("Image after morphological closing that fills dark gaps and cracks."),
        ]
    }

    pub async fn run(inputs: &mut [Input]) -> Result<OperationResponse, OperationError> {
        let start_time = Instant::now();
        let mut input_errors: Vec<(usize, String)> = vec![];

        let image_converted = convert_input(inputs, 0, ValueType::Image, &mut input_errors);
        let radius_converted = convert_input(inputs, 1, ValueType::Integer, &mut input_errors);

        if !input_errors.is_empty() { return Err(OperationError { input_errors, node_error: None }); }

        let Value::Image { data, change_id: _ } = image_converted.unwrap() else { unreachable!() };
        let Value::Integer(radius) = radius_converted.unwrap() else { unreachable!() };

        // Radius is authored in reference pixels (at 1024px) and scaled to the
        // actual image so closing is the same relative size at any resolution.
        let (w, h) = data.dimensions();
        let radius = scale_to_resolution(radius.max(1) as f32, w, h).round().max(1.0) as i32;
        let dilated = separable_morphology(&data, radius, |a, b| a.max(b));
        let closed = separable_morphology(&dilated, radius, |a, b| a.min(b));

        Ok(OperationResponse {
            time: Instant::now().duration_since(start_time),
            responses: vec![
                OutputResponse { value: Value::Image { data: Arc::new(closed), change_id: get_id() } },
            ],
        })
    }
}

#[cfg(test)]
#[path = "close_tests.rs"]
mod tests;
