//! Blit (pixel-copy overlay) compositing operation.
//!
//! Overlays a foreground image onto a background image at a specified x/y
//! position using alpha-aware pixel copying via `image::imageops::overlay`.

use crate::get_id;
use crate::input::{Input, InputSettings};
use crate::node_settings::NodeSettings;
use crate::operations::{OperationResponse, OperationError, OutputResponse, default_image, convert_input};
use crate::output::Output;
use crate::value::{Value, ValueType};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use std::time::Instant;

/// Operation that blits (overlays) a foreground image onto a background.
///
/// Unlike the blend operation, this performs a simple alpha-composited overlay
/// without blend modes, amount controls, or color space selection. It delegates
/// to `image::imageops::overlay` for the actual pixel compositing.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpImageCombineBlit {}

impl OpImageCombineBlit {
    /// Returns the node metadata (name and description) for this operation.
    pub fn settings() -> NodeSettings {
        NodeSettings {
            name: "blit".to_string(),
            description: "Blits an image onto another image.".to_string(),
        }
    }

    /// Creates the input definitions: background image, foreground image, and x/y position.
    pub fn create_inputs() -> Vec<Input> {
        vec![
            Input::new("background".to_string(),  Value::DynamicImage { data:default_image(), change_id:get_id() }, None, None),
            Input::new("foreground".to_string(),  Value::DynamicImage { data:default_image(), change_id:get_id() }, None, None),
            Input::new("position x".to_string(), Value::Integer(i32::default()), Some(InputSettings::DragValue { speed:None, clamp:None }), None),
            Input::new("position y".to_string(), Value::Integer(i32::default()), Some(InputSettings::DragValue { speed:None, clamp:None }), None),
        ]
    }

    /// Creates the output definitions: the composited result image.
    pub fn create_outputs() -> Vec<Output> {
        vec![
            Output::new("output".to_string(), Value::DynamicImage { data:default_image(), change_id:get_id()}, None),
        ]
    }

    /// Executes the operation: overlays the foreground onto the background at the given position.
    pub async fn run(inputs: &mut [Input]) -> Result<OperationResponse, OperationError> {
        let start_time = Instant::now();
        let mut input_errors: Vec<(usize, String)> = vec![];

        // convert inputs
        let background_converted = convert_input(inputs, 0, ValueType::DynamicImage, &mut input_errors);
        let foreground_converted = convert_input(inputs, 1, ValueType::DynamicImage, &mut input_errors);
        let position_x_converted = convert_input(inputs, 2, ValueType::Integer, &mut input_errors);
        let position_y_converted = convert_input(inputs, 3, ValueType::Integer, &mut input_errors);


        // return if error
        if !input_errors.is_empty() { return Err(OperationError { input_errors, node_error: None }); }

        // get values
        let Value::DynamicImage{data:background_arc, change_id:_} = background_converted.unwrap() else { unreachable!() };
        let Value::DynamicImage{data:foreground, change_id:_} = foreground_converted.unwrap() else { unreachable!() };
        let Value::Integer(x) = position_x_converted.unwrap() else { unreachable!() };
        let Value::Integer(y) = position_y_converted.unwrap() else { unreachable!() };

        // run node — try to take ownership of the background to avoid cloning if possible
        let mut background = Arc::try_unwrap(background_arc).unwrap_or_else(|a| (*a).clone());
        image::imageops::overlay(&mut background, &*foreground, x as i64, y as i64);

        Ok(OperationResponse {
            time: Instant::now().duration_since(start_time),
            responses: vec![
                OutputResponse {value: Value::DynamicImage { data: Arc::new(background), change_id:get_id() }},
            ],
        })
    }
}

#[cfg(test)]
#[path = "blit_tests.rs"]
mod tests;
