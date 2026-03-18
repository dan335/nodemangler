//! Distance field computation for images.
//!
//! Converts a grayscale image into a signed distance field by thresholding
//! pixels into inside/outside regions, then computing the minimum Euclidean
//! distance to the nearest boundary pixel. Output is normalized with 0.5 at
//! the boundary, values above 0.5 for inside regions, and below 0.5 for outside.

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

/// Distance field operation that computes a signed distance from a binary threshold.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpImageAdjustmentDistance{}

impl OpImageAdjustmentDistance {
    /// Returns the node metadata (name and description) for the distance operation.
    pub fn settings() -> NodeSettings {
        NodeSettings {
            name: "distance".to_string(),
            description: "Computes distance field from a binary image.".to_string(),
        }
    }

    /// Creates the input ports: image, luminance threshold for the binary mask, and spread
    /// (maximum search radius in pixels).
    pub fn create_inputs() -> Vec<Input> {
        vec![
            Input::new("image".to_string(),  Value::DynamicImage { data:default_image(), change_id:get_id() }, None, None),
            Input::new("threshold".to_string(), Value::Decimal(0.5), Some(InputSettings::Slider { range: (0.0, 1.0), step_by: Some(0.01), clamp_to_range: true }), None),
            Input::new("spread".to_string(), Value::Decimal(32.0), Some(InputSettings::DragValue { speed: None, clamp: Some((1.0, 256.0)) }), None),
        ]
    }

    /// Creates the output port: the distance field image.
    pub fn create_outputs() -> Vec<Output> {
        vec![
            Output::new("output".to_string(), Value::DynamicImage { data:default_image(), change_id:get_id()}, None),
        ]
    }

    /// Executes the distance field computation using brute-force nearest-boundary search
    /// within the spread radius.
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

                // Map distance to [0, 1]: inside pixels > 0.5, outside pixels < 0.5
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

#[cfg(test)]
mod tests {
    use super::*;

    use crate::get_id;
    use crate::input::Input;
    use crate::value::Value;
    use image::DynamicImage;
    use std::sync::Arc;

    fn test_image(w: u32, h: u32) -> Arc<DynamicImage> {
        let mut imgbuf = image::RgbaImage::new(w, h);
        for (x, y, pixel) in imgbuf.enumerate_pixels_mut() {
            let r = (x * 255 / w.max(1)) as u8;
            let g = (y * 255 / h.max(1)) as u8;
            *pixel = image::Rgba([r, g, 128, 255]);
        }
        Arc::new(DynamicImage::ImageRgba8(imgbuf))
    }

    fn image_input(w: u32, h: u32) -> Value {
        Value::DynamicImage { data: test_image(w, h), change_id: get_id() }
    }

    #[tokio::test]
    async fn test_distance_settings() {
        let s = OpImageAdjustmentDistance::settings();
        assert_eq!(s.name, "distance");
        assert_eq!(OpImageAdjustmentDistance::create_inputs().len(), 3);
        assert_eq!(OpImageAdjustmentDistance::create_outputs().len(), 1);
    }

    #[tokio::test]
    async fn test_distance_basic() {
        let mut inputs = vec![
            Input::new("image".to_string(), image_input(8, 8), None, None),
            Input::new("threshold".to_string(), Value::Decimal(0.5), None, None),
            Input::new("spread".to_string(), Value::Decimal(8.0), None, None),
        ];
        let result = OpImageAdjustmentDistance::run(&mut inputs).await.unwrap();
        match &result.responses[0].value {
            Value::DynamicImage { data, .. } => {
                assert_eq!(data.width(), 8);
                assert_eq!(data.height(), 8);
            }
            other => panic!("Expected DynamicImage, got {:?}", other),
        }
    }

    #[tokio::test]
    async fn test_distance_1x1() {
        let mut imgbuf = image::RgbaImage::new(1, 1);
        imgbuf.put_pixel(0, 0, image::Rgba([255u8, 255, 255, 255]));
        let img = Arc::new(DynamicImage::ImageRgba8(imgbuf));
        let mut inputs = vec![
            Input::new("image".to_string(), Value::DynamicImage { data: img, change_id: get_id() }, None, None),
            Input::new("threshold".to_string(), Value::Decimal(0.5), None, None),
            Input::new("spread".to_string(), Value::Decimal(4.0), None, None),
        ];
        let result = OpImageAdjustmentDistance::run(&mut inputs).await;
        assert!(result.is_ok(), "distance 1x1 failed: {:?}", result.err());
    }

    #[tokio::test]
    async fn test_distance_output_range() {
        let mut inputs = vec![
            Input::new("image".to_string(), image_input(8, 8), None, None),
            Input::new("threshold".to_string(), Value::Decimal(0.5), None, None),
            Input::new("spread".to_string(), Value::Decimal(8.0), None, None),
        ];
        let result = OpImageAdjustmentDistance::run(&mut inputs).await.unwrap();
        match &result.responses[0].value {
            Value::DynamicImage { data, .. } => {
                for pixel in data.to_rgba32f().pixels() {
                    assert!(pixel[0] >= 0.0 && pixel[0] <= 1.0, "pixel out of range: {}", pixel[0]);
                }
            }
            other => panic!("Expected DynamicImage, got {:?}", other),
        }
    }

    #[tokio::test]
    async fn test_distance_all_white() {
        let white = {
            let img = image::RgbaImage::from_pixel(8, 8, image::Rgba([255, 255, 255, 255]));
            Arc::new(DynamicImage::ImageRgba8(img))
        };
        let mut inputs = vec![
            Input::new("image".to_string(), Value::DynamicImage { data: white, change_id: get_id() }, None, None),
            Input::new("threshold".to_string(), Value::Decimal(0.5), None, None),
            Input::new("spread".to_string(), Value::Decimal(8.0), None, None),
        ];
        let result = OpImageAdjustmentDistance::run(&mut inputs).await.unwrap();
        match &result.responses[0].value {
            Value::DynamicImage { data, .. } => {
                let buf = data.to_rgba32f();
                let p = buf.get_pixel(4, 4).0;
                assert!(p[0] >= 0.5, "Inside pixel should be >= 0.5, got {}", p[0]);
            }
            other => panic!("Expected DynamicImage, got {:?}", other),
        }
    }
}
