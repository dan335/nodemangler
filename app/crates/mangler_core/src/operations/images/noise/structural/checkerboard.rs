//! Checkerboard pattern image generator.
//!
//! Produces a grayscale checkerboard pattern. The size input sets how many
//! squares span the image's larger dimension.

use crate::float_image::FloatImage;
use crate::get_id;
use crate::input::{Input, InputSettings};
use crate::node_settings::NodeSettings;
use crate::operations::{OperationResponse, OperationError, OutputResponse, default_image};
use crate::output::Output;
use crate::value::{Value, ValueType};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use std::time::Instant;

/// Operation that generates a checkerboard pattern as a grayscale image.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpImageNoiseCheckerboard {}

impl OpImageNoiseCheckerboard {
    /// Returns the node metadata (name and description) for this operation.
    pub fn settings() -> NodeSettings {
        NodeSettings {
            name: "checkerboard noise".to_string(),
            description: "Creates a checkerboard noise image.".to_string(),
            help: "Not a stochastic noise at all: a deterministic alternating black/white grid. The size input sets how many squares span the image's larger dimension; the top-left square is white.\n\nHandy as a UV test pattern, a mask for regular alternation, or an input to other nodes (warp, blur, blend) that turn the regular grid into something less obvious.".to_string(),
        }
    }

    /// Creates the default inputs: width, height, and size (number of checkerboard divisions).
    pub fn create_inputs() -> Vec<Input> {
        vec![
            Input::new("width".to_string(), Value::Integer(512), Some(InputSettings::DragValue {clamp:Some((1.0,10000.0)), speed: None }), None)
                .with_description("Output image width in pixels."),
            Input::new("height".to_string(), Value::Integer(512), Some(InputSettings::DragValue {clamp:Some((1.0,10000.0)), speed: None }), None)
                .with_description("Output image height in pixels."),
            Input::new("size".to_string(), Value::Integer(10), Some(InputSettings::DragValue { clamp: Some((1.0, 10000.0)), speed: None }), None)
                .with_description("Number of squares across the image's larger dimension."),
        ]
    }

    /// Creates the default output: a single grayscale image.
    pub fn create_outputs() -> Vec<Output> {
        vec![
            Output::new("output".to_string(), Value::Image { data:default_image(), change_id:get_id() }, None)
                .with_description("Grayscale checkerboard pattern image."),
        ]
    }

    /// Generates a checkerboard pattern image from the given inputs.
    pub async fn run(inputs: &mut [Input]) -> Result<OperationResponse, OperationError> {
        let start_time = Instant::now();

        let Ok(Value::Integer(mut width)) = inputs[0].value.try_convert_to(ValueType::Integer) else { return Err(OperationError { input_errors: vec![(0, "Unable to convert to integer.".to_string())], node_error: None })};
        let Ok(Value::Integer(mut height)) = inputs[1].value.try_convert_to(ValueType::Integer) else { return Err(OperationError { input_errors: vec![(1, "Unable to convert to integer.".to_string())], node_error: None })};
        let Ok(Value::Integer(mut size)) = inputs[2].value.try_convert_to(ValueType::Integer) else { return Err(OperationError { input_errors: vec![(2, "Unable to convert to integer.".to_string())], node_error: None })};
        
        width = width.max(1);
        height = height.max(1);
        size = size.max(1);

        // Build a single-channel FloatImage from the checkerboard pattern
        let mut float_image = FloatImage::new(width as u32, height as u32, 1);

        // Square edge in pixels so that `size` squares span the larger dimension
        let cell = (width.max(height) as f64 / size as f64).max(1.0);

        for y in 0..height {
            for x in 0..width {
                let cx = (x as f64 / cell) as i64;
                let cy = (y as f64 / cell) as i64;
                let value = 1.0 - ((cx + cy) & 1) as f32;
                float_image.put_pixel(x as u32, y as u32, &[value]);
            }
        }

        Ok(OperationResponse { 
            time: Instant::now().duration_since(start_time),
            responses: vec![
                OutputResponse { value: Value::Image { data: Arc::new(float_image), change_id: get_id() } },
            ],
        })
    }
}


#[cfg(test)]
#[path = "checkerboard_tests.rs"]
mod tests;
