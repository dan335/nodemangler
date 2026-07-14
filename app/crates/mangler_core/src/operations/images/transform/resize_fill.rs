//! Resize-to-fill operation that crops to fill exact dimensions.
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

/// Resizes an image to fill the specified dimensions, cropping excess content.
///
/// The image is scaled to cover the entire target area while preserving aspect ratio,
/// then center-cropped to the exact requested size. This guarantees the output
/// matches the requested width and height without distortion.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpImageTransformResizeFill {}

impl OpImageTransformResizeFill {
    /// Returns the node metadata (name and description) for this operation.
    pub fn settings() -> NodeSettings {
        NodeSettings {
            name: "resize fill".to_string(),
            description: "Resizes an image to fill a specific size.".to_string(),
            help: "Scales the image so it completely covers the target rectangle while preserving aspect ratio, then center-crops the overflow to produce output at exactly the requested width and height.\n\nThis is the standard \"cover\" behaviour used for thumbnails and cards: no distortion, no letterboxing, but content on the longer axis will be clipped. Use `resize` if you want to letterbox instead, or `resize exact` to accept distortion. The filter type controls the resampling kernel; the image is round-tripped through DynamicImage internally.".to_string(),
        }
    }

    /// Creates the default inputs: source image, target width/height, and resampling filter type.
    pub fn create_inputs() -> Vec<Input> {
        vec![
            Input::new("image".to_string(),  Value::Image { data:default_image(), change_id:get_id() }, None, None)
                .with_description("Source image to resize."),
            Input::new("width".to_string(), Value::Integer(1), Some(InputSettings::DragValue {clamp:Some((1.0,10000.0)), speed: None }), None)
                .with_description("Target output width in pixels; image is scaled and center-cropped to fill."),
            Input::new("height".to_string(), Value::Integer(1), Some(InputSettings::DragValue {clamp:Some((1.0,10000.0)), speed: None }), None)
                .with_description("Target output height in pixels; image is scaled and center-cropped to fill."),
            Input::new("filter type".to_string(), Value::FilterType(image::imageops::FilterType::Gaussian), None, None)
                .with_description("Resampling filter used for the scale."),
        ]
    }

    /// Creates the default outputs: filled image, and its width and height.
    pub fn create_outputs() -> Vec<Output> {
        vec![
            Output::new("output".to_string(), Value::Image { data:default_image(), change_id:get_id()}, None)
                .with_description("Scaled-and-cropped image filling the requested dimensions exactly."),
            Output::new("width".to_string(), Value::Integer(1), None)
                .with_description("Output width in pixels."),
            Output::new("height".to_string(), Value::Integer(1), None)
                .with_description("Output height in pixels."),
        ]
    }

    /// Executes the resize-to-fill operation, scaling and center-cropping to the target size.
    ///
    /// Converts to DynamicImage for the image crate's resize_to_fill, then converts back.
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

        // Resample in premultiplied-alpha space: the image crate interpolates
        // straight RGBA, which lets fully transparent pixels bleed their hidden
        // colour into semi-transparent edges (white fringe around dark glyphs
        // on a transparent background).
        let dyn_img = data.premultiply_alpha().to_dynamic();
        let resized = dyn_img.resize_to_fill(width as u32, height as u32, filter_type);
        // Convert back to FloatImage and back to straight alpha
        let mut output = FloatImage::from_dynamic(&resized);
        output.unpremultiply_alpha();

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
#[path = "resize_fill_tests.rs"]
mod tests;
