//! Star shape image generator.
//!
//! Generates an anti-aliased star polygon as a grayscale SDF image with
//! configurable point count, inner/outer radii, and rotation.

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

/// Computes the unsigned distance from point `(px, py)` to the line segment
/// from `(ax, ay)` to `(bx, by)`.
fn dist_to_segment(px: f64, py: f64, ax: f64, ay: f64, bx: f64, by: f64) -> f64 {
    let dx = bx - ax;
    let dy = by - ay;
    let seg_len_sq = dx * dx + dy * dy;
    if seg_len_sq < 1e-12 {
        return ((px - ax).powi(2) + (py - ay).powi(2)).sqrt();
    }
    let t = ((px - ax) * dx + (py - ay) * dy) / seg_len_sq;
    let t = t.clamp(0.0, 1.0);
    let cx = ax + t * dx;
    let cy = ay + t * dy;
    ((px - cx).powi(2) + (py - cy).powi(2)).sqrt()
}

/// Returns the signed distance for a star polygon.
/// `n` is the number of points, `outer` and `inner` are the two radii.
fn sdf_star(px: f64, py: f64, n: i32, outer: f64, inner: f64) -> f64 {
    let n = n as usize;
    let total_verts = n * 2;
    let angle_step = std::f64::consts::TAU / total_verts as f64;

    // Build vertices alternating outer/inner
    let mut min_dist = f64::MAX;
    // Use ray casting for inside/outside test + min edge distance
    let mut crossings = 0i32;

    let mut prev_vx;
    let mut prev_vy;
    {
        let a = angle_step * (total_verts - 1) as f64 - std::f64::consts::FRAC_PI_2;
        let r = if (total_verts - 1) % 2 == 0 { outer } else { inner };
        prev_vx = r * a.cos();
        prev_vy = r * a.sin();
    }

    for i in 0..total_verts {
        let a = angle_step * i as f64 - std::f64::consts::FRAC_PI_2;
        let r = if i % 2 == 0 { outer } else { inner };
        let vx = r * a.cos();
        let vy = r * a.sin();

        // edge distance
        let d = dist_to_segment(px, py, prev_vx, prev_vy, vx, vy);
        if d < min_dist {
            min_dist = d;
        }

        // ray casting for inside/outside test (ray along +x)
        if (prev_vy > py) != (vy > py) {
            let t = (py - prev_vy) / (vy - prev_vy);
            let ix = prev_vx + t * (vx - prev_vx);
            if px < ix {
                crossings += 1;
            }
        }

        prev_vx = vx;
        prev_vy = vy;
    }

    let inside = crossings % 2 == 1;

    if inside { -min_dist } else { min_dist }
}

/// Operation that generates a star shape as a grayscale SDF image.
///
/// The star is defined by the number of points and two radii (outer and inner),
/// creating alternating spikes. Uses ray-casting for inside/outside determination.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpImageShapeStar {}

impl OpImageShapeStar {
    /// Returns the node metadata (name and description) for this operation.
    pub fn settings() -> NodeSettings {
        NodeSettings {
            name: "star".to_string(),
            description: "Generates a star shape as a grayscale SDF.".to_string(),
        }
    }

    /// Creates the default inputs: width, height, points, outer_radius, inner_radius, and rotation.
    pub fn create_inputs() -> Vec<Input> {
        vec![
            Input::new("width".to_string(), Value::Integer(512), Some(InputSettings::DragValue { clamp: Some((1.0, 10000.0)), speed: None }), None),
            Input::new("height".to_string(), Value::Integer(512), Some(InputSettings::DragValue { clamp: Some((1.0, 10000.0)), speed: None }), None),
            Input::new("points".to_string(), Value::Integer(5), Some(InputSettings::DragValue { clamp: Some((3.0, 64.0)), speed: None }), None),
            Input::new("outer_radius".to_string(), Value::Decimal(0.4), Some(InputSettings::Slider { range: (0.01, 1.0), step_by: None, clamp_to_range: false }), None),
            Input::new("inner_radius".to_string(), Value::Decimal(0.2), Some(InputSettings::Slider { range: (0.01, 1.0), step_by: None, clamp_to_range: false }), None),
            Input::new("rotation".to_string(), Value::Decimal(0.0), Some(InputSettings::Slider { range: (0.0, 360.0), step_by: None, clamp_to_range: false }), None),
        ]
    }

    /// Creates the default output: a single grayscale image.
    pub fn create_outputs() -> Vec<Output> {
        vec![
            Output::new("output".to_string(), Value::DynamicImage { data: default_image(), change_id: get_id() }, None),
        ]
    }

    /// Generates an anti-aliased star shape image from the given inputs.
    pub async fn run(inputs: &mut Vec<Input>) -> Result<OperationResponse, OperationError> {
        let start_time = Instant::now();
        let mut input_errors: Vec<(usize, String)> = vec![];

        // convert inputs
        let width_converted = convert_input(inputs, 0, ValueType::Integer, &mut input_errors);
        let height_converted = convert_input(inputs, 1, ValueType::Integer, &mut input_errors);
        let points_converted = convert_input(inputs, 2, ValueType::Integer, &mut input_errors);
        let outer_radius_converted = convert_input(inputs, 3, ValueType::Decimal, &mut input_errors);
        let inner_radius_converted = convert_input(inputs, 4, ValueType::Decimal, &mut input_errors);
        let rotation_converted = convert_input(inputs, 5, ValueType::Decimal, &mut input_errors);

        // return if error
        if input_errors.len() > 0 { return Err(OperationError { input_errors, node_error: None }); }

        // get values
        let Value::Integer(mut width) = width_converted.unwrap() else { unreachable!() };
        let Value::Integer(mut height) = height_converted.unwrap() else { unreachable!() };
        let Value::Integer(mut points) = points_converted.unwrap() else { unreachable!() };
        let Value::Decimal(outer_radius) = outer_radius_converted.unwrap() else { unreachable!() };
        let Value::Decimal(inner_radius) = inner_radius_converted.unwrap() else { unreachable!() };
        let Value::Decimal(rotation) = rotation_converted.unwrap() else { unreachable!() };

        // run node
        width = width.max(1);
        height = height.max(1);
        points = points.max(3);

        let outer = (outer_radius as f64).max(0.001);
        let inner = (inner_radius as f64).max(0.001);
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

                let dist = sdf_star(px, py, points, outer, inner);

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
    async fn test_opimageshapestar_settings() {
        let s = OpImageShapeStar::settings();
        assert_eq!(s.name, "star");
        assert_eq!(OpImageShapeStar::create_inputs().len(), 6);
        assert_eq!(OpImageShapeStar::create_outputs().len(), 1);
    }


    #[tokio::test]
    async fn test_opimageshapestar_run() {
        let mut inputs = vec![
            Input::new("i0".to_string(), Value::Integer(4), None, None),
            Input::new("i1".to_string(), Value::Integer(4), None, None),
            Input::new("i2".to_string(), Value::Integer(4), None, None),
            Input::new("i3".to_string(), Value::Integer(4), None, None),
            Input::new("i4".to_string(), Value::Integer(4), None, None),
            Input::new("i5".to_string(), Value::Integer(4), None, None)
        ];
        let result = OpImageShapeStar::run(&mut inputs).await;
        assert!(result.is_ok(), "run failed: {:?}", result.err());
        match &result.unwrap().responses[0].value {
            Value::DynamicImage { .. } => {}
            other => panic!("Expected DynamicImage, got {:?}", other),
        }
    }

    #[tokio::test]
    async fn test_opimageshapestar_correct_dimensions() {
        let mut inputs = vec![
            Input::new("width".to_string(), Value::Integer(16), None, None),
            Input::new("height".to_string(), Value::Integer(8), None, None),
            Input::new("points".to_string(), Value::Integer(5), None, None),
            Input::new("outer_radius".to_string(), Value::Decimal(0.4), None, None),
            Input::new("inner_radius".to_string(), Value::Decimal(0.2), None, None),
            Input::new("rotation".to_string(), Value::Decimal(0.0), None, None),
        ];
        let result = OpImageShapeStar::run(&mut inputs).await.unwrap();
        match &result.responses[0].value {
            Value::DynamicImage { data, .. } => {
                assert_eq!(data.width(), 16);
                assert_eq!(data.height(), 8);
            }
            other => panic!("Expected DynamicImage, got {:?}", other),
        }
    }

    #[tokio::test]
    async fn test_opimageshapestar_three_point() {
        // minimum 3-point star
        let mut inputs = vec![
            Input::new("width".to_string(), Value::Integer(8), None, None),
            Input::new("height".to_string(), Value::Integer(8), None, None),
            Input::new("points".to_string(), Value::Integer(3), None, None),
            Input::new("outer_radius".to_string(), Value::Decimal(0.4), None, None),
            Input::new("inner_radius".to_string(), Value::Decimal(0.2), None, None),
            Input::new("rotation".to_string(), Value::Decimal(0.0), None, None),
        ];
        let result = OpImageShapeStar::run(&mut inputs).await;
        assert!(result.is_ok(), "3-point star failed: {:?}", result.err());
    }

}
