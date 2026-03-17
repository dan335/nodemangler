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

fn smoothstep(edge0: f64, edge1: f64, x: f64) -> f64 {
    let t = ((x - edge0) / (edge1 - edge0)).clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpImageShapeRectangle {}

impl OpImageShapeRectangle {
    pub fn settings() -> NodeSettings {
        NodeSettings {
            name: "rectangle".to_string(),
            description: "Generates a rectangle shape as a grayscale SDF.".to_string(),
        }
    }

    pub fn create_inputs() -> Vec<Input> {
        vec![
            Input::new("width".to_string(), Value::Integer(512), Some(InputSettings::DragValue { clamp: Some((1.0, 10000.0)), speed: None }), None),
            Input::new("height".to_string(), Value::Integer(512), Some(InputSettings::DragValue { clamp: Some((1.0, 10000.0)), speed: None }), None),
            Input::new("rect_width".to_string(), Value::Decimal(0.5), Some(InputSettings::Slider { range: (0.01, 1.0), step_by: None, clamp_to_range: false }), None),
            Input::new("rect_height".to_string(), Value::Decimal(0.5), Some(InputSettings::Slider { range: (0.01, 1.0), step_by: None, clamp_to_range: false }), None),
            Input::new("corner_radius".to_string(), Value::Decimal(0.0), Some(InputSettings::Slider { range: (0.0, 0.5), step_by: None, clamp_to_range: false }), None),
            Input::new("rotation".to_string(), Value::Decimal(0.0), Some(InputSettings::Slider { range: (0.0, 360.0), step_by: None, clamp_to_range: false }), None),
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
        let width_converted = convert_input(inputs, 0, ValueType::Integer, &mut input_errors);
        let height_converted = convert_input(inputs, 1, ValueType::Integer, &mut input_errors);
        let rect_width_converted = convert_input(inputs, 2, ValueType::Decimal, &mut input_errors);
        let rect_height_converted = convert_input(inputs, 3, ValueType::Decimal, &mut input_errors);
        let corner_radius_converted = convert_input(inputs, 4, ValueType::Decimal, &mut input_errors);
        let rotation_converted = convert_input(inputs, 5, ValueType::Decimal, &mut input_errors);

        // return if error
        if input_errors.len() > 0 { return Err(OperationError { input_errors, node_error: None }); }

        // get values
        let Value::Integer(mut width) = width_converted.unwrap() else { unreachable!() };
        let Value::Integer(mut height) = height_converted.unwrap() else { unreachable!() };
        let Value::Decimal(rect_width) = rect_width_converted.unwrap() else { unreachable!() };
        let Value::Decimal(rect_height) = rect_height_converted.unwrap() else { unreachable!() };
        let Value::Decimal(corner_radius) = corner_radius_converted.unwrap() else { unreachable!() };
        let Value::Decimal(rotation) = rotation_converted.unwrap() else { unreachable!() };

        // run node
        width = width.max(1);
        height = height.max(1);

        let half_w = (rect_width as f64) * 0.5;
        let half_h = (rect_height as f64) * 0.5;
        let r = (corner_radius as f64).min(half_w.min(half_h));
        let angle = (rotation as f64).to_radians();
        let cos_a = angle.cos();
        let sin_a = angle.sin();
        let pixel_size = 1.5 / (width.max(height) as f64 * 0.5);

        let mut image_buffer = ImageBuffer::new(width as u32, height as u32);

        for y in 0..height {
            for x in 0..width {
                // normalize to [-1, 1]
                let nx = (x as f64 / (width as f64 - 1.0).max(1.0)) * 2.0 - 1.0;
                let ny = (y as f64 / (height as f64 - 1.0).max(1.0)) * 2.0 - 1.0;

                // apply rotation
                let px = nx * cos_a + ny * sin_a;
                let py = -nx * sin_a + ny * cos_a;

                // rounded box SDF
                let dx = px.abs() - half_w + r;
                let dy = py.abs() - half_h + r;
                let dist = dx.max(0.0).hypot(dy.max(0.0)) + dx.max(dy).min(0.0) - r;

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
    async fn test_opimageshaperectangle_settings() {
        let s = OpImageShapeRectangle::settings();
        assert_eq!(s.name, "rectangle");
        assert_eq!(OpImageShapeRectangle::create_inputs().len(), 6);
        assert_eq!(OpImageShapeRectangle::create_outputs().len(), 1);
    }


    #[tokio::test]
    async fn test_opimageshaperectangle_run() {
        let mut inputs = vec![
            Input::new("i0".to_string(), Value::Integer(4), None, None),
            Input::new("i1".to_string(), Value::Integer(4), None, None),
            Input::new("i2".to_string(), Value::Integer(4), None, None),
            Input::new("i3".to_string(), Value::Integer(4), None, None),
            Input::new("i4".to_string(), Value::Integer(4), None, None),
            Input::new("i5".to_string(), Value::Integer(4), None, None)
        ];
        let result = OpImageShapeRectangle::run(&mut inputs).await;
        assert!(result.is_ok(), "run failed: {:?}", result.err());
        match &result.unwrap().responses[0].value {
            Value::DynamicImage { .. } => {}
            other => panic!("Expected DynamicImage, got {:?}", other),
        }
    }

    #[tokio::test]
    async fn test_opimageshaperectangle_correct_dimensions() {
        let mut inputs = vec![
            Input::new("width".to_string(), Value::Integer(16), None, None),
            Input::new("height".to_string(), Value::Integer(8), None, None),
            Input::new("rect_width".to_string(), Value::Decimal(0.5), None, None),
            Input::new("rect_height".to_string(), Value::Decimal(0.5), None, None),
            Input::new("corner_radius".to_string(), Value::Decimal(0.0), None, None),
            Input::new("rotation".to_string(), Value::Decimal(0.0), None, None),
        ];
        let result = OpImageShapeRectangle::run(&mut inputs).await.unwrap();
        match &result.responses[0].value {
            Value::DynamicImage { data, .. } => {
                assert_eq!(data.width(), 16);
                assert_eq!(data.height(), 8);
            }
            other => panic!("Expected DynamicImage, got {:?}", other),
        }
    }

    #[tokio::test]
    async fn test_opimageshaperectangle_1x1() {
        let mut inputs = vec![
            Input::new("width".to_string(), Value::Integer(1), None, None),
            Input::new("height".to_string(), Value::Integer(1), None, None),
            Input::new("rect_width".to_string(), Value::Decimal(1.0), None, None),
            Input::new("rect_height".to_string(), Value::Decimal(1.0), None, None),
            Input::new("corner_radius".to_string(), Value::Decimal(0.0), None, None),
            Input::new("rotation".to_string(), Value::Decimal(0.0), None, None),
        ];
        let result = OpImageShapeRectangle::run(&mut inputs).await;
        assert!(result.is_ok(), "1x1 rectangle failed: {:?}", result.err());
    }

}
