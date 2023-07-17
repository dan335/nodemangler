use crate::color::Color;
use crate::get_id;
use crate::value::ValueType;
use crate::input::{Input, InputSettings};
use crate::node_settings::NodeSettings;
use crate::operations::{OperationResponse, OperationError, OutputResponse, default_image};
use crate::output::Output;
use crate::value::Value;
use serde::{Deserialize, Serialize};
use std::time::Instant;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpImageTransformRotateAroundCenter {}

impl OpImageTransformRotateAroundCenter {
    pub fn settings() -> NodeSettings {
        NodeSettings {
            name: "rotate around center".to_string(),
            description: "Rotates an image around its center.".to_string(),
        }
    }

    pub fn create_inputs() -> Vec<Input> {
        vec![
            Input::new("image".to_string(),  Value::DynamicImage { data:default_image(), change_id:get_id() }, None, None),
            Input::new("degrees".to_string(), Value::Decimal(45.0), Some(InputSettings::Slider { range: (0.0, 360.0), step_by: Some(0.01), clamp_to_range:false }), None),
            Input::new("background color".to_string(), Value::Color(Color::from_srgb_u8(0,0,0,0)), None, None),
        ]
    }

    pub fn create_outputs() -> Vec<Output> {
        vec![
            Output::new("output".to_string(), Value::DynamicImage { data:default_image(), change_id:get_id()}, None),
        ]
    }

    pub async fn run(inputs: &mut Vec<Input>) -> Result<OperationResponse, OperationError> {
        let start_time = Instant::now();
        let mut input_errors: Vec<(usize, String)> = vec![];

        // convert inputs
        let image_converted = inputs[0].value.try_convert_to(ValueType::DynamicImage);
        let degrees_converted = inputs[1].value.try_convert_to(ValueType::Decimal);
        let bg_color_converted = inputs[2].value.try_convert_to(ValueType::Color);

        // gather errors
        if image_converted.is_err() { input_errors.push((0, image_converted.as_ref().err().unwrap().message.clone())); }
        if degrees_converted.is_err() { input_errors.push((1, degrees_converted.as_ref().err().unwrap().message.clone())); }
        if bg_color_converted.is_err() { input_errors.push((2, bg_color_converted.as_ref().err().unwrap().message.clone())); }

        // return if error
        if input_errors.len() > 0 { return Err(OperationError { input_errors, node_error: None }); }

        // get values
        let Ok(Value::DynamicImage{data, change_id:_}) = image_converted else { return Err(OperationError { input_errors, node_error: Some("Error converting.".to_string()) }); };
        let Ok(Value::Decimal(degrees)) = degrees_converted else { return Err(OperationError { input_errors, node_error: Some("Error converting.".to_string()) }); };
        let Ok(Value::Color(bg_color)) = bg_color_converted else { return Err(OperationError { input_errors, node_error: Some("Error converting.".to_string()) }); };

        // run node
        let color = bg_color.to_srgb_u8();

        let adjusted = imageproc::geometric_transformations::rotate_about_center(&data.to_rgba8(), degrees.to_radians(), imageproc::geometric_transformations::Interpolation::Bicubic, image::Rgba([color.0,color.1,color.2,color.3]));

        Ok(OperationResponse {
            time: Instant::now().duration_since(start_time),
            responses: vec![
                OutputResponse {value: Value::DynamicImage { data: image::DynamicImage::ImageRgba8(adjusted), change_id:get_id() }},
            ],
        })
    }
}
