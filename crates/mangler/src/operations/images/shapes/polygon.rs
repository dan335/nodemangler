//! Regular polygon shape image generator.
//!
//! Generates an anti-aliased regular polygon as a grayscale SDF image with
//! configurable side count, radius, and rotation.

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

/// Signed distance function for a regular polygon with `n` sides and given `radius`.
fn sdf_polygon(px: f64, py: f64, radius: f64, n: i32) -> f64 {
    let n = n as f64;
    let angle_step = std::f64::consts::TAU / n;
    let half_step = angle_step * 0.5;

    let a = py.atan2(px);
    // wrap angle into one sector
    let sector_angle = ((a % angle_step) + angle_step) % angle_step - half_step;

    let r = (px * px + py * py).sqrt();
    let sx = r * sector_angle.cos();
    let sy = r * sector_angle.sin();

    // distance to the edge of the polygon in this sector
    let edge_dist = sx - radius * half_step.cos();
    let corner_y = sy.abs() - radius * half_step.sin();

    if corner_y > 0.0 {
        (edge_dist * edge_dist + corner_y * corner_y).sqrt()
    } else {
        edge_dist
    }
}

/// Operation that generates a regular polygon shape as a grayscale SDF image.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpImageShapePolygon {}

impl OpImageShapePolygon {
    /// Returns the node metadata (name and description) for this operation.
    pub fn settings() -> NodeSettings {
        NodeSettings {
            name: "polygon".to_string(),
            description: "Generates a regular polygon shape as a grayscale SDF.".to_string(),
        }
    }

    /// Creates the default inputs: width, height, sides, radius, and rotation.
    pub fn create_inputs() -> Vec<Input> {
        vec![
            Input::new("width".to_string(), Value::Integer(512), Some(InputSettings::DragValue { clamp: Some((1.0, 10000.0)), speed: None }), None),
            Input::new("height".to_string(), Value::Integer(512), Some(InputSettings::DragValue { clamp: Some((1.0, 10000.0)), speed: None }), None),
            Input::new("sides".to_string(), Value::Integer(6), Some(InputSettings::DragValue { clamp: Some((3.0, 64.0)), speed: None }), None),
            Input::new("radius".to_string(), Value::Decimal(0.4), Some(InputSettings::Slider { range: (0.01, 1.0), step_by: None, clamp_to_range: false }), None),
            Input::new("rotation".to_string(), Value::Decimal(0.0), Some(InputSettings::Slider { range: (0.0, 360.0), step_by: None, clamp_to_range: false }), None),
        ]
    }

    /// Creates the default output: a single grayscale image.
    pub fn create_outputs() -> Vec<Output> {
        vec![
            Output::new("output".to_string(), Value::DynamicImage { data: default_image(), change_id: get_id() }, None),
        ]
    }

    /// Generates an anti-aliased regular polygon image from the given inputs.
    pub async fn run(inputs: &mut Vec<Input>) -> Result<OperationResponse, OperationError> {
        let start_time = Instant::now();
        let mut input_errors: Vec<(usize, String)> = vec![];

        // convert inputs
        let width_converted = convert_input(inputs, 0, ValueType::Integer, &mut input_errors);
        let height_converted = convert_input(inputs, 1, ValueType::Integer, &mut input_errors);
        let sides_converted = convert_input(inputs, 2, ValueType::Integer, &mut input_errors);
        let radius_converted = convert_input(inputs, 3, ValueType::Decimal, &mut input_errors);
        let rotation_converted = convert_input(inputs, 4, ValueType::Decimal, &mut input_errors);

        // return if error
        if input_errors.len() > 0 { return Err(OperationError { input_errors, node_error: None }); }

        // get values
        let Value::Integer(mut width) = width_converted.unwrap() else { unreachable!() };
        let Value::Integer(mut height) = height_converted.unwrap() else { unreachable!() };
        let Value::Integer(mut sides) = sides_converted.unwrap() else { unreachable!() };
        let Value::Decimal(radius) = radius_converted.unwrap() else { unreachable!() };
        let Value::Decimal(rotation) = rotation_converted.unwrap() else { unreachable!() };

        // run node
        width = width.max(1);
        height = height.max(1);
        sides = sides.max(3);

        let rad = (radius as f64).max(0.001);
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

                let dist = sdf_polygon(px, py, rad, sides);

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
    async fn test_opimageshapepolygon_settings() {
        let s = OpImageShapePolygon::settings();
        assert_eq!(s.name, "polygon");
        assert_eq!(OpImageShapePolygon::create_inputs().len(), 5);
        assert_eq!(OpImageShapePolygon::create_outputs().len(), 1);
    }


    #[tokio::test]
    async fn test_opimageshapepolygon_run() {
        let mut inputs = vec![
            Input::new("i0".to_string(), Value::Integer(4), None, None),
            Input::new("i1".to_string(), Value::Integer(4), None, None),
            Input::new("i2".to_string(), Value::Integer(4), None, None),
            Input::new("i3".to_string(), Value::Integer(4), None, None),
            Input::new("i4".to_string(), Value::Integer(4), None, None)
        ];
        let result = OpImageShapePolygon::run(&mut inputs).await;
        assert!(result.is_ok(), "run failed: {:?}", result.err());
        match &result.unwrap().responses[0].value {
            Value::DynamicImage { .. } => {}
            other => panic!("Expected DynamicImage, got {:?}", other),
        }
    }

    #[tokio::test]
    async fn test_opimageshapepolygon_correct_dimensions() {
        let mut inputs = vec![
            Input::new("width".to_string(), Value::Integer(16), None, None),
            Input::new("height".to_string(), Value::Integer(8), None, None),
            Input::new("sides".to_string(), Value::Integer(6), None, None),
            Input::new("radius".to_string(), Value::Decimal(0.4), None, None),
            Input::new("rotation".to_string(), Value::Decimal(0.0), None, None),
        ];
        let result = OpImageShapePolygon::run(&mut inputs).await.unwrap();
        match &result.responses[0].value {
            Value::DynamicImage { data, .. } => {
                assert_eq!(data.width(), 16);
                assert_eq!(data.height(), 8);
            }
            other => panic!("Expected DynamicImage, got {:?}", other),
        }
    }

    #[tokio::test]
    async fn test_opimageshapepolygon_triangle() {
        // 3-sided polygon should work
        let mut inputs = vec![
            Input::new("width".to_string(), Value::Integer(8), None, None),
            Input::new("height".to_string(), Value::Integer(8), None, None),
            Input::new("sides".to_string(), Value::Integer(3), None, None),
            Input::new("radius".to_string(), Value::Decimal(0.4), None, None),
            Input::new("rotation".to_string(), Value::Decimal(0.0), None, None),
        ];
        let result = OpImageShapePolygon::run(&mut inputs).await;
        assert!(result.is_ok(), "triangle polygon failed: {:?}", result.err());
    }

}
