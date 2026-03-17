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
pub struct OpImageAdjustmentSharpen {}

impl OpImageAdjustmentSharpen {
    pub fn settings() -> NodeSettings {
        NodeSettings {
            name: "sharpen".to_string(),
            description: "Sharpens an image using convolution.".to_string(),
        }
    }

    pub fn create_inputs() -> Vec<Input> {
        vec![
            Input::new("image".to_string(), Value::DynamicImage { data: default_image(), change_id: get_id() }, None, None),
            Input::new("intensity".to_string(), Value::Decimal(1.0), Some(InputSettings::Slider { range: (0.0, 10.0), step_by: Some(0.1), clamp_to_range: true }), None),
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
        let intensity_converted = convert_input(inputs, 1, ValueType::Decimal, &mut input_errors);

        // return if error
        if input_errors.len() > 0 { return Err(OperationError { input_errors, node_error: None }); }

        // get values
        let Value::DynamicImage { data, change_id: _ } = image_converted.unwrap() else { unreachable!() };
        let Value::Decimal(intensity) = intensity_converted.unwrap() else { unreachable!() };

        // run node
        let buffer = data.to_rgba32f();
        let (width, height) = (buffer.width(), buffer.height());
        let mut output = buffer.clone();
        let intensity = intensity as f32;

        // Sharpen kernel: center = 1 + 4*intensity, edges = -intensity, corners = 0
        let center = 1.0 + 4.0 * intensity;
        let edge = -intensity;

        for y in 0..height {
            for x in 0..width {
                let sample = |px: u32, py: u32| -> [f32; 3] {
                    let p = buffer.get_pixel(px.clamp(0, width - 1), py.clamp(0, height - 1));
                    [p[0], p[1], p[2]]
                };

                let x0 = if x > 0 { x - 1 } else { 0 };
                let x2 = if x + 1 < width { x + 1 } else { width - 1 };
                let y0 = if y > 0 { y - 1 } else { 0 };
                let y2 = if y + 1 < height { y + 1 } else { height - 1 };

                let c_val = sample(x, y);
                let top = sample(x, y0);
                let bottom = sample(x, y2);
                let left = sample(x0, y);
                let right = sample(x2, y);

                let pixel = output.get_pixel_mut(x, y);
                for c in 0..3 {
                    let val = center * c_val[c]
                        + edge * top[c]
                        + edge * bottom[c]
                        + edge * left[c]
                        + edge * right[c];
                    pixel[c] = val.clamp(0.0, 1.0);
                }
                // alpha unchanged
            }
        }

        let adjusted = DynamicImage::ImageRgba32F(output);

        Ok(OperationResponse {
            time: Instant::now().duration_since(start_time),
            responses: vec![
                OutputResponse { value: Value::DynamicImage { data: Arc::new(adjusted), change_id: get_id() } },
            ],
        })
    }
}
