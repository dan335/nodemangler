//! Highpass filter: subtract a blurred copy from the original.
//!
//! The blur captures low-frequency content; subtracting it leaves the
//! high-frequency detail. Output is biased by +0.5 so mid-grey represents
//! zero difference, matching Photoshop's conventional "High Pass" filter.

use crate::float_image::FloatImage;
use crate::get_id;
use crate::input::{Input, InputSettings};
use crate::node_settings::NodeSettings;
use crate::operations::images::blur::blur::gaussian_blur_image;
use crate::operations::{OperationResponse, OperationError, OutputResponse, default_image, convert_input};
use crate::output::Output;
use crate::value::{Value, ValueType};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use std::time::Instant;

/// Highpass operation — emphasises high-frequency detail.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpImageAdjustmentHighpass {}

impl OpImageAdjustmentHighpass {
    pub fn settings() -> NodeSettings {
        NodeSettings {
            name: "highpass".to_string(),
            description: "Subtracts a blurred copy from the original, keeping only high-frequency detail. Output is biased by 0.5 so mid-grey = zero detail.".to_string(),
        }
    }

    pub fn create_inputs() -> Vec<Input> {
        vec![
            Input::new("image".to_string(), Value::Image { data: default_image(), change_id: get_id() }, None, None),
            Input::new("radius".to_string(), Value::Decimal(4.0), Some(InputSettings::DragValue { speed: None, clamp: Some((0.0, 256.0)) }), None),
        ]
    }

    pub fn create_outputs() -> Vec<Output> {
        vec![
            Output::new("output".to_string(), Value::Image { data: default_image(), change_id: get_id() }, None),
        ]
    }

    pub async fn run(inputs: &mut [Input]) -> Result<OperationResponse, OperationError> {
        let start_time = Instant::now();
        let mut input_errors: Vec<(usize, String)> = vec![];

        let image_converted = convert_input(inputs, 0, ValueType::Image, &mut input_errors);
        let radius_converted = convert_input(inputs, 1, ValueType::Decimal, &mut input_errors);

        if !input_errors.is_empty() { return Err(OperationError { input_errors, node_error: None }); }

        let Value::Image { data, change_id: _ } = image_converted.unwrap() else { unreachable!() };
        let Value::Decimal(radius) = radius_converted.unwrap() else { unreachable!() };

        let blurred = gaussian_blur_image(&data, radius.max(0.0));
        let (width, height) = data.dimensions();
        let ch = data.channels() as usize;
        let has_alpha = ch == 2 || ch == 4;
        let color_ch = if has_alpha { ch - 1 } else { ch };

        let mut output = FloatImage::new(width, height, data.channels());
        let mut buf = [0.0f32; 4];
        for y in 0..height {
            for x in 0..width {
                let src = data.get_pixel(x, y);
                let blur = blurred.get_pixel(x, y);
                for c in 0..color_ch {
                    // +0.5 bias so zero detail sits at mid-grey.
                    buf[c] = (src[c] - blur[c] + 0.5).clamp(0.0, 1.0);
                }
                if has_alpha {
                    buf[ch - 1] = src[ch - 1];
                }
                output.put_pixel(x, y, &buf[..ch]);
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
#[path = "highpass_tests.rs"]
mod tests;
