//! Exact resize operation that ignores aspect ratio.
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

/// Resizes an image to exactly the specified width and height, ignoring aspect ratio.
///
/// Unlike [`OpImageTransformResize`], this always produces output with the exact
/// requested dimensions, which may distort the image if the aspect ratio differs.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpImageTransformResizeExact {}

impl OpImageTransformResizeExact {
    /// Returns the node metadata (name and description) for this operation.
    pub fn settings() -> NodeSettings {
        NodeSettings {
            name: "resize exact".to_string(),
            description: "Resizes an image to the exact width and height.".to_string(),
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

    /// Creates the default outputs: resized image, and its width and height.
    pub fn create_outputs() -> Vec<Output> {
        vec![
            Output::new("output".to_string(), Value::Image { data:default_image(), change_id:get_id()}, None),
            Output::new("width".to_string(), Value::Integer(1), None),
            Output::new("height".to_string(), Value::Integer(1), None),
        ]
    }

    /// Executes the exact resize operation, stretching or squashing to the target dimensions.
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

        // Convert to DynamicImage for the image crate's exact resize algorithm
        let dyn_img = data.to_dynamic();
        let resized = dyn_img.resize_exact(width as u32, height as u32, filter_type);
        // Convert back to FloatImage
        let output = FloatImage::from_dynamic(&resized);

        let value_width = Value::Integer(output.width() as i32);
        let value_height = Value::Integer(output.height() as i32);

        Ok(OperationResponse { ai_cost_usd: None,
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
#[path = "resize_exact_tests.rs"]
mod tests;
