//! Line shape image generator.
//!
//! Generates an anti-aliased line segment as a grayscale SDF image with
//! configurable start/end points and thickness.

use image::{ImageBuffer, DynamicImage};
use crate::get_id;
use crate::input::{Input, InputSettings};
use crate::node_settings::NodeSettings;
use crate::operations::{OperationResponse, OperationError, OutputResponse, default_image, convert_input};
use crate::output::Output;
use crate::value::{Value, ValueType};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use std::time::Instant;

/// Hermite interpolation between two edges, producing a smooth transition.
fn smoothstep(edge0: f64, edge1: f64, x: f64) -> f64 {
    let t = ((x - edge0) / (edge1 - edge0)).clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

/// Operation that generates a line segment as a grayscale SDF image.
///
/// The line is defined by start and end points in normalized `[0, 1]` coordinates
/// and a thickness value. Handles the degenerate case where start equals end
/// by rendering a circle.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpImageShapeLine {}

impl OpImageShapeLine {
    /// Returns the node metadata (name and description) for this operation.
    pub fn settings() -> NodeSettings {
        NodeSettings {
            name: "line".to_string(),
            description: "Generates a line shape as a grayscale SDF.".to_string(),
        }
    }

    /// Creates the default inputs: width, height, start_x, start_y, end_x, end_y, and thickness.
    pub fn create_inputs() -> Vec<Input> {
        vec![
            Input::new("width".to_string(), Value::Integer(512), Some(InputSettings::DragValue { clamp: Some((1.0, 10000.0)), speed: None }), None),
            Input::new("height".to_string(), Value::Integer(512), Some(InputSettings::DragValue { clamp: Some((1.0, 10000.0)), speed: None }), None),
            Input::new("start_x".to_string(), Value::Decimal(0.25), Some(InputSettings::Slider { range: (0.0, 1.0), step_by: None, clamp_to_range: false }), None),
            Input::new("start_y".to_string(), Value::Decimal(0.5), Some(InputSettings::Slider { range: (0.0, 1.0), step_by: None, clamp_to_range: false }), None),
            Input::new("end_x".to_string(), Value::Decimal(0.75), Some(InputSettings::Slider { range: (0.0, 1.0), step_by: None, clamp_to_range: false }), None),
            Input::new("end_y".to_string(), Value::Decimal(0.5), Some(InputSettings::Slider { range: (0.0, 1.0), step_by: None, clamp_to_range: false }), None),
            Input::new("thickness".to_string(), Value::Decimal(0.02), Some(InputSettings::Slider { range: (0.001, 0.2), step_by: None, clamp_to_range: false }), None),
        ]
    }

    /// Creates the default output: a single grayscale image.
    pub fn create_outputs() -> Vec<Output> {
        vec![
            Output::new("output".to_string(), Value::DynamicImage { data: default_image(), change_id: get_id() }, None),
        ]
    }

    /// Generates an anti-aliased line segment image from the given inputs.
    pub async fn run(inputs: &mut Vec<Input>) -> Result<OperationResponse, OperationError> {
        let start_time = Instant::now();
        let mut input_errors: Vec<(usize, String)> = vec![];

        // convert inputs
        let width_converted = convert_input(inputs, 0, ValueType::Integer, &mut input_errors);
        let height_converted = convert_input(inputs, 1, ValueType::Integer, &mut input_errors);
        let start_x_converted = convert_input(inputs, 2, ValueType::Decimal, &mut input_errors);
        let start_y_converted = convert_input(inputs, 3, ValueType::Decimal, &mut input_errors);
        let end_x_converted = convert_input(inputs, 4, ValueType::Decimal, &mut input_errors);
        let end_y_converted = convert_input(inputs, 5, ValueType::Decimal, &mut input_errors);
        let thickness_converted = convert_input(inputs, 6, ValueType::Decimal, &mut input_errors);

        // return if error
        if input_errors.len() > 0 { return Err(OperationError { input_errors, node_error: None }); }

        // get values
        let Value::Integer(mut width) = width_converted.unwrap() else { unreachable!() };
        let Value::Integer(mut height) = height_converted.unwrap() else { unreachable!() };
        let Value::Decimal(start_x) = start_x_converted.unwrap() else { unreachable!() };
        let Value::Decimal(start_y) = start_y_converted.unwrap() else { unreachable!() };
        let Value::Decimal(end_x) = end_x_converted.unwrap() else { unreachable!() };
        let Value::Decimal(end_y) = end_y_converted.unwrap() else { unreachable!() };
        let Value::Decimal(thickness) = thickness_converted.unwrap() else { unreachable!() };

        // run node
        width = width.max(1);
        height = height.max(1);

        let half_thick = (thickness as f64).max(0.0001) * 0.5;
        // convert start/end from [0,1] to [-1,1]
        let ax = (start_x as f64) * 2.0 - 1.0;
        let ay = (start_y as f64) * 2.0 - 1.0;
        let bx = (end_x as f64) * 2.0 - 1.0;
        let by = (end_y as f64) * 2.0 - 1.0;

        let dx = bx - ax;
        let dy = by - ay;
        let seg_len_sq = dx * dx + dy * dy;
        let pixel_size = 1.5 / (width.max(height) as f64 * 0.5);

        let mut image_buffer = ImageBuffer::new(width as u32, height as u32);

        for y in 0..height {
            for x in 0..width {
                // normalize to [-1, 1]
                let nx = (x as f64 / (width as f64 - 1.0).max(1.0)) * 2.0 - 1.0;
                let ny = (y as f64 / (height as f64 - 1.0).max(1.0)) * 2.0 - 1.0;

                // line segment SDF
                let dist = if seg_len_sq < 1e-12 {
                    // degenerate line (point)
                    ((nx - ax).powi(2) + (ny - ay).powi(2)).sqrt() - half_thick
                } else {
                    let t = ((nx - ax) * dx + (ny - ay) * dy) / seg_len_sq;
                    let t = t.clamp(0.0, 1.0);
                    let cx = ax + t * dx;
                    let cy = ay + t * dy;
                    ((nx - cx).powi(2) + (ny - cy).powi(2)).sqrt() - half_thick
                };

                let alpha = 1.0 - smoothstep(-pixel_size, pixel_size, dist);
                let g = (alpha * 255.0).clamp(0.0, 255.0) as u8;
                image_buffer.put_pixel(x as u32, y as u32, image::Luma([g]));
            }
        }

        let dynamic_image = DynamicImage::ImageLuma8(image_buffer);

        Ok(OperationResponse {
            time: Instant::now().duration_since(start_time),
            responses: vec![
                OutputResponse { value: Value::DynamicImage { data: Arc::new(dynamic_image), change_id: get_id() } },
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
    use image::{DynamicImage, RgbaImage};
    use std::sync::Arc;

    fn test_image(w: u32, h: u32) -> Arc<DynamicImage> {
        let mut img = RgbaImage::new(w, h);
        for y in 0..h {
            for x in 0..w {
                let r = ((x as f32 / w as f32) * 255.0) as u8;
                let g = ((y as f32 / h as f32) * 255.0) as u8;
                img.put_pixel(x, y, image::Rgba([r, g, 128, 255]));
            }
        }
        Arc::new(DynamicImage::ImageRgba8(img))
    }

    fn image_input(w: u32, h: u32) -> Value {
        Value::DynamicImage { data: test_image(w, h), change_id: get_id() }
    }


    #[tokio::test]
    async fn test_opimageshapeline_settings() {
        let s = OpImageShapeLine::settings();
        assert_eq!(s.name, "line");
        assert_eq!(OpImageShapeLine::create_inputs().len(), 7);
        assert_eq!(OpImageShapeLine::create_outputs().len(), 1);
    }


    #[tokio::test]
    async fn test_opimageshapeline_run() {
        let mut inputs = vec![
            Input::new("i0".to_string(), Value::Integer(4), None, None),
            Input::new("i1".to_string(), Value::Integer(4), None, None),
            Input::new("i2".to_string(), Value::Integer(4), None, None),
            Input::new("i3".to_string(), Value::Integer(4), None, None),
            Input::new("i4".to_string(), Value::Integer(4), None, None),
            Input::new("i5".to_string(), Value::Integer(4), None, None),
            Input::new("i6".to_string(), Value::Integer(4), None, None)
        ];
        let result = OpImageShapeLine::run(&mut inputs).await;
        assert!(result.is_ok(), "run failed: {:?}", result.err());
        match &result.unwrap().responses[0].value {
            Value::DynamicImage { .. } => {}
            other => panic!("Expected DynamicImage, got {:?}", other),
        }
    }

    #[tokio::test]
    async fn test_opimageshapeline_correct_dimensions() {
        let mut inputs = vec![
            Input::new("width".to_string(), Value::Integer(16), None, None),
            Input::new("height".to_string(), Value::Integer(8), None, None),
            Input::new("start_x".to_string(), Value::Decimal(-0.5), None, None),
            Input::new("start_y".to_string(), Value::Decimal(0.0), None, None),
            Input::new("end_x".to_string(), Value::Decimal(0.5), None, None),
            Input::new("end_y".to_string(), Value::Decimal(0.0), None, None),
            Input::new("thickness".to_string(), Value::Decimal(0.05), None, None),
        ];
        let result = OpImageShapeLine::run(&mut inputs).await.unwrap();
        match &result.responses[0].value {
            Value::DynamicImage { data, .. } => {
                assert_eq!(data.width(), 16);
                assert_eq!(data.height(), 8);
            }
            other => panic!("Expected DynamicImage, got {:?}", other),
        }
    }

    #[tokio::test]
    async fn test_opimageshapeline_zero_length() {
        // A line where start == end (zero-length)
        let mut inputs = vec![
            Input::new("width".to_string(), Value::Integer(8), None, None),
            Input::new("height".to_string(), Value::Integer(8), None, None),
            Input::new("start_x".to_string(), Value::Decimal(0.0), None, None),
            Input::new("start_y".to_string(), Value::Decimal(0.0), None, None),
            Input::new("end_x".to_string(), Value::Decimal(0.0), None, None),
            Input::new("end_y".to_string(), Value::Decimal(0.0), None, None),
            Input::new("thickness".to_string(), Value::Decimal(0.05), None, None),
        ];
        let result = OpImageShapeLine::run(&mut inputs).await;
        assert!(result.is_ok(), "zero-length line failed: {:?}", result.err());
    }

}
