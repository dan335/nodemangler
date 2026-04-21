//! Crop operation for extracting a rectangular sub-region from an image.

use crate::get_id;
use crate::value::ValueType;
use crate::input::{Input, InputSettings};
use crate::node_settings::NodeSettings;
use crate::operations::{OperationResponse, OperationError, OutputResponse, default_image, convert_input};
use crate::output::Output;
use crate::value::Value;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use std::time::Instant;

/// Crops an image to a rectangular sub-region defined by position (x, y) and size (width, height).
///
/// Inputs are clamped to valid ranges based on the source image dimensions.
/// Outputs the cropped image along with its actual width and height.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpImageTransformCrop {}

impl OpImageTransformCrop {
    /// Returns the node metadata (name and description) for this operation.
    pub fn settings() -> NodeSettings {
        NodeSettings {
            name: "crop".to_string(),
            description: "Extracts a rectangular region using position and size.".to_string(),
        }
    }

    /// Creates the default inputs: source image, x/y position, and width/height of the crop region.
    pub fn create_inputs() -> Vec<Input> {
        vec![
            Input::new("image".to_string(),  Value::Image { data:default_image(), change_id:get_id() }, None, None),
            Input::new("x".to_string(), Value::Integer(0), Some(InputSettings::DragValue {clamp:None, speed: None }), None),
            Input::new("y".to_string(), Value::Integer(0), Some(InputSettings::DragValue {clamp:None, speed: None }), None),
            Input::new("width".to_string(), Value::Integer(512), Some(InputSettings::DragValue {clamp:None, speed: None }), None),
            Input::new("height".to_string(), Value::Integer(512), Some(InputSettings::DragValue {clamp:None, speed: None }), None),
        ]
    }

    /// Creates the default outputs: cropped image, and its width and height as integers.
    pub fn create_outputs() -> Vec<Output> {
        vec![
            Output::new("output".to_string(), Value::Image { data:default_image(), change_id:get_id()}, None),
            Output::new("width".to_string(), Value::Integer(1), None),
            Output::new("height".to_string(), Value::Integer(1), None),
        ]
    }

    /// Executes the crop operation.
    ///
    /// Clamps x, y, width, and height to the source image bounds before cropping.
    pub async fn run(inputs: &mut [Input]) -> Result<OperationResponse, OperationError> {
        let start_time = Instant::now();
        let mut input_errors: Vec<(usize, String)> = vec![];

        // convert inputs
        let image_converted = convert_input(inputs, 0, ValueType::Image, &mut input_errors);
        let x_converted = convert_input(inputs, 1, ValueType::Integer, &mut input_errors);
        let y_converted = convert_input(inputs, 2, ValueType::Integer, &mut input_errors);
        let width_converted = convert_input(inputs, 3, ValueType::Integer, &mut input_errors);
        let height_converted = convert_input(inputs, 4, ValueType::Integer, &mut input_errors);


        // return if error
        if !input_errors.is_empty() { return Err(OperationError { input_errors, node_error: None }); }

        // get values
        let Value::Image{data, change_id:_} = image_converted.unwrap() else { unreachable!() };
        let Value::Integer(mut x) = x_converted.unwrap() else { unreachable!() };
        let Value::Integer(mut y) = y_converted.unwrap() else { unreachable!() };
        let Value::Integer(mut width) = width_converted.unwrap() else { unreachable!() };
        let Value::Integer(mut height) = height_converted.unwrap() else { unreachable!() };

        // run node
        // Clamp crop parameters to valid image bounds
        x = x.max(0).min(data.width() as i32 - 1);
        y = y.max(0).min(data.height() as i32 - 1);
        width = width.max(1).min(data.width() as i32);
        height = height.max(1).min(data.height() as i32);

        let cx = x as u32;
        let cy = y as u32;
        let cw = width as u32;
        let ch = height as u32;

        // Copy the crop region into a new FloatImage, preserving channel count
        let mut output = crate::float_image::FloatImage::new(cw, ch, data.channels());
        for oy in 0..ch {
            for ox in 0..cw {
                let sx = (cx + ox).min(data.width() - 1);
                let sy = (cy + oy).min(data.height() - 1);
                output.put_pixel(ox, oy, data.get_pixel(sx, sy));
            }
        }

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
#[path = "crop_tests.rs"]
mod tests;
