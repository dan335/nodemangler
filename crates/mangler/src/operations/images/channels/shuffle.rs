use crate::get_id;
use crate::value::ValueType;
use image::RgbaImage;
use crate::input::{Input, InputSettings};
use crate::node_settings::NodeSettings;
use crate::operations::{OperationResponse, OperationError, OutputResponse, default_image, convert_input};
use crate::output::Output;
use crate::value::Value;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use std::time::Instant;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpImageChannelShuffle {}

impl OpImageChannelShuffle {
    pub fn settings() -> NodeSettings {
        NodeSettings {
            name: "channel shuffle".to_string(),
            description: "Remaps image channels using selectable source channels.".to_string(),
        }
    }

    pub fn create_inputs() -> Vec<Input> {
        vec![
            Input::new("image".to_string(), Value::DynamicImage { data: default_image(), change_id: get_id() }, None, None),
            Input::new("red source".to_string(), Value::Integer(0), Some(InputSettings::Slider { range: (0.0, 3.0), step_by: Some(1.0), clamp_to_range: true }), None),
            Input::new("green source".to_string(), Value::Integer(1), Some(InputSettings::Slider { range: (0.0, 3.0), step_by: Some(1.0), clamp_to_range: true }), None),
            Input::new("blue source".to_string(), Value::Integer(2), Some(InputSettings::Slider { range: (0.0, 3.0), step_by: Some(1.0), clamp_to_range: true }), None),
            Input::new("alpha source".to_string(), Value::Integer(3), Some(InputSettings::Slider { range: (0.0, 3.0), step_by: Some(1.0), clamp_to_range: true }), None),
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
        let red_source_converted = convert_input(inputs, 1, ValueType::Integer, &mut input_errors);
        let green_source_converted = convert_input(inputs, 2, ValueType::Integer, &mut input_errors);
        let blue_source_converted = convert_input(inputs, 3, ValueType::Integer, &mut input_errors);
        let alpha_source_converted = convert_input(inputs, 4, ValueType::Integer, &mut input_errors);

        // return if error
        if input_errors.len() > 0 { return Err(OperationError { input_errors, node_error: None }); }

        // get values
        let Value::DynamicImage{data, change_id:_} = image_converted.unwrap() else { unreachable!() };
        let Value::Integer(red_source) = red_source_converted.unwrap() else { unreachable!() };
        let Value::Integer(green_source) = green_source_converted.unwrap() else { unreachable!() };
        let Value::Integer(blue_source) = blue_source_converted.unwrap() else { unreachable!() };
        let Value::Integer(alpha_source) = alpha_source_converted.unwrap() else { unreachable!() };

        // run node
        let red_idx = (red_source.clamp(0, 3)) as usize;
        let green_idx = (green_source.clamp(0, 3)) as usize;
        let blue_idx = (blue_source.clamp(0, 3)) as usize;
        let alpha_idx = (alpha_source.clamp(0, 3)) as usize;

        let rgba = data.to_rgba8();
        let (width, height) = rgba.dimensions();
        let mut output = RgbaImage::new(width, height);

        for (x, y, pixel) in rgba.enumerate_pixels() {
            let channels = pixel.0;
            output.put_pixel(x, y, image::Rgba([
                channels[red_idx],
                channels[green_idx],
                channels[blue_idx],
                channels[alpha_idx],
            ]));
        }

        Ok(OperationResponse {
            time: Instant::now().duration_since(start_time),
            responses: vec![
                OutputResponse { value: Value::DynamicImage { data: Arc::new(image::DynamicImage::ImageRgba8(output)), change_id: get_id() } },
            ],
        })
    }
}
