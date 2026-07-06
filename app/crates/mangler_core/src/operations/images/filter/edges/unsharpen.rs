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
use crate::operations::{OperationResponse, OperationError, OutputResponse, default_image, convert_input, scale_to_resolution};
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
            help: "Classic unsharp mask: Gaussian-blur the image with the given sigma and output `source + (source - blur)` so the high-frequency detail above `threshold` is added back on top of itself. Larger sigma widens the sharpening halo; threshold spares flat, low-contrast areas to avoid amplifying noise.\n\nImplemented via the `image` crate's `unsharpen`, which requires a positive sigma; sigma = 0 passes the image through unchanged. Unlike `sharpen`, the radius is tunable, so this can target coarse or fine detail instead of only the 3x3 neighborhood.".to_string(),
        }
    }

    /// Creates the input ports: an image, sigma (blur radius), and threshold (edge sensitivity).
    pub fn create_inputs() -> Vec<Input> {
        vec![
            Input::new("image".to_string(),  Value::Image { data:default_image(), change_id:get_id() }, None, None)
                .with_description("Source image to sharpen via unsharp masking."),
            Input::new("sigma".to_string(), Value::Decimal(1.0), Some(InputSettings::DragValue { speed: None, clamp: Some((0.0, 1000.0)) }), None)
                .with_description("Gaussian blur standard deviation, in pixels at a 1024px reference (scales with image size); larger values widen the sharpening halo."),
            Input::new("threshold".to_string(), Value::Integer(1), Some(InputSettings::DragValue { speed: None, clamp: None }), None)
                .with_description("Minimum local contrast required before a pixel is sharpened; higher values spare flat areas."),
        ]
    }

    /// Creates the output port: the unsharp-masked image.
    pub fn create_outputs() -> Vec<Output> {
        vec![
            Output::new("output".to_string(), Value::Image { data:default_image(), change_id:get_id()}, None)
                .with_description("Unsharp-masked image with edge contrast boosted."),
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
        // Sigma is authored in reference pixels (at 1024px) and scaled to the
        // actual image, so the sharpening halo looks the same at any resolution.
        let (w, h) = data.dimensions();
        sigma = scale_to_resolution(sigma.max(0.0), w, h);
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
