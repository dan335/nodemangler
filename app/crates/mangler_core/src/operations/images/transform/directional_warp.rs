//! Directional warp operation that displaces pixels along a single angle.
//!
//! Uses [`FloatImage::bilinear_sample`] for channel-agnostic interpolation.

use crate::get_id;
use crate::value::ValueType;
use crate::input::{Input, InputSettings};
use crate::node_settings::NodeSettings;
use crate::operations::{OperationResponse, OperationError, OutputResponse, default_image, convert_input};
use crate::output::Output;
use crate::value::Value;
use crate::float_image::FloatImage;
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
            Input::new("image".to_string(), Value::Image { data: default_image(), change_id: get_id() }, None, None),
            Input::new("intensity map".to_string(), Value::Image { data: default_image(), change_id: get_id() }, None, None),
            Input::new("angle".to_string(), Value::Decimal(0.0), Some(InputSettings::Slider { range: (0.0, 360.0), step_by: Some(0.1), clamp_to_range: false }), None),
            Input::new("intensity".to_string(), Value::Decimal(10.0), Some(InputSettings::Slider { range: (0.0, 200.0), step_by: Some(0.1), clamp_to_range: false }), None),
        ]
    }

    /// Creates the default outputs: the warped image.
    pub fn create_outputs() -> Vec<Output> {
        vec![
            Output::new("output".to_string(), Value::Image { data: default_image(), change_id: get_id() }, None),
        ]
    }

    /// Executes the directional warp by displacing each pixel along the specified angle.
    pub async fn run(inputs: &mut [Input]) -> Result<OperationResponse, OperationError> {
        let start_time = Instant::now();
        let mut input_errors: Vec<(usize, String)> = vec![];

        let image_converted = convert_input(inputs, 0, ValueType::Image, &mut input_errors);
        let map_converted = convert_input(inputs, 1, ValueType::Image, &mut input_errors);
        let angle_converted = convert_input(inputs, 2, ValueType::Decimal, &mut input_errors);
        let intensity_converted = convert_input(inputs, 3, ValueType::Decimal, &mut input_errors);

        if !input_errors.is_empty() { return Err(OperationError { input_errors, node_error: None }); }

        let Value::Image { data: src_data, change_id: _ } = image_converted.unwrap() else { unreachable!() };
        let Value::Image { data: map_data, change_id: _ } = map_converted.unwrap() else { unreachable!() };
        let Value::Decimal(angle) = angle_converted.unwrap() else { unreachable!() };
        let Value::Decimal(intensity) = intensity_converted.unwrap() else { unreachable!() };

        let (w, h) = src_data.dimensions();
        // Output preserves the source image's channel count
        let mut output = FloatImage::new(w, h, src_data.channels());

        // Precompute the unit direction vector from the angle
        let angle_rad = angle.to_radians();
        let dir_x = angle_rad.cos();
        let dir_y = angle_rad.sin();

        // Temporary buffers for bilinear sampling
        let map_ch = map_data.channels() as usize;
        let src_ch = src_data.channels() as usize;
        let mut mp = vec![0.0f32; map_ch];
        let mut sp = vec![0.0f32; src_ch];

        for y in 0..h {
            for x in 0..w {
                // Sample intensity map (resize-aware), mapping output coords to map coords
                let mx = x as f32 * map_data.width() as f32 / w as f32;
                let my = y as f32 * map_data.height() as f32 / h as f32;
                map_data.bilinear_sample(mx, my, &mut mp);

                // Compute luminance using BT.601 coefficients, centered to -0.5..0.5.
                // For single-channel maps, use the value directly.
                let lum = if map_ch >= 3 {
                    mp[0] * 0.299 + mp[1] * 0.587 + mp[2] * 0.114
                } else {
                    mp[0]
                } - 0.5;
                let displacement = lum * intensity;

                let sx = x as f32 + dir_x * displacement;
                let sy = y as f32 + dir_y * displacement;

                // Sample source image at displaced coordinates
                src_data.bilinear_sample(sx, sy, &mut sp);
                output.put_pixel(x, y, &sp);
            }
        }

        Ok(OperationResponse {
            time: Instant::now().duration_since(start_time),
            responses: vec![
                OutputResponse { value: Value::Image { data: Arc::new(output), change_id: get_id() } },
            ],
        })
    }
}

#[cfg(test)]
#[path = "directional_warp_tests.rs"]
mod tests;
