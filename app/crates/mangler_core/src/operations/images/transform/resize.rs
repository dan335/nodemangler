//! Aspect-ratio-preserving resize operation.
//!
//! Converts to [`DynamicImage`] for the `image` crate's resize algorithms,
//! then converts the result back to [`FloatImage`].

use crate::get_id;
use crate::float_image::FloatImage;
use crate::value::ValueType;
use crate::input::{Input, InputSettings};
use crate::node_settings::NodeSettings;
use crate::operations::{OperationResponse, OperationError, OutputResponse, default_image, convert_input};
use crate::output::Output;
use crate::value::Value;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use std::time::Instant;

/// Resizes an image to fit within the specified width and height while preserving aspect ratio.
///
/// The output dimensions may be smaller than the requested size because the image
/// is scaled uniformly to fit within the bounding box. Use `resize_exact` or
/// `resize_fill` for operations that always produce the exact requested dimensions.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpImageTransformResize {}

impl OpImageTransformResize {
    /// Returns the node metadata (name and description) for this operation.
    pub fn settings() -> NodeSettings {
        NodeSettings {
            name: "resize".to_string(),
            description: "Resizes an image to fit within the target dimensions while preserving aspect ratio. Output may be smaller than requested.".to_string(),
        }
    }

    /// Creates the default inputs: source image, target width/height, and resampling filter type.
    pub fn create_inputs() -> Vec<Input> {
        vec![
            Input::new("image".to_string(),  Value::Image { data:default_image(), change_id:get_id() }, None, None),
            Input::new("width".to_string(), Value::Integer(1), Some(InputSettings::DragValue {clamp:Some((1.0,10000.0)), speed: None }), None),
            Input::new("height".to_string(), Value::Integer(1), Some(InputSettings::DragValue {clamp:Some((1.0,10000.0)), speed: None }), None),
            Input::new("filter type".to_string(), Value::FilterType(image::imageops::FilterType::Gaussian), None, None),
        ]
    }

    /// Creates the default outputs: resized image, and its actual width and height.
    pub fn create_outputs() -> Vec<Output> {
        vec![
            Output::new("output".to_string(), Value::Image { data:default_image(), change_id:get_id()}, None),
            Output::new("width".to_string(), Value::Integer(1), None),
            Output::new("height".to_string(), Value::Integer(1), None),
        ]
    }

    /// Executes the resize operation, fitting the image within the target dimensions.
    ///
    /// Converts to DynamicImage for the image crate's resize, then converts back.
    pub async fn run(inputs: &mut [Input]) -> Result<OperationResponse, OperationError> {
        let start_time = Instant::now();
        let mut input_errors: Vec<(usize, String)> = vec![];

        // convert inputs
        let image_converted = convert_input(inputs, 0, ValueType::Image, &mut input_errors);
        let width_converted = convert_input(inputs, 1, ValueType::Integer, &mut input_errors);
        let height_converted = convert_input(inputs, 2, ValueType::Integer, &mut input_errors);
        let filter_type_converted = convert_input(inputs, 3, ValueType::FilterType, &mut input_errors);

        // return if error
        if !input_errors.is_empty() { return Err(OperationError { input_errors, node_error: None }); }

        // get values
        let Value::Image{data, change_id:_} = image_converted.unwrap() else { unreachable!() };
        let Value::Integer(mut width) = width_converted.unwrap() else { unreachable!() };
        let Value::Integer(mut height) = height_converted.unwrap() else { unreachable!() };
        let Value::FilterType(filter_type) = filter_type_converted.unwrap() else { unreachable!() };

        // Ensure minimum dimensions of 1x1
        width = width.max(1);
        height = height.max(1);

        // Convert to DynamicImage for the image crate's resize algorithm
        let dyn_img = data.to_dynamic();
        // resize() preserves aspect ratio, so output may be smaller than requested
        let resized = dyn_img.resize(width as u32, height as u32, filter_type);
        // Convert back to FloatImage
        let output = FloatImage::from_dynamic(&resized);

        let value_width = Value::Integer(output.width() as i32);
        let value_height = Value::Integer(output.height() as i32);

        Ok(OperationResponse { 
            time: Instant::now().duration_since(start_time),
            responses: vec![
                OutputResponse {value: Value::Image { data:Arc::new(output), change_id:get_id() }},
                OutputResponse {value: value_width},
                OutputResponse {value: value_height},
            ],
        })
    }
}

#[cfg(test)]
#[path = "resize_tests.rs"]
mod tests;
