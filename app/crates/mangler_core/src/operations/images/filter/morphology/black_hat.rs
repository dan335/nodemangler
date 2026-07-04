//! Black top-hat transform: morphological closing minus the image.
//!
//! Isolates dark features smaller than the structuring element (and darkens
//! against an uneven background). Closing is a dilation followed by an erosion;
//! subtracting the original leaves the dark detail the closing filled in.

use crate::get_id;
use crate::input::{Input, InputSettings};
use crate::node_settings::NodeSettings;
use crate::operations::{OperationResponse, OperationError, OutputResponse, default_image, convert_input};
use crate::operations::images::filter::morphology::erode::separable_morphology;
use crate::output::Output;
use crate::value::{Value, ValueType};
use crate::float_image::FloatImage;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use std::time::Instant;

/// Black top-hat (closing − image).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpImageAdjustmentBlackHat {}

impl OpImageAdjustmentBlackHat {
    /// Returns the node metadata (name and description) for black-hat.
    pub fn settings() -> NodeSettings {
        NodeSettings {
            name: "black hat".to_string(),
            description: "Black top-hat: closing minus image. Extracts small dark details.".to_string(),
            help: "Computes the morphological closing (dilation then erosion over a (2r+1) square window), which fills dark structures smaller than the window, then subtracts the original. What remains are the dark details and crevices the closing filled — the dark-feature counterpart of the white top-hat, useful for finding thin dark lines, dust, and cracks on a bright background.\n\nLarger radius fills larger dark features (so more is extracted). Implemented with separable min/max passes; cost is O(r) per pixel. Alpha is processed alongside colour; output dimensions and channel count match the input.".to_string(),
        }
    }

    /// Creates input ports: image and structuring-element radius.
    pub fn create_inputs() -> Vec<Input> {
        vec![
            Input::new("image".to_string(), Value::Image { data: default_image(), change_id: get_id() }, None, None)
                .with_description("Source image to extract dark details from."),
            Input::new("radius".to_string(), Value::Integer(3), Some(InputSettings::Slider { range: (1.0, 32.0), step_by: Some(1.0), clamp_to_range: true }), None)
                .with_description("Half-size of the square window; dark features larger than this are ignored."),
        ]
    }

    /// Creates the output port: the black-hat image.
    pub fn create_outputs() -> Vec<Output> {
        vec![
            Output::new("output".to_string(), Value::Image { data: default_image(), change_id: get_id() }, None)
                .with_description("Dark details smaller than the window; flat areas are black."),
        ]
    }

    /// Runs the black top-hat transform.
    pub async fn run(inputs: &mut [Input]) -> Result<OperationResponse, OperationError> {
        let start_time = Instant::now();
        let mut input_errors: Vec<(usize, String)> = vec![];

        let image_converted = convert_input(inputs, 0, ValueType::Image, &mut input_errors);
        let radius_converted = convert_input(inputs, 1, ValueType::Integer, &mut input_errors);

        if !input_errors.is_empty() { return Err(OperationError { input_errors, node_error: None }); }

        let Value::Image { data, change_id: _ } = image_converted.unwrap() else { unreachable!() };
        let Value::Integer(radius) = radius_converted.unwrap() else { unreachable!() };
        let radius = radius.max(1);

        let (w, h) = data.dimensions();
        // Closing = dilate then erode.
        let dilated = separable_morphology(&data, radius, |a, b| a.max(b));
        let closed = separable_morphology(&dilated, radius, |a, b| a.min(b));
        let diff: Vec<f32> = closed.as_raw().iter().zip(data.as_raw().iter()).map(|(a, b)| a - b).collect();
        let out = FloatImage::from_raw(w, h, data.channels(), diff).unwrap();

        Ok(OperationResponse {
            time: Instant::now().duration_since(start_time),
            responses: vec![OutputResponse { value: Value::Image { data: Arc::new(out), change_id: get_id() } }],
        })
    }
}

#[cfg(test)]
#[path = "black_hat_tests.rs"]
mod tests;
