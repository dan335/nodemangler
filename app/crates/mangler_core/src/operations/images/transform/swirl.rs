//! Swirl distortion — radial rotational warp around a configurable centre.
//!
//! Implemented via inverse mapping: for every output pixel we figure out
//! which source coordinate to sample. The sampled coordinate is the output
//! coordinate rotated around the swirl centre by an angle that falls off
//! smoothly with distance, producing the characteristic whirlpool effect.

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

/// Radial rotational distortion (swirl / whirlpool) warp.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpImageTransformSwirl {}

impl OpImageTransformSwirl {
    pub fn settings() -> NodeSettings {
        NodeSettings {
            name: "swirl".to_string(),
            description: "Radial rotational distortion around a configurable centre.".to_string(),
            help: "For every output pixel, converts the coordinate relative to `(centre_x, centre_y)` into polar form, adds an angular offset `angle * (1 − r/radius)²` (zero at the radius, maximum at the centre), and samples the source at the rotated Cartesian coordinate via bilinear interpolation. The squared falloff gives a soft blend into an unswirled outer region.\n\nPixels outside the effective radius are copied through unchanged. Coordinates near image edges are clamped so the output remains fully populated. `angle` is in degrees; positive values spin counter-clockwise. Alpha and every other channel are sampled identically.".to_string(),
        }
    }

    pub fn create_inputs() -> Vec<Input> {
        vec![
            Input::new("image".to_string(), Value::Image { data: default_image(), change_id: get_id() }, None, None)
                .with_description("Source image to swirl."),
            Input::new("center x".to_string(), Value::Decimal(0.5), Some(InputSettings::Slider { range: (0.0, 1.0), step_by: Some(0.001), clamp_to_range: false }), None)
                .with_description("Horizontal centre of the swirl as a fraction of image width."),
            Input::new("center y".to_string(), Value::Decimal(0.5), Some(InputSettings::Slider { range: (0.0, 1.0), step_by: Some(0.001), clamp_to_range: false }), None)
                .with_description("Vertical centre of the swirl as a fraction of image height."),
            Input::new("angle".to_string(), Value::Decimal(180.0), Some(InputSettings::Slider { range: (-1080.0, 1080.0), step_by: Some(1.0), clamp_to_range: false }), None)
                .with_description("Maximum rotation at the centre, in degrees."),
            Input::new("radius".to_string(), Value::Decimal(0.5), Some(InputSettings::Slider { range: (0.0, 2.0), step_by: Some(0.01), clamp_to_range: false }), None)
                .with_description("Effective radius of the swirl as a fraction of the image diagonal."),
        ]
    }

    pub fn create_outputs() -> Vec<Output> {
        vec![
            Output::new("output".to_string(), Value::Image { data: default_image(), change_id: get_id() }, None)
                .with_description("Swirled image with the same dimensions and channel count as the source."),
        ]
    }

    pub async fn run(inputs: &mut [Input]) -> Result<OperationResponse, OperationError> {
        let start_time = Instant::now();
        let mut input_errors: Vec<(usize, String)> = vec![];

        let image_converted = convert_input(inputs, 0, ValueType::Image, &mut input_errors);
        let cx_converted = convert_input(inputs, 1, ValueType::Decimal, &mut input_errors);
        let cy_converted = convert_input(inputs, 2, ValueType::Decimal, &mut input_errors);
        let angle_converted = convert_input(inputs, 3, ValueType::Decimal, &mut input_errors);
        let radius_converted = convert_input(inputs, 4, ValueType::Decimal, &mut input_errors);

        if !input_errors.is_empty() { return Err(OperationError { input_errors, node_error: None }); }

        let Value::Image { data, change_id: _ } = image_converted.unwrap() else { unreachable!() };
        let Value::Decimal(cx) = cx_converted.unwrap() else { unreachable!() };
        let Value::Decimal(cy) = cy_converted.unwrap() else { unreachable!() };
        let Value::Decimal(angle) = angle_converted.unwrap() else { unreachable!() };
        let Value::Decimal(radius_frac) = radius_converted.unwrap() else { unreachable!() };

        let (w, h) = data.dimensions();
        let ch = data.channels() as usize;
        let fw = w as f32;
        let fh = h as f32;

        // Effective radius in pixels, derived from the image diagonal so the
        // input is framing-agnostic.
        let diag = ((fw * fw + fh * fh).sqrt()).max(1.0);
        let r_px = (radius_frac.max(0.0) * diag).max(1.0);
        let max_angle_rad = angle.to_radians();
        let cpx = cx * fw;
        let cpy = cy * fh;

        let mut output = FloatImage::new(w, h, ch as u32);
        let mut sample_buf = [0.0f32; 4];

        for y in 0..h {
            for x in 0..w {
                // Distance from the swirl centre for this output pixel.
                let dx = x as f32 - cpx;
                let dy = y as f32 - cpy;
                let d = (dx * dx + dy * dy).sqrt();

                let (sx, sy) = if d >= r_px {
                    // Outside the effective radius: passthrough.
                    (x as f32, y as f32)
                } else {
                    // Squared falloff: full swirl at d=0, 0 at d=r_px.
                    let t = 1.0 - d / r_px;
                    let rot = max_angle_rad * t * t;
                    let (s, c) = rot.sin_cos();
                    // Inverse map: rotate the delta by `-rot` so the output at
                    // angle θ came from the source at angle θ − rot.
                    let rx = c * dx + s * dy;
                    let ry = -s * dx + c * dy;
                    (cpx + rx, cpy + ry)
                };

                data.bilinear_sample(sx, sy, &mut sample_buf);
                output.put_pixel(x, y, &sample_buf[..ch]);
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
#[path = "swirl_tests.rs"]
mod tests;
