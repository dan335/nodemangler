use crate::get_id;
use crate::value::ValueType;
use image::RgbaImage;
use crate::input::Input;
use crate::node_settings::NodeSettings;
use crate::operations::{OperationResponse, OperationError, OutputResponse, default_image, convert_input};
use crate::output::Output;
use crate::value::Value;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use std::time::Instant;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpImageChannelMerge {}

impl OpImageChannelMerge {
    pub fn settings() -> NodeSettings {
        NodeSettings {
            name: "channel merge".to_string(),
            description: "Merges R, G, B, A channel images into one RGBA image.".to_string(),
        }
    }

    pub fn create_inputs() -> Vec<Input> {
        vec![
            Input::new("red".to_string(), Value::DynamicImage { data: default_image(), change_id: get_id() }, None, None),
            Input::new("green".to_string(), Value::DynamicImage { data: default_image(), change_id: get_id() }, None, None),
            Input::new("blue".to_string(), Value::DynamicImage { data: default_image(), change_id: get_id() }, None, None),
            Input::new("alpha".to_string(), Value::DynamicImage { data: default_image(), change_id: get_id() }, None, None),
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
        let red_converted = convert_input(inputs, 0, ValueType::DynamicImage, &mut input_errors);
        let green_converted = convert_input(inputs, 1, ValueType::DynamicImage, &mut input_errors);
        let blue_converted = convert_input(inputs, 2, ValueType::DynamicImage, &mut input_errors);
        let alpha_converted = convert_input(inputs, 3, ValueType::DynamicImage, &mut input_errors);

        // return if error
        if input_errors.len() > 0 { return Err(OperationError { input_errors, node_error: None }); }

        // get values
        let Value::DynamicImage{data:red_data, change_id:_} = red_converted.unwrap() else { unreachable!() };
        let Value::DynamicImage{data:green_data, change_id:_} = green_converted.unwrap() else { unreachable!() };
        let Value::DynamicImage{data:blue_data, change_id:_} = blue_converted.unwrap() else { unreachable!() };
        let Value::DynamicImage{data:alpha_data, change_id:_} = alpha_converted.unwrap() else { unreachable!() };

        // run node
        let red_luma = red_data.to_luma8();
        let green_luma = green_data.to_luma8();
        let blue_luma = blue_data.to_luma8();
        let alpha_luma = alpha_data.to_luma8();

        let (width, height) = red_luma.dimensions();
        let mut output = RgbaImage::new(width, height);

        for y in 0..height {
            for x in 0..width {
                let r = red_luma.get_pixel(x, y).0[0];
                let g = if x < green_luma.width() && y < green_luma.height() { green_luma.get_pixel(x, y).0[0] } else { 0 };
                let b = if x < blue_luma.width() && y < blue_luma.height() { blue_luma.get_pixel(x, y).0[0] } else { 0 };
                let a = if x < alpha_luma.width() && y < alpha_luma.height() { alpha_luma.get_pixel(x, y).0[0] } else { 255 };
                output.put_pixel(x, y, image::Rgba([r, g, b, a]));
            }
        }

        Ok(OperationResponse {
            time: Instant::now().duration_since(start_time),
            responses: vec![
                OutputResponse { value: Value::DynamicImage { data: Arc::new(image::DynamicImage::ImageRgba8(output)), change_id: get_id() } },
            ],
        })
    }
}
