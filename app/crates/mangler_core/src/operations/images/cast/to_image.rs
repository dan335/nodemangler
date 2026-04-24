//! Cast-to-image operation for the node graph.
//!
//! Converts a value (bool, integer, decimal, or color) to a 1x1 RGBA image
//! using `try_convert_to`. This provides an explicit cast node for generating
//! images from scalar values.

use crate::get_id;
use crate::input::Input;
use crate::node_settings::NodeSettings;
use crate::operations::{OperationResponse, OperationError, OutputResponse, default_image};
use crate::output::Output;
use crate::value::{Value, ValueType};
use serde::{Deserialize, Serialize};
use std::time::Instant;

/// Node operation that converts a value to a 1x1 image.
///
/// Uses `Value::try_convert_to(ValueType::Image)` for the conversion.
/// Accepts booleans (black/white), integers (grayscale 0–255), decimals
/// (grayscale 0.0–1.0), and colors (sRGBA pixel).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpImageCastToImage {}

impl OpImageCastToImage {
    /// Returns the node metadata (name and description).
    pub fn settings() -> NodeSettings {
        NodeSettings {
            name: "to image".to_string(),
            description: "Converts a value to a 1x1 image.".to_string(),
            help: "Packs the input scalar into a single-pixel RGBA FloatImage via `Value::try_convert_to(ValueType::Image)`. Booleans become black or white, integers are treated as 0-255 grayscale levels, decimals as 0-1 grayscale, and colours are written as their sRGBA float quadruple.\n\nUseful when a downstream node only accepts images (for example a blend operand or a gradient endpoint supplied at runtime). The output is always 1x1; use `resize` if you need a larger constant field, though feeding the value into a dedicated image input is usually cheaper.".to_string(),
        }
    }

    /// Creates the default input list: a single decimal input (0.0–1.0 grayscale).
    pub fn create_inputs() -> Vec<Input> {
        vec![
            Input::new("input".to_string(), Value::Decimal(0.0), None, None)
                .with_description("Scalar value (bool, integer, decimal, or colour) to pack into a 1x1 pixel."),
        ]
    }

    /// Creates the default output list: a single image output.
    pub fn create_outputs() -> Vec<Output> {
        vec![
            Output::new("output".to_string(), Value::Image { data: default_image(), change_id: get_id() }, None)
                .with_description("1x1 RGBA image representing the input value."),
        ]
    }

    /// Executes the cast: converts the input to a DynamicImage via `try_convert_to`.
    pub async fn run(inputs: &mut [Input]) -> Result<OperationResponse, OperationError> {
        let start_time = Instant::now();

        let result = inputs[0].value.try_convert_to(ValueType::Image);

        match result {
            Ok(image_value) => Ok(OperationResponse { 
                time: Instant::now().duration_since(start_time),
                responses: vec![OutputResponse { value: image_value }],
            }),
            Err(_) => Err(OperationError {
                input_errors: vec![(0, "Unable to convert to image.".to_string())],
                node_error: None,
            }),
        }
    }
}

#[cfg(test)]
#[path = "to_image_tests.rs"]
mod tests;
