//! Star shape image generator.
//!
//! Generates an anti-aliased star polygon as a grayscale SDF image with
//! configurable point count, inner/outer radii, and rotation. Outputs a
//! single-channel FloatImage mask with values in [0.0, 1.0].

use crate::float_image::FloatImage;
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
        let r = if (total_verts - 1).is_multiple_of(2) { outer } else { inner };
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
            Output::new("output".to_string(), Value::Image { data: default_image(), change_id: get_id() }, None),
        ]
    }

    /// Generates an anti-aliased star shape image from the given inputs.
    ///
    /// The output is a 1-channel FloatImage where 1.0 = inside the star and
    /// 0.0 = outside, with smooth anti-aliased edges.
    pub async fn run(inputs: &mut [Input]) -> Result<OperationResponse, OperationError> {
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
        if !input_errors.is_empty() { return Err(OperationError { input_errors, node_error: None }); }

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
        // anti-aliasing width in normalized coordinates
        let pixel_size = 1.5 / (width.max(height) as f64 * 0.5);

        // 1-channel grayscale mask
        let mut image = FloatImage::new(width as u32, height as u32, 1);

        for y in 0..height {
            for x in 0..width {
                // normalize to [-1, 1]
                let nx = (x as f64 / (width as f64 - 1.0).max(1.0)) * 2.0 - 1.0;
                let ny = (y as f64 / (height as f64 - 1.0).max(1.0)) * 2.0 - 1.0;

                // apply rotation
                let px = nx * cos_a + ny * sin_a;
                let py = -nx * sin_a + ny * cos_a;

                let dist = sdf_star(px, py, points, outer, inner);

                // smoothstep for anti-aliased edge, result in [0.0, 1.0]
                let alpha = 1.0 - smoothstep(-pixel_size, pixel_size, dist);
                image.put_pixel(x as u32, y as u32, &[alpha as f32]);
            }
        }

        Ok(OperationResponse { 
            time: Instant::now().duration_since(start_time),
            responses: vec![
                OutputResponse { value: Value::Image { data: Arc::new(image), change_id: get_id() } },
            ],
        })
    }
}


#[cfg(test)]
#[path = "star_tests.rs"]
mod tests;
