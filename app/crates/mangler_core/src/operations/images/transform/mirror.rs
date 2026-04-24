//! Mirror operation that reflects image content across configurable axes.

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

/// Mirrors an image across the X axis, Y axis, or both, with configurable split offsets.
///
/// The offset parameters (0.0 to 1.0) control where the mirror axis sits within the image.
/// At 0.5, the mirror axis is at the center. Pixels on one side of the axis are reflected
/// onto the other side, creating a symmetric result.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpImageTransformMirror {}

impl OpImageTransformMirror {
    /// Returns the node metadata (name and description) for this operation.
    pub fn settings() -> NodeSettings {
        NodeSettings {
            name: "mirror".to_string(),
            description: "Mirrors an image across X, Y, or both axes with configurable offset.".to_string(),
            help: "Reflects pixels around a user-placed axis on each enabled side. `offset x` and `offset y` (0..1) place the split line as a fraction of width/height; the left/top side passes through untouched and the right/bottom side is filled by mirroring back across the split.\n\nWhen the reflection walks off the edge it clamps to the image border rather than wrapping, so an offset near 0 leaves most of the image as a copy of the narrow left/top strip. Output dimensions and channel count match the input, and pixel copies are nearest-neighbour (no interpolation).".to_string(),
        }
    }

    /// Creates the default inputs: source image, mirror X/Y toggles, and X/Y offset positions.
    pub fn create_inputs() -> Vec<Input> {
        vec![
            Input::new("image".to_string(), Value::Image { data: default_image(), change_id: get_id() }, None, None)
                .with_description("Source image to reflect."),
            Input::new("mirror x".to_string(), Value::Bool(true), None, None)
                .with_description("Enable reflection across the vertical axis."),
            Input::new("mirror y".to_string(), Value::Bool(false), None, None)
                .with_description("Enable reflection across the horizontal axis."),
            Input::new("offset x".to_string(), Value::Decimal(0.5), Some(InputSettings::Slider { range: (0.0, 1.0), step_by: Some(0.001), clamp_to_range: true }), None)
                .with_description("Position of the vertical mirror axis as a fraction of width."),
            Input::new("offset y".to_string(), Value::Decimal(0.5), Some(InputSettings::Slider { range: (0.0, 1.0), step_by: Some(0.001), clamp_to_range: true }), None)
                .with_description("Position of the horizontal mirror axis as a fraction of height."),
        ]
    }

    /// Creates the default outputs: the mirrored image.
    pub fn create_outputs() -> Vec<Output> {
        vec![
            Output::new("output".to_string(), Value::Image { data: default_image(), change_id: get_id() }, None)
                .with_description("Image with content reflected across the selected axes."),
        ]
    }

    /// Executes the mirror operation by reflecting pixels across the configured axes.
    pub async fn run(inputs: &mut [Input]) -> Result<OperationResponse, OperationError> {
        let start_time = Instant::now();
        let mut input_errors: Vec<(usize, String)> = vec![];

        let image_converted = convert_input(inputs, 0, ValueType::Image, &mut input_errors);
        let mirror_x_converted = convert_input(inputs, 1, ValueType::Bool, &mut input_errors);
        let mirror_y_converted = convert_input(inputs, 2, ValueType::Bool, &mut input_errors);
        let offset_x_converted = convert_input(inputs, 3, ValueType::Decimal, &mut input_errors);
        let offset_y_converted = convert_input(inputs, 4, ValueType::Decimal, &mut input_errors);

        if !input_errors.is_empty() { return Err(OperationError { input_errors, node_error: None }); }

        let Value::Image { data: src_data, change_id: _ } = image_converted.unwrap() else { unreachable!() };
        let Value::Bool(mirror_x) = mirror_x_converted.unwrap() else { unreachable!() };
        let Value::Bool(mirror_y) = mirror_y_converted.unwrap() else { unreachable!() };
        let Value::Decimal(offset_x) = offset_x_converted.unwrap() else { unreachable!() };
        let Value::Decimal(offset_y) = offset_y_converted.unwrap() else { unreachable!() };

        let (w, h) = src_data.dimensions();
        let mut output = crate::float_image::FloatImage::new(w, h, src_data.channels());

        // Convert normalized offsets to pixel positions for the mirror axes
        let split_x = (w as f32 * offset_x.clamp(0.0, 1.0)) as u32;
        let split_y = (h as f32 * offset_y.clamp(0.0, 1.0)) as u32;

        for y in 0..h {
            for x in 0..w {
                let sx = if mirror_x && x >= split_x {
                    // Reflect: compute distance past the split and map back symmetrically
                    let dist = x - split_x;
                    if split_x as i32 - dist as i32 > 0 {
                        split_x - dist - 1
                    } else {
                        0
                    }
                } else {
                    x
                };

                let sy = if mirror_y && y >= split_y {
                    let dist = y - split_y;
                    if split_y as i32 - dist as i32 > 0 {
                        split_y - dist - 1
                    } else {
                        0
                    }
                } else {
                    y
                };

                let sx = sx.min(w - 1);
                let sy = sy.min(h - 1);
                // Copy pixel preserving all channels natively
                output.put_pixel(x, y, src_data.get_pixel(sx, sy));
            }
        }

        Ok(OperationResponse { 
            time: Instant::now().duration_since(start_time),
            responses: vec![
                OutputResponse { value: Value::Image { data: Arc::new(output), change_id: get_id() } },
            ],
        })
    }
}

#[cfg(test)]
#[path = "mirror_tests.rs"]
mod tests;
