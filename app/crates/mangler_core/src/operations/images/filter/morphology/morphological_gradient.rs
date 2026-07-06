//! Morphological gradient: dilation minus erosion.
//!
//! Outlines edges by subtracting the per-channel local minimum from the local
//! maximum over a square window. Flat regions yield zero; the response is
//! widest where neighbouring pixels differ most. Reuses the separable
//! morphology primitive from `erode.rs`.

use crate::get_id;
use crate::input::{Input, InputSettings};
use crate::node_settings::NodeSettings;
use crate::operations::{OperationResponse, OperationError, OutputResponse, default_image, convert_input, scale_to_resolution};
use crate::operations::images::filter::morphology::erode::separable_morphology;
use crate::output::Output;
use crate::value::{Value, ValueType};
use crate::float_image::FloatImage;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use std::time::Instant;

/// Morphological gradient (dilation − erosion).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpImageAdjustmentMorphGradient {}

impl OpImageAdjustmentMorphGradient {
    /// Returns the node metadata (name and description) for morphological gradient.
    pub fn settings() -> NodeSettings {
        NodeSettings {
            name: "morphological gradient".to_string(),
            description: "Edge band from dilation minus erosion over a square window.".to_string(),
            help: "Computes the per-channel maximum (dilation) and minimum (erosion) over a (2r+1) square window and outputs their difference. Inside flat regions the max and min agree and the result is 0; near edges the difference grows, producing a band that thickens with `radius`.\n\nUseful as an edge/outline mask or to feed thresholding and compositing. Implemented with two separable min/max passes per operator, so cost is O(r) per pixel. Alpha is processed alongside colour; output dimensions and channel count match the input.".to_string(),
        }
    }

    /// Creates input ports: image and window radius.
    pub fn create_inputs() -> Vec<Input> {
        vec![
            Input::new("image".to_string(), Value::Image { data: default_image(), change_id: get_id() }, None, None)
                .with_description("Source image or mask to outline."),
            Input::new("radius".to_string(), Value::Integer(1), Some(InputSettings::Slider { range: (1.0, 16.0), step_by: Some(1.0), clamp_to_range: true }), None)
                .with_description("Half-size of the square window, in pixels at a 1024px reference (scales with image size, so the effect is the same at any resolution); larger values widen the edge band."),
        ]
    }

    /// Creates the output port: the gradient (edge band) image.
    pub fn create_outputs() -> Vec<Output> {
        vec![
            Output::new("output".to_string(), Value::Image { data: default_image(), change_id: get_id() }, None)
                .with_description("Edge band where local max and min differ; flat areas are black."),
        ]
    }

    /// Runs the morphological gradient.
    pub async fn run(inputs: &mut [Input]) -> Result<OperationResponse, OperationError> {
        let start_time = Instant::now();
        let mut input_errors: Vec<(usize, String)> = vec![];

        let image_converted = convert_input(inputs, 0, ValueType::Image, &mut input_errors);
        let radius_converted = convert_input(inputs, 1, ValueType::Integer, &mut input_errors);

        if !input_errors.is_empty() { return Err(OperationError { input_errors, node_error: None }); }

        let Value::Image { data, change_id: _ } = image_converted.unwrap() else { unreachable!() };
        let Value::Integer(radius) = radius_converted.unwrap() else { unreachable!() };

        // Radius is authored in reference pixels (at 1024px) and scaled to the
        // actual image so the edge band is the same relative size at any resolution.
        let (w, h) = data.dimensions();
        let radius = scale_to_resolution(radius.max(1) as f32, w, h).round().max(1.0) as i32;
        let dilated = separable_morphology(&data, radius, |a, b| a.max(b));
        let eroded = separable_morphology(&data, radius, |a, b| a.min(b));
        let diff: Vec<f32> = dilated.as_raw().iter().zip(eroded.as_raw().iter()).map(|(a, b)| a - b).collect();
        let out = FloatImage::from_raw(w, h, data.channels(), diff).unwrap();

        Ok(OperationResponse {
            time: Instant::now().duration_since(start_time),
            responses: vec![OutputResponse { value: Value::Image { data: Arc::new(out), change_id: get_id() } }],
        })
    }
}

#[cfg(test)]
#[path = "morphological_gradient_tests.rs"]
mod tests;
