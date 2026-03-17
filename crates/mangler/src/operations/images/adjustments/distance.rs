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
pub struct OpImageAdjustmentDistance{}

impl OpImageAdjustmentDistance {
    pub fn settings() -> NodeSettings {
        NodeSettings {
            name: "distance".to_string(),
            description: "Computes distance field from a binary image.".to_string(),
        }
    }

    pub fn create_inputs() -> Vec<Input> {
        vec![
            Input::new("image".to_string(),  Value::DynamicImage { data:default_image(), change_id:get_id() }, None, None),
            Input::new("threshold".to_string(), Value::Decimal(0.5), Some(InputSettings::Slider { range: (0.0, 1.0), step_by: Some(0.01), clamp_to_range: true }), None),
            Input::new("spread".to_string(), Value::Decimal(32.0), Some(InputSettings::DragValue { speed: None, clamp: Some((1.0, 256.0)) }), None),
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
        let image_converted = convert_input(inputs, 0, ValueType::DynamicImage, &mut input_errors);
        let threshold_converted = convert_input(inputs, 1, ValueType::Decimal, &mut input_errors);
        let spread_converted = convert_input(inputs, 2, ValueType::Decimal, &mut input_errors);

        // return if error
        if input_errors.len() > 0 { return Err(OperationError { input_errors, node_error: None }); }

        // get values
        let Value::DynamicImage{data, change_id:_} = image_converted.unwrap() else { unreachable!() };
        let Value::Decimal(threshold) = threshold_converted.unwrap() else { unreachable!() };
        let Value::Decimal(spread) = spread_converted.unwrap() else { unreachable!() };

        // run node
        let buffer = data.to_rgba32f();
        let threshold = threshold as f32;
        let spread = (spread as f32).max(1.0);
        let width = buffer.width() as i32;
        let height = buffer.height() as i32;
        let spread_i = spread.ceil() as i32;

        // threshold the image: compute binary mask
        let inside: Vec<bool> = buffer.pixels().map(|pixel| {
            let lum = 0.2126 * pixel[0] + 0.7152 * pixel[1] + 0.0722 * pixel[2];
            lum >= threshold
        }).collect();

        // compute distance transform with brute-force search, limited to spread radius
        let mut distances: Vec<f32> = vec![spread; (width * height) as usize];

        for y in 0..height {
            for x in 0..width {
                let idx = (y * width + x) as usize;
                let is_inside = inside[idx];
                let mut min_dist_sq = spread * spread;

                let y_start = (y - spread_i).max(0);
                let y_end = (y + spread_i).min(height - 1);
                let x_start = (x - spread_i).max(0);
                let x_end = (x + spread_i).min(width - 1);

                for sy in y_start..=y_end {
                    for sx in x_start..=x_end {
                        let sidx = (sy * width + sx) as usize;
                        if inside[sidx] != is_inside {
                            let dx = (sx - x) as f32;
                            let dy = (sy - y) as f32;
                            let dist_sq = dx * dx + dy * dy;
                            if dist_sq < min_dist_sq {
                                min_dist_sq = dist_sq;
                                // early termination: can't get closer than 1 pixel
                                if dist_sq <= 1.0 { break; }
                            }
                        }
                    }
                    if min_dist_sq <= 1.0 { break; }
                }

                let dist = min_dist_sq.sqrt();
                distances[idx] = dist;
            }
        }

        // build output image
        let mut out_buffer = image::Rgba32FImage::new(width as u32, height as u32);
        for y in 0..height {
            for x in 0..width {
                let idx = (y * width + x) as usize;
                let is_inside = inside[idx];
                let normalized_dist = (distances[idx] / spread).clamp(0.0, 1.0);

                let result = if is_inside {
                    0.5 + normalized_dist / 2.0
                } else {
                    0.5 - normalized_dist / 2.0
                };

                let result = result.clamp(0.0, 1.0);
                out_buffer.put_pixel(x as u32, y as u32, image::Rgba([result, result, result, 1.0]));
            }
        }

        let adjusted = DynamicImage::ImageRgba32F(out_buffer);

        Ok(OperationResponse {
            time: Instant::now().duration_since(start_time),
            responses: vec![
                OutputResponse {value: Value::DynamicImage { data:Arc::new(adjusted), change_id:get_id() }},
            ],
        })
    }
}
