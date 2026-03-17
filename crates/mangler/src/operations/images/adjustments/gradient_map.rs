use crate::color::Color;
use crate::get_id;
use crate::input::{Input, InputSettings};
use crate::node_settings::NodeSettings;
use crate::operations::{OperationResponse, OperationError, OutputResponse, default_image, convert_input};
use crate::output::Output;
use crate::value::{Value, ValueType};
use image::{DynamicImage, Rgba32FImage};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use std::time::Instant;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpImageAdjustmentGradientMap {}

impl OpImageAdjustmentGradientMap {
    pub fn settings() -> NodeSettings {
        NodeSettings {
            name: "gradient map".to_string(),
            description: "Maps image luminance to a color gradient.".to_string(),
        }
    }

    pub fn create_inputs() -> Vec<Input> {
        vec![
            Input::new("image".to_string(), Value::DynamicImage { data: default_image(), change_id: get_id() }, None, None),
            Input::new("color a".to_string(), Value::Color(Color::default()), None, None),
            Input::new("color b".to_string(), Value::Color(Color::from_srgb_float(1.0, 1.0, 1.0, 1.0)), None, None),
            Input::new("color c".to_string(), Value::Color(Color::from_srgb_float(0.5, 0.5, 0.5, 1.0)), None, None),
            Input::new("use mid color".to_string(), Value::Bool(false), None, None),
            Input::new("mid position".to_string(), Value::Decimal(0.5), Some(InputSettings::Slider { range: (0.0, 1.0), step_by: Some(0.01), clamp_to_range: true }), None),
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
        let color_a_converted = convert_input(inputs, 1, ValueType::Color, &mut input_errors);
        let color_b_converted = convert_input(inputs, 2, ValueType::Color, &mut input_errors);
        let color_c_converted = convert_input(inputs, 3, ValueType::Color, &mut input_errors);
        let use_mid_converted = convert_input(inputs, 4, ValueType::Bool, &mut input_errors);
        let mid_pos_converted = convert_input(inputs, 5, ValueType::Decimal, &mut input_errors);


        // return if error
        if input_errors.len() > 0 { return Err(OperationError { input_errors, node_error: None }); }

        // get values
        let Value::DynamicImage{data, change_id:_} = image_converted.unwrap() else { unreachable!() };
        let Value::Color(color_a) = color_a_converted.unwrap() else { unreachable!() };
        let Value::Color(color_b) = color_b_converted.unwrap() else { unreachable!() };
        let Value::Color(color_c) = color_c_converted.unwrap() else { unreachable!() };
        let Value::Bool(use_mid) = use_mid_converted.unwrap() else { unreachable!() };
        let Value::Decimal(mid_pos) = mid_pos_converted.unwrap() else { unreachable!() };

        // run node
        let rgba32f = data.to_rgba32f();
        let (width, height) = rgba32f.dimensions();

        let (ar, ag, ab, aa) = color_a.to_srgb_float();
        let (br, bg, bb, ba) = color_b.to_srgb_float();
        let (cr, cg, cb, ca) = color_c.to_srgb_float();

        let mut buffer = Rgba32FImage::new(width, height);

        for (x, y, pixel) in rgba32f.enumerate_pixels() {
            let r = pixel[0];
            let g = pixel[1];
            let b = pixel[2];
            let original_a = pixel[3];

            let lum = (0.2126 * r + 0.7152 * g + 0.0722 * b).clamp(0.0, 1.0);

            let (out_r, out_g, out_b, _out_a) = if use_mid {
                if lum < mid_pos as f32 {
                    let t = if mid_pos > 0.0 { lum / mid_pos as f32 } else { 0.0 };
                    (
                        ar + (cr - ar) * t,
                        ag + (cg - ag) * t,
                        ab + (cb - ab) * t,
                        aa + (ca - aa) * t,
                    )
                } else {
                    let t = if mid_pos < 1.0 { (lum - mid_pos as f32) / (1.0 - mid_pos as f32) } else { 1.0 };
                    (
                        cr + (br - cr) * t,
                        cg + (bg - cg) * t,
                        cb + (bb - cb) * t,
                        ca + (ba - ca) * t,
                    )
                }
            } else {
                (
                    ar + (br - ar) * lum,
                    ag + (bg - ag) * lum,
                    ab + (bb - ab) * lum,
                    aa + (ba - aa) * lum,
                )
            };

            buffer.put_pixel(x, y, image::Rgba([out_r, out_g, out_b, original_a]));
        }

        Ok(OperationResponse {
            time: Instant::now().duration_since(start_time),
            responses: vec![
                OutputResponse {value: Value::DynamicImage { data: Arc::new(DynamicImage::ImageRgba32F(buffer)), change_id: get_id() }},
            ],
        })
    }
}
