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
pub struct OpImageTransformMakeTile {}

impl OpImageTransformMakeTile {
    pub fn settings() -> NodeSettings {
        NodeSettings {
            name: "make tile".to_string(),
            description: "Makes an image tile seamlessly by cross-fading overlapping border regions.".to_string(),
        }
    }

    pub fn create_inputs() -> Vec<Input> {
        vec![
            Input::new("image".to_string(), Value::DynamicImage { data: default_image(), change_id: get_id() }, None, None),
            Input::new("blend size".to_string(), Value::Decimal(0.25), Some(InputSettings::Slider { range: (0.01, 0.5), step_by: Some(0.01), clamp_to_range: true }), None),
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
        let blend_converted = convert_input(inputs, 1, ValueType::Decimal, &mut input_errors);

        if input_errors.len() > 0 { return Err(OperationError { input_errors, node_error: None }); }

        let Value::DynamicImage { data: src_data, change_id: _ } = image_converted.unwrap() else { unreachable!() };
        let Value::Decimal(blend_size) = blend_converted.unwrap() else { unreachable!() };

        let src = src_data.to_rgba8();
        let (w, h) = (src.width(), src.height());
        let mut output = src.clone();

        let blend_size = blend_size.clamp(0.01, 0.5);
        let blend_w = (w as f32 * blend_size) as u32;
        let blend_h = (h as f32 * blend_size) as u32;

        if blend_w == 0 || blend_h == 0 {
            return Ok(OperationResponse {
                time: Instant::now().duration_since(start_time),
                responses: vec![
                    OutputResponse { value: Value::DynamicImage { data: Arc::new(image::DynamicImage::ImageRgba8(output)), change_id: get_id() } },
                ],
            });
        }

        // Cross-fade horizontal edges
        for y in 0..h {
            for bx in 0..blend_w {
                let t = bx as f32 / blend_w as f32; // 0 at left edge, 1 at blend boundary
                let left_x = bx;
                let right_x = w - blend_w + bx;

                let left_pixel = src.get_pixel(left_x, y).0;
                let right_pixel = src.get_pixel(right_x, y).0;

                // Blend: at left edge (t=0), use right pixel; at boundary (t=1), use left pixel
                let blended = blend_pixels(&left_pixel, &right_pixel, t);

                output.put_pixel(left_x, y, image::Rgba(blended));
                output.put_pixel(right_x, y, image::Rgba(blend_pixels(&right_pixel, &left_pixel, t)));
            }
        }

        // Cross-fade vertical edges (use the already-horizontally-blended data)
        let h_blended = output.clone();
        for x in 0..w {
            for by in 0..blend_h {
                let t = by as f32 / blend_h as f32;
                let top_y = by;
                let bottom_y = h - blend_h + by;

                let top_pixel = h_blended.get_pixel(x, top_y).0;
                let bottom_pixel = h_blended.get_pixel(x, bottom_y).0;

                let blended = blend_pixels(&top_pixel, &bottom_pixel, t);
                output.put_pixel(x, top_y, image::Rgba(blended));
                output.put_pixel(x, bottom_y, image::Rgba(blend_pixels(&bottom_pixel, &top_pixel, t)));
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

fn blend_pixels(a: &[u8; 4], b: &[u8; 4], t: f32) -> [u8; 4] {
    let mut result = [0u8; 4];
    for i in 0..4 {
        result[i] = (a[i] as f32 * t + b[i] as f32 * (1.0 - t)).clamp(0.0, 255.0) as u8;
    }
    result
}
