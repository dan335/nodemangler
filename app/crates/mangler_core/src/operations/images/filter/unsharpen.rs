//! Unsharp mask operation for images.
//!
//! Applies an unsharp mask filter using a Gaussian blur subtraction technique.
//! The sigma controls the blur radius and the threshold determines which edges
//! are enhanced (higher threshold = only stronger edges are sharpened).

use crate::float_image::FloatImage;
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

/// Unsharp mask operation that enhances edges by subtracting a blurred version of the image.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpImageAdjustmentUnsharpen {}

impl OpImageAdjustmentUnsharpen {
    /// Returns the node metadata (name and description) for the unsharpen operation.
    pub fn settings() -> NodeSettings {
        NodeSettings {
            name: "unsharp mask".to_string(),
            description: "Sharpens by subtracting a blurred version. Controls radius and intensity.".to_string(),
        }
    }

    /// Creates the input ports: an image, sigma (blur radius), and threshold (edge sensitivity).
    pub fn create_inputs() -> Vec<Input> {
        vec![
            Input::new("image".to_string(),  Value::Image { data:default_image(), change_id:get_id() }, None, None),
            Input::new("sigma".to_string(), Value::Decimal(1.0), Some(InputSettings::DragValue { speed: None, clamp: Some((0.0, 1000.0)) }), None),
            Input::new("threshold".to_string(), Value::Integer(1), Some(InputSettings::DragValue { speed: None, clamp: None }), None),
        ]
    }

    /// Creates the output port: the unsharp-masked image.
    pub fn create_outputs() -> Vec<Output> {
        vec![
            Output::new("output".to_string(), Value::Image { data:default_image(), change_id:get_id()}, None),
        ]
    }

    /// Executes the unsharp mask. Converts to DynamicImage for the blur step, then back.
    pub async fn run(inputs: &mut [Input]) -> Result<OperationResponse, OperationError> {
        let start_time = Instant::now();
        let mut input_errors: Vec<(usize, String)> = vec![];

        // convert inputs
        let image_converted = convert_input(inputs, 0, ValueType::Image, &mut input_errors);
        let sigma_converted = convert_input(inputs, 1, ValueType::Decimal, &mut input_errors);
        let threshold_converted = convert_input(inputs, 2, ValueType::Integer, &mut input_errors);

        // return if error
        if !input_errors.is_empty() { return Err(OperationError { input_errors, node_error: None }); }

        // get values
        let Value::Image{data, change_id:_} = image_converted.unwrap() else { unreachable!() };
        let Value::Decimal(mut sigma) = sigma_converted.unwrap() else { unreachable!() };
        let Value::Integer(threshold) = threshold_converted.unwrap() else { unreachable!() };

        // run node — use DynamicImage for unsharpen, then convert back.
        // image 0.25's blur rejects sigma=0/subnormal; unsharpen with sigma<=0 is a no-op so pass through.
        sigma = sigma.max(0.0);
        let result = if sigma <= f32::MIN_POSITIVE {
            (*data).clone()
        } else {
            let dynamic = data.to_dynamic();
            let sharpened = dynamic.unsharpen(sigma, threshold);
            FloatImage::from_dynamic(&sharpened)
        };

        Ok(OperationResponse { 
            time: Instant::now().duration_since(start_time),
            responses: vec![
                OutputResponse {value: Value::Image { data:Arc::new(result), change_id:get_id() }},
            ],
        })
    }
}

#[cfg(test)]
#[path = "unsharpen_tests.rs"]
mod tests;
