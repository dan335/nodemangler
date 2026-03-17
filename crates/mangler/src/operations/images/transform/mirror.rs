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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpImageTransformMirror {}

impl OpImageTransformMirror {
    pub fn settings() -> NodeSettings {
        NodeSettings {
            name: "mirror".to_string(),
            description: "Mirrors an image across X, Y, or both axes with configurable offset.".to_string(),
        }
    }

    pub fn create_inputs() -> Vec<Input> {
        vec![
            Input::new("image".to_string(), Value::DynamicImage { data: default_image(), change_id: get_id() }, None, None),
            Input::new("mirror x".to_string(), Value::Bool(true), None, None),
            Input::new("mirror y".to_string(), Value::Bool(false), None, None),
            Input::new("offset x".to_string(), Value::Decimal(0.5), Some(InputSettings::Slider { range: (0.0, 1.0), step_by: Some(0.001), clamp_to_range: true }), None),
            Input::new("offset y".to_string(), Value::Decimal(0.5), Some(InputSettings::Slider { range: (0.0, 1.0), step_by: Some(0.001), clamp_to_range: true }), None),
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

        let image_converted = convert_input(inputs, 0, ValueType::DynamicImage, &mut input_errors);
        let mirror_x_converted = convert_input(inputs, 1, ValueType::Bool, &mut input_errors);
        let mirror_y_converted = convert_input(inputs, 2, ValueType::Bool, &mut input_errors);
        let offset_x_converted = convert_input(inputs, 3, ValueType::Decimal, &mut input_errors);
        let offset_y_converted = convert_input(inputs, 4, ValueType::Decimal, &mut input_errors);

        if input_errors.len() > 0 { return Err(OperationError { input_errors, node_error: None }); }

        let Value::DynamicImage { data: src_data, change_id: _ } = image_converted.unwrap() else { unreachable!() };
        let Value::Bool(mirror_x) = mirror_x_converted.unwrap() else { unreachable!() };
        let Value::Bool(mirror_y) = mirror_y_converted.unwrap() else { unreachable!() };
        let Value::Decimal(offset_x) = offset_x_converted.unwrap() else { unreachable!() };
        let Value::Decimal(offset_y) = offset_y_converted.unwrap() else { unreachable!() };

        let src = src_data.to_rgba8();
        let (w, h) = (src.width(), src.height());
        let mut output = image::RgbaImage::new(w, h);

        let split_x = (w as f32 * offset_x.clamp(0.0, 1.0)) as u32;
        let split_y = (h as f32 * offset_y.clamp(0.0, 1.0)) as u32;

        for y in 0..h {
            for x in 0..w {
                let sx = if mirror_x && x >= split_x {
                    // Mirror: reflect around split_x
                    let dist = x - split_x;
                    if split_x as i32 - dist as i32 - 1 >= 0 {
                        split_x - dist - 1
                    } else {
                        0
                    }
                } else {
                    x
                };

                let sy = if mirror_y && y >= split_y {
                    let dist = y - split_y;
                    if split_y as i32 - dist as i32 - 1 >= 0 {
                        split_y - dist - 1
                    } else {
                        0
                    }
                } else {
                    y
                };

                let sx = sx.min(w - 1);
                let sy = sy.min(h - 1);
                output.put_pixel(x, y, *src.get_pixel(sx, sy));
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
