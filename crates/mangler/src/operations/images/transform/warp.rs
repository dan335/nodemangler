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
pub struct OpImageTransformWarp {}

impl OpImageTransformWarp {
    pub fn settings() -> NodeSettings {
        NodeSettings {
            name: "warp".to_string(),
            description: "Displaces pixels using a displacement map. Red channel offsets X, green channel offsets Y.".to_string(),
        }
    }

    pub fn create_inputs() -> Vec<Input> {
        vec![
            Input::new("image".to_string(), Value::DynamicImage { data: default_image(), change_id: get_id() }, None, None),
            Input::new("displacement".to_string(), Value::DynamicImage { data: default_image(), change_id: get_id() }, None, None),
            Input::new("intensity".to_string(), Value::Decimal(10.0), Some(InputSettings::Slider { range: (0.0, 200.0), step_by: Some(0.1), clamp_to_range: false }), None),
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
        let disp_converted = convert_input(inputs, 1, ValueType::DynamicImage, &mut input_errors);
        let intensity_converted = convert_input(inputs, 2, ValueType::Decimal, &mut input_errors);

        if input_errors.len() > 0 { return Err(OperationError { input_errors, node_error: None }); }

        let Value::DynamicImage { data: src_data, change_id: _ } = image_converted.unwrap() else { unreachable!() };
        let Value::DynamicImage { data: disp_data, change_id: _ } = disp_converted.unwrap() else { unreachable!() };
        let Value::Decimal(intensity) = intensity_converted.unwrap() else { unreachable!() };

        let src = src_data.to_rgba8();
        let disp = disp_data.to_rgba8();
        let (w, h) = (src.width(), src.height());
        let mut output = image::RgbaImage::new(w, h);

        for y in 0..h {
            for x in 0..w {
                // Sample displacement map (resize-aware)
                let dx = x as f32 * disp.width() as f32 / w as f32;
                let dy = y as f32 * disp.height() as f32 / h as f32;
                let dp = bilinear_sample_rgba(&disp, dx, dy);

                // Map 0..255 to -0.5..0.5, then multiply by intensity
                let offset_x = (dp[0] as f32 / 255.0 - 0.5) * intensity;
                let offset_y = (dp[1] as f32 / 255.0 - 0.5) * intensity;

                let sx = x as f32 + offset_x;
                let sy = y as f32 + offset_y;

                let pixel = bilinear_sample_rgba(&src, sx, sy);
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

/// Bilinear interpolation sampling with clamped edge handling.
pub fn bilinear_sample_rgba(img: &image::RgbaImage, x: f32, y: f32) -> [u8; 4] {
    let (w, h) = (img.width(), img.height());
    if w == 0 || h == 0 {
        return [0, 0, 0, 0];
    }

    let x0 = (x.floor() as i32).clamp(0, w as i32 - 1) as u32;
    let y0 = (y.floor() as i32).clamp(0, h as i32 - 1) as u32;
    let x1 = (x0 + 1).min(w - 1);
    let y1 = (y0 + 1).min(h - 1);

    let fx = x - x.floor();
    let fy = y - y.floor();

    let p00 = img.get_pixel(x0, y0).0;
    let p10 = img.get_pixel(x1, y0).0;
    let p01 = img.get_pixel(x0, y1).0;
    let p11 = img.get_pixel(x1, y1).0;

    let mut result = [0u8; 4];
    for i in 0..4 {
        let v = p00[i] as f32 * (1.0 - fx) * (1.0 - fy)
            + p10[i] as f32 * fx * (1.0 - fy)
            + p01[i] as f32 * (1.0 - fx) * fy
            + p11[i] as f32 * fx * fy;
        result[i] = v.clamp(0.0, 255.0) as u8;
    }
    result
}
