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
pub struct OpImageTransformSafeTransform {}

impl OpImageTransformSafeTransform {
    pub fn settings() -> NodeSettings {
        NodeSettings {
            name: "safe transform".to_string(),
            description: "Translate, rotate, and scale with wrapping at edges for seamless tiling.".to_string(),
        }
    }

    pub fn create_inputs() -> Vec<Input> {
        vec![
            Input::new("image".to_string(), Value::DynamicImage { data: default_image(), change_id: get_id() }, None, None),
            Input::new("translate x".to_string(), Value::Decimal(0.0), Some(InputSettings::Slider { range: (-1.0, 1.0), step_by: Some(0.001), clamp_to_range: false }), None),
            Input::new("translate y".to_string(), Value::Decimal(0.0), Some(InputSettings::Slider { range: (-1.0, 1.0), step_by: Some(0.001), clamp_to_range: false }), None),
            Input::new("rotation".to_string(), Value::Decimal(0.0), Some(InputSettings::Slider { range: (0.0, 360.0), step_by: Some(0.1), clamp_to_range: false }), None),
            Input::new("scale".to_string(), Value::Decimal(1.0), Some(InputSettings::Slider { range: (0.01, 4.0), step_by: Some(0.01), clamp_to_range: false }), None),
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
        let tx_converted = convert_input(inputs, 1, ValueType::Decimal, &mut input_errors);
        let ty_converted = convert_input(inputs, 2, ValueType::Decimal, &mut input_errors);
        let rot_converted = convert_input(inputs, 3, ValueType::Decimal, &mut input_errors);
        let scale_converted = convert_input(inputs, 4, ValueType::Decimal, &mut input_errors);

        if input_errors.len() > 0 { return Err(OperationError { input_errors, node_error: None }); }

        let Value::DynamicImage { data: src_data, change_id: _ } = image_converted.unwrap() else { unreachable!() };
        let Value::Decimal(tx) = tx_converted.unwrap() else { unreachable!() };
        let Value::Decimal(ty) = ty_converted.unwrap() else { unreachable!() };
        let Value::Decimal(rotation) = rot_converted.unwrap() else { unreachable!() };
        let Value::Decimal(scale) = scale_converted.unwrap() else { unreachable!() };

        let src = src_data.to_rgba8();
        let (w, h) = (src.width(), src.height());
        let mut output = image::RgbaImage::new(w, h);

        let angle_rad = rotation.to_radians();
        let cos_a = angle_rad.cos();
        let sin_a = angle_rad.sin();
        let cx = w as f32 / 2.0;
        let cy = h as f32 / 2.0;
        let safe_scale = if scale.abs() < 0.001 { 0.001 } else { scale };

        for y in 0..h {
            for x in 0..w {
                // Inverse transform: from output pixel to source pixel
                // 1. Center
                let px = x as f32 - cx;
                let py = y as f32 - cy;
                // 2. Inverse scale
                let px = px / safe_scale;
                let py = py / safe_scale;
                // 3. Inverse rotate
                let rx = px * cos_a + py * sin_a;
                let ry = -px * sin_a + py * cos_a;
                // 4. Un-center and inverse translate
                let sx = rx + cx - tx * w as f32;
                let sy = ry + cy - ty * h as f32;

                // Wrap coordinates for seamless tiling
                let sx = ((sx % w as f32) + w as f32) % w as f32;
                let sy = ((sy % h as f32) + h as f32) % h as f32;

                let pixel = super::warp::bilinear_sample_rgba(&src, sx, sy);
                output.put_pixel(x, y, image::Rgba(pixel));
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
