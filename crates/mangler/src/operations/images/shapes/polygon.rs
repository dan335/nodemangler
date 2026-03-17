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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpImageShapePolygon {}

impl OpImageShapePolygon {
    pub fn settings() -> NodeSettings {
        NodeSettings {
            name: "polygon".to_string(),
            description: "Generates a regular polygon shape as a grayscale SDF.".to_string(),
        }
    }

    pub fn create_inputs() -> Vec<Input> {
        vec![
            Input::new("width".to_string(), Value::Integer(512), Some(InputSettings::DragValue { clamp: Some((1.0, 10000.0)), speed: None }), None),
            Input::new("height".to_string(), Value::Integer(512), Some(InputSettings::DragValue { clamp: Some((1.0, 10000.0)), speed: None }), None),
            Input::new("sides".to_string(), Value::Integer(6), Some(InputSettings::DragValue { clamp: Some((3.0, 64.0)), speed: None }), None),
            Input::new("radius".to_string(), Value::Decimal(0.4), Some(InputSettings::Slider { range: (0.01, 1.0), step_by: None, clamp_to_range: false }), None),
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
