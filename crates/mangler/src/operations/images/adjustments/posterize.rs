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
use image::DynamicImage;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpImageAdjustmentPosterize {}

impl OpImageAdjustmentPosterize {
    pub fn settings() -> NodeSettings {
        NodeSettings {
            name: "posterize".to_string(),
            description: "Reduces the number of color levels.".to_string(),
        }
    }

    pub fn create_inputs() -> Vec<Input> {
        vec![
            Input::new("image".to_string(), Value::DynamicImage { data: default_image(), change_id: get_id() }, None, None),
            Input::new("levels".to_string(), Value::Integer(4), Some(InputSettings::DragValue { speed: None, clamp: Some((2.0, 256.0)) }), None),
        ]
    }

    pub fn create_outputs() -> Vec<Output> {
        vec![
            Output::new("output".to_string(), Value::DynamicImage { data: default_image(), change_id: get_id() }, None),
        ]
    }

    pub async fn run(inputs: &mut Vec<Input>) -> Result<OperationResponse, OperationError> {
        let start_time = Instant::now();
        let mut input_errors: Vec<(usize, String)> = vec![];

        // convert inputs
        let image_converted = convert_input(inputs, 0, ValueType::DynamicImage, &mut input_errors);
        let levels_converted = convert_input(inputs, 1, ValueType::Integer, &mut input_errors);

        // return if error
        if input_errors.len() > 0 { return Err(OperationError { input_errors, node_error: None }); }

        // get values
        let Value::DynamicImage { data, change_id: _ } = image_converted.unwrap() else { unreachable!() };
        let Value::Integer(levels) = levels_converted.unwrap() else { unreachable!() };

        // run node
        let mut buffer = data.to_rgba32f();
        let levels = (levels as f32).max(2.0);
        let steps = levels - 1.0;

        for pixel in buffer.pixels_mut() {
            for c in 0..3 {
                let val = pixel[c];
                let quantized = (val * steps + 0.5).floor() / steps;
                pixel[c] = quantized.clamp(0.0, 1.0);
            }
            // alpha unchanged
        }

        let adjusted = DynamicImage::ImageRgba32F(buffer);

        Ok(OperationResponse {
            time: Instant::now().duration_since(start_time),
            responses: vec![
                OutputResponse { value: Value::DynamicImage { data: Arc::new(adjusted), change_id: get_id() } },
            ],
        })
    }
}
