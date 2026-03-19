//! Directional warp operation that displaces pixels along a single angle.

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

/// Displaces pixels along a fixed angle, with displacement magnitude driven by a grayscale intensity map.
///
/// Unlike the standard warp node which uses separate R/G channels for X/Y offsets,
/// this operation computes luminance from the intensity map and displaces all pixels
/// in a single user-specified direction. This is useful for effects like wind distortion
/// or directional smearing.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpImageTransformDirectionalWarp {}

impl OpImageTransformDirectionalWarp {
    /// Returns the node metadata (name and description) for this operation.
    pub fn settings() -> NodeSettings {
        NodeSettings {
            name: "directional warp".to_string(),
            description: "Displaces pixels along a single angle, with intensity driven by a grayscale map.".to_string(),
        }
    }

    /// Creates the default inputs: source image, grayscale intensity map, angle (degrees), and intensity scalar.
    pub fn create_inputs() -> Vec<Input> {
        vec![
            Input::new("image".to_string(), Value::DynamicImage { data: default_image(), change_id: get_id() }, None, None),
            Input::new("intensity map".to_string(), Value::DynamicImage { data: default_image(), change_id: get_id() }, None, None),
            Input::new("angle".to_string(), Value::Decimal(0.0), Some(InputSettings::Slider { range: (0.0, 360.0), step_by: Some(0.1), clamp_to_range: false }), None),
            Input::new("intensity".to_string(), Value::Decimal(10.0), Some(InputSettings::Slider { range: (0.0, 200.0), step_by: Some(0.1), clamp_to_range: false }), None),
        ]
    }

    /// Creates the default outputs: the warped image.
    pub fn create_outputs() -> Vec<Output> {
        vec![
            Output::new("output".to_string(), Value::DynamicImage { data: default_image(), change_id: get_id() }, None),
        ]
    }

    /// Executes the directional warp by displacing each pixel along the specified angle.
    pub async fn run(inputs: &mut [Input]) -> Result<OperationResponse, OperationError> {
        let start_time = Instant::now();
        let mut input_errors: Vec<(usize, String)> = vec![];

        let image_converted = convert_input(inputs, 0, ValueType::DynamicImage, &mut input_errors);
        let map_converted = convert_input(inputs, 1, ValueType::DynamicImage, &mut input_errors);
        let angle_converted = convert_input(inputs, 2, ValueType::Decimal, &mut input_errors);
        let intensity_converted = convert_input(inputs, 3, ValueType::Decimal, &mut input_errors);

        if !input_errors.is_empty() { return Err(OperationError { input_errors, node_error: None }); }

        let Value::DynamicImage { data: src_data, change_id: _ } = image_converted.unwrap() else { unreachable!() };
        let Value::DynamicImage { data: map_data, change_id: _ } = map_converted.unwrap() else { unreachable!() };
        let Value::Decimal(angle) = angle_converted.unwrap() else { unreachable!() };
        let Value::Decimal(intensity) = intensity_converted.unwrap() else { unreachable!() };

        let src = src_data.to_rgba8();
        let map_img = map_data.to_rgba8();
        let (w, h) = (src.width(), src.height());
        let mut output = image::RgbaImage::new(w, h);

        // Precompute the unit direction vector from the angle
        let angle_rad = angle.to_radians();
        let dir_x = angle_rad.cos();
        let dir_y = angle_rad.sin();

        for y in 0..h {
            for x in 0..w {
                // Sample intensity map (resize-aware)
                let mx = x as f32 * map_img.width() as f32 / w as f32;
                let my = y as f32 * map_img.height() as f32 / h as f32;
                let mp = super::warp::bilinear_sample_rgba(&map_img, mx, my);

                // Compute luminance using BT.601 coefficients, centered to -0.5..0.5
                let lum = (mp[0] as f32 * 0.299 + mp[1] as f32 * 0.587 + mp[2] as f32 * 0.114) / 255.0 - 0.5;
                let displacement = lum * intensity;

                let sx = x as f32 + dir_x * displacement;
                let sy = y as f32 + dir_y * displacement;

                let pixel = super::warp::bilinear_sample_rgba(&src, sx, sy);
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

#[cfg(test)]
#[path = "directional_warp_tests.rs"]
mod tests;
