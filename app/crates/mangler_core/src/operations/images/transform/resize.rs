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
            description: "Resizes an image to fit within the target dimensions while preserving aspect ratio. 0 leaves a dimension unconstrained.".to_string(),
            help: "Scales the image uniformly so it fits inside the (width, height) bounding box. Because aspect ratio is preserved, one dimension usually ends up smaller than requested; use the width/height outputs to read the actual result size.\n\nA width or height of 0 means \"no limit on that side\": set only width to scale to an exact width keeping aspect, only height for the reverse, and leave both at 0 to pass the image through unchanged.\n\nThe selected filter type (Nearest, Triangle, CatmullRom, Gaussian, Lanczos3) controls sharpness vs. aliasing of the resample. Internally the FloatImage round-trips through DynamicImage, which means output channel count is normalized to the 8-bit image representation used by the image crate (except for the both-zero passthrough, which keeps the source untouched).".to_string(),
        }
    }

    /// Creates the default inputs: source image, target width/height, and resampling filter type.
    pub fn create_inputs() -> Vec<Input> {
        vec![
            Input::new("image".to_string(),  Value::Image { data:default_image(), change_id:get_id() }, None, None)
                .with_description("Source image to resize."),
            Input::new("width".to_string(), Value::Integer(0), Some(InputSettings::DragValue {clamp:Some((0.0,10000.0)), speed: None }), None)
                .with_description("Maximum output width; result is scaled to fit within this. 0 = no width limit."),
            Input::new("height".to_string(), Value::Integer(0), Some(InputSettings::DragValue {clamp:Some((0.0,10000.0)), speed: None }), None)
                .with_description("Maximum output height; result is scaled to fit within this. 0 = no height limit."),
            Input::new("filter type".to_string(), Value::FilterType(image::imageops::FilterType::Gaussian), None, None)
                .with_description("Resampling filter used for the scale."),
        ]
    }

    /// Creates the default outputs: resized image, and its actual width and height.
    pub fn create_outputs() -> Vec<Output> {
        vec![
            Output::new("output".to_string(), Value::Image { data:default_image(), change_id:get_id()}, None)
                .with_description("Aspect-preserving resized image fitting inside the target box."),
            Output::new("width".to_string(), Value::Integer(1), None)
                .with_description("Actual output width in pixels after aspect-preserving fit."),
            Output::new("height".to_string(), Value::Integer(1), None)
                .with_description("Actual output height in pixels after aspect-preserving fit."),
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

        // 0 (and any negative) means "no limit on that side".
        width = width.max(0);
        height = height.max(0);

        // Both sides unconstrained: pass the image through untouched (a
        // resize toward an unbounded box would otherwise blow up to u32::MAX).
        if width == 0 && height == 0 {
            let out_width = data.width() as i32;
            let out_height = data.height() as i32;
            return Ok(OperationResponse {
                time: Instant::now().duration_since(start_time),
                responses: vec![
                    OutputResponse { value: Value::Image { data, change_id: get_id() } },
                    OutputResponse { value: Value::Integer(out_width) },
                    OutputResponse { value: Value::Integer(out_height) },
                ],
            });
        }

        // An unconstrained side becomes a u32::MAX bound: the fit ratio
        // (min of the per-axis ratios, computed in f64 by the image crate)
        // is then decided entirely by the constrained side.
        let bound_width = if width == 0 { u32::MAX } else { width as u32 };
        let bound_height = if height == 0 { u32::MAX } else { height as u32 };

        // Resample in premultiplied-alpha space: the image crate interpolates
        // straight RGBA, which lets fully transparent pixels bleed their hidden
        // colour into semi-transparent edges (white fringe around dark glyphs
        // on a transparent background).
        let dyn_img = data.premultiply_alpha().to_dynamic();
        // resize() preserves aspect ratio, so output may be smaller than requested
        let resized = dyn_img.resize(bound_width, bound_height, filter_type);
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
#[path = "resize_tests.rs"]
mod tests;
