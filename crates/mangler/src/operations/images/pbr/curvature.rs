use crate::get_id;
use crate::value::ValueType;
use image::DynamicImage;
use crate::input::{Input, InputSettings};
use crate::node_settings::NodeSettings;
use crate::operations::{OperationResponse, OperationError, OutputResponse, default_image, convert_input};
use crate::output::Output;
use crate::value::Value;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use std::time::Instant;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpImagePbrCurvature {}

impl OpImagePbrCurvature {
    pub fn settings() -> NodeSettings {
        NodeSettings {
            name: "curvature".to_string(),
            description: "Detects convex and concave areas from a normal map.".to_string(),
        }
    }

    pub fn create_inputs() -> Vec<Input> {
        vec![
            Input::new("image".to_string(), Value::DynamicImage { data: default_image(), change_id: get_id() }, None, None),
            Input::new("intensity".to_string(), Value::Decimal(1.0), Some(InputSettings::Slider { range: (0.1, 10.0), step_by: Some(0.1), clamp_to_range: true }), None),
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
        let rgba = data.to_rgba32f();
        let width = rgba.width() as i32;
        let height = rgba.height() as i32;
        let intensity = intensity as f32;

        let mut buffer = image::ImageBuffer::new(width as u32, height as u32);

        for y in 0..height {
            for x in 0..width {
                // Read normal x,y components at current pixel and neighbors, clamping to bounds
                let left_x = (x - 1).max(0);
                let right_x = (x + 1).min(width - 1);
                let top_y = (y - 1).max(0);
                let bottom_y = (y + 1).min(height - 1);

                let left_nx = rgba.get_pixel(left_x as u32, y as u32)[0] * 2.0 - 1.0;
                let right_nx = rgba.get_pixel(right_x as u32, y as u32)[0] * 2.0 - 1.0;
                let top_ny = rgba.get_pixel(x as u32, top_y as u32)[1] * 2.0 - 1.0;
                let bottom_ny = rgba.get_pixel(x as u32, bottom_y as u32)[1] * 2.0 - 1.0;

                // Compute divergence of the normal field using finite differences
                let dnx_dx = right_nx - left_nx;
                let dny_dy = bottom_ny - top_ny;
                let curvature_raw = (dnx_dx + dny_dy) * 0.5;

                // Map to output: 0.5 = flat, >0.5 = convex, <0.5 = concave
                let output = (0.5 + curvature_raw * intensity).clamp(0.0, 1.0);

                buffer.put_pixel(x as u32, y as u32, image::Rgba([output, output, output, 1.0]));
            }
        }

        let result = DynamicImage::ImageRgba32F(buffer);

        Ok(OperationResponse {
            time: Instant::now().duration_since(start_time),
            responses: vec![
                OutputResponse { value: Value::DynamicImage { data: Arc::new(result), change_id: get_id() } },
            ],
        })
    }
}
