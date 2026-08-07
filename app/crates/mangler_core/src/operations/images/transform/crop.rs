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

/// Crops an image to a rectangular sub-region defined by position (x, y) and size (width, height),
/// all expressed as 0-1 fractions of the source image's dimensions.
///
/// Working in fractions makes the node resolution-independent: the same values keep framing the
/// same part of the picture whether the source is 512px or 6000px wide.
///
/// Inputs are clamped so the region always keeps at least one pixel and never extends past the
/// right or bottom edge. Outputs the cropped image along with its actual pixel width and height.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpImageTransformCrop {}

impl OpImageTransformCrop {
    /// Returns the node metadata (name and description) for this operation.
    pub fn settings() -> NodeSettings {
        NodeSettings {
            name: "crop".to_string(),
            description: "Extracts a rectangular region using fractional position and size.".to_string(),
            help: "Copies a rectangular sub-region of the source image starting at (x, y) with the requested width and height; the result is a new image whose pixel dimensions are emitted on the width/height outputs.\n\nAll four parameters are 0-1 fractions of the source image's size, not pixels: x = 0.25 starts a quarter of the way across, width = 0.5 keeps half the image's width. That makes the crop resolution-independent — the same values frame the same part of the picture at any input size — so swapping a 1024px source for a 6000px one needs no re-tuning.\n\nFractions are converted to pixel edges by rounding, then clamped to the source's valid range: the region always keeps at least one pixel and never extends past the right or bottom edge, so an off-origin crop clips instead of running off the image. No resampling is performed; channel count is preserved exactly.".to_string(),
        }
    }

    /// Creates the default inputs: source image, x/y position, and width/height of the crop
    /// region — the latter four all as 0-1 fractions of the source image's dimensions.
    pub fn create_inputs() -> Vec<Input> {
        vec![
            Input::new("image".to_string(),  Value::Image { data:default_image(), change_id:get_id() }, None, None)
                .with_description("Source image to crop."),
            Input::new("x".to_string(), Value::Decimal(0.0), Some(InputSettings::Slider { range: (0.0, 1.0), step_by: None, clamp_to_range: true }), None)
                .with_description("Left edge of the crop region as a 0-1 fraction of image width (0.25 = a quarter across). Resolution-independent."),
            Input::new("y".to_string(), Value::Decimal(0.0), Some(InputSettings::Slider { range: (0.0, 1.0), step_by: None, clamp_to_range: true }), None)
                .with_description("Top edge of the crop region as a 0-1 fraction of image height. Resolution-independent."),
            Input::new("width".to_string(), Value::Decimal(0.5), Some(InputSettings::Slider { range: (0.0, 1.0), step_by: None, clamp_to_range: true }), None)
                .with_description("Width of the crop region as a 0-1 fraction of image width (0.5 = half the image); clipped at the right edge."),
            Input::new("height".to_string(), Value::Decimal(0.5), Some(InputSettings::Slider { range: (0.0, 1.0), step_by: None, clamp_to_range: true }), None)
                .with_description("Height of the crop region as a 0-1 fraction of image height; clipped at the bottom edge."),
        ]
    }

    /// Creates the default outputs: cropped image, and its width and height as integers.
    pub fn create_outputs() -> Vec<Output> {
        vec![
            Output::new("output".to_string(), Value::Image { data:default_image(), change_id:get_id()}, None)
                .with_description("Cropped sub-region of the source image."),
            Output::new("width".to_string(), Value::Integer(1), None)
                .with_description("Actual cropped image width in pixels."),
            Output::new("height".to_string(), Value::Integer(1), None)
                .with_description("Actual cropped image height in pixels."),
        ]
    }

    /// Executes the crop operation.
    ///
    /// Converts the fractional x, y, width, and height into pixel edges and clamps them to the
    /// source image bounds before cropping.
    pub async fn run(inputs: &mut [Input]) -> Result<OperationResponse, OperationError> {
        let start_time = Instant::now();
        let mut input_errors: Vec<(usize, String)> = vec![];

        // convert inputs
        let image_converted = convert_input(inputs, 0, ValueType::Image, &mut input_errors);
        let x_converted = convert_input(inputs, 1, ValueType::Decimal, &mut input_errors);
        let y_converted = convert_input(inputs, 2, ValueType::Decimal, &mut input_errors);
        let width_converted = convert_input(inputs, 3, ValueType::Decimal, &mut input_errors);
        let height_converted = convert_input(inputs, 4, ValueType::Decimal, &mut input_errors);


        // return if error
        if !input_errors.is_empty() { return Err(OperationError { input_errors, node_error: None }); }

        // get values
        let Value::Image{data, change_id:_} = image_converted.unwrap() else { unreachable!() };
        let Value::Decimal(x) = x_converted.unwrap() else { unreachable!() };
        let Value::Decimal(y) = y_converted.unwrap() else { unreachable!() };
        let Value::Decimal(width) = width_converted.unwrap() else { unreachable!() };
        let Value::Decimal(height) = height_converted.unwrap() else { unreachable!() };

        // run node
        // The parameters are 0-1 fractions of the source size, so resolve them
        // against the actual image to get pixel edges. Rounding the far edge
        // from (origin + size) rather than rounding the size on its own means
        // abutting crops share an edge exactly instead of gapping or
        // overlapping by a pixel.
        let iw = data.width() as i64;
        let ih = data.height() as i64;
        // NaN casts to 0 in Rust, so a garbage fraction degrades to the origin.
        let x0 = ((x * iw as f32).round() as i64).clamp(0, iw - 1);
        let y0 = ((y * ih as f32).round() as i64).clamp(0, ih - 1);
        // Clamp the far edge to at least one pixel past the origin and at most
        // the image edge, so an off-origin crop clips at the right/bottom edge
        // instead of edge-replicating past-the-edge pixels.
        let x1 = ((((x + width) * iw as f32).round()) as i64).clamp(x0 + 1, iw);
        let y1 = ((((y + height) * ih as f32).round()) as i64).clamp(y0 + 1, ih);

        let cx = x0 as u32;
        let cy = y0 as u32;
        let cw = (x1 - x0) as u32;
        let ch = (y1 - y0) as u32;

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
