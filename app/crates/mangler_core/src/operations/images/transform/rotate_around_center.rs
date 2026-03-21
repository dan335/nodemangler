//! Arbitrary-angle rotation around the image center.
//!
//! Delegates to `imageproc` for bicubic interpolation, converting to/from
//! [`FloatImage`] at the boundary.

use crate::color::Color;
use crate::get_id;
use crate::value::ValueType;
use crate::float_image::FloatImage;
use crate::input::{Input, InputSettings};
use crate::node_settings::NodeSettings;
use crate::operations::{OperationResponse, OperationError, OutputResponse, default_image, convert_input};
use crate::output::Output;
use crate::value::Value;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use std::time::Instant;

/// Rotates an image by an arbitrary angle (in degrees) around its center point.
///
/// Uses bicubic interpolation for smooth results. Areas outside the original
/// image bounds are filled with the specified background color.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpImageTransformRotateAroundCenter {}

impl OpImageTransformRotateAroundCenter {
    /// Returns the node metadata (name and description) for this operation.
    pub fn settings() -> NodeSettings {
        NodeSettings {
            name: "rotate".to_string(),
            description: "Rotates an image by any angle around its center point.".to_string(),
        }
    }

    /// Creates the default inputs: source image, rotation angle in degrees, and background fill color.
    pub fn create_inputs() -> Vec<Input> {
        vec![
            Input::new("image".to_string(),  Value::Image { data:default_image(), change_id:get_id() }, None, None),
            Input::new("degrees".to_string(), Value::Decimal(45.0), Some(InputSettings::Slider { range: (0.0, 360.0), step_by: Some(0.01), clamp_to_range:false }), None),
            Input::new("background color".to_string(), Value::Color(Color::from_srgb_u8(0,0,0,0)), None, None),
        ]
    }

    /// Creates the default outputs: the rotated image.
    pub fn create_outputs() -> Vec<Output> {
        vec![
            Output::new("output".to_string(), Value::Image { data:default_image(), change_id:get_id()}, None),
        ]
    }

    /// Executes the rotation using `imageproc` bicubic interpolation.
    ///
    /// Converts the FloatImage to an RGBA8 buffer for imageproc, then converts the
    /// result back to a FloatImage via DynamicImage.
    pub async fn run(inputs: &mut [Input]) -> Result<OperationResponse, OperationError> {
        let start_time = Instant::now();
        let mut input_errors: Vec<(usize, String)> = vec![];

        // convert inputs
        let image_converted = convert_input(inputs, 0, ValueType::Image, &mut input_errors);
        let degrees_converted = convert_input(inputs, 1, ValueType::Decimal, &mut input_errors);
        let bg_color_converted = convert_input(inputs, 2, ValueType::Color, &mut input_errors);

        // return if error
        if !input_errors.is_empty() { return Err(OperationError { input_errors, node_error: None }); }

        // get values
        let Value::Image{data, change_id:_} = image_converted.unwrap() else { unreachable!() };
        let Value::Decimal(degrees) = degrees_converted.unwrap() else { unreachable!() };
        let Value::Color(bg_color) = bg_color_converted.unwrap() else { unreachable!() };

        // Convert FloatImage to RGBA8 for the imageproc API
        let rgba8 = data.to_rgba8();

        // Convert the background color to sRGB u8 for the imageproc API
        let color = bg_color.to_srgb_u8();

        // Perform the rotation using imageproc's bicubic interpolation
        let adjusted = imageproc::geometric_transformations::rotate_about_center(
            &rgba8,
            degrees.to_radians(),
            imageproc::geometric_transformations::Interpolation::Bicubic,
            image::Rgba([color.0, color.1, color.2, color.3]),
        );

        // Convert the rotated RGBA8 result back to FloatImage
        let output = FloatImage::from_dynamic(&image::DynamicImage::ImageRgba8(adjusted));

        Ok(OperationResponse {
            time: Instant::now().duration_since(start_time),
            responses: vec![
                OutputResponse {value: Value::Image { data: Arc::new(output), change_id:get_id() }},
            ],
        })
    }
}

#[cfg(test)]
#[path = "rotate_around_center_tests.rs"]
mod tests;
