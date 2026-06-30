//! Swirl (twirl) distortion: rotate pixels around the centre, falling off with radius.
//!
//! The rotation angle is strongest at the centre and smoothly decays to zero at
//! the effect radius, twisting the image into a spiral. Sampling is bilinear.

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

/// Radial swirl / twirl distortion around the image centre.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpImageTransformSwirl {}

impl OpImageTransformSwirl {
    /// Returns the node metadata (name and description) for swirl.
    pub fn settings() -> NodeSettings {
        NodeSettings {
            name: "swirl".to_string(),
            description: "Twists the image around its centre with a rotation that fades out with radius.".to_string(),
            help: "For each output pixel the distance from the centre selects a rotation that is full strength at the centre and smoothly (quadratically) decays to zero at the effect radius. The source is then sampled at the inverse-rotated position, spinning the interior into a spiral while the edges stay put.\n\nAngle is in degrees (positive twists counter-clockwise) and radius is a fraction of the image half-diagonal, so radius 1 reaches the corners. Sampling is bilinear and clamps at the edges. Output dimensions and channel count match the input.".to_string(),
        }
    }

    /// Creates input ports: image, twist angle, and effect radius.
    pub fn create_inputs() -> Vec<Input> {
        vec![
            Input::new("image".to_string(), Value::Image { data: default_image(), change_id: get_id() }, None, None)
                .with_description("Source image to swirl."),
            Input::new("angle".to_string(), Value::Decimal(90.0), Some(InputSettings::Slider { range: (-720.0, 720.0), step_by: Some(1.0), clamp_to_range: false }), None)
                .with_description("Maximum twist at the centre, in degrees."),
            Input::new("radius".to_string(), Value::Decimal(1.0), Some(InputSettings::Slider { range: (0.0, 1.0), step_by: Some(0.01), clamp_to_range: true }), None)
                .with_description("Effect radius as a fraction of the image half-diagonal."),
        ]
    }

    /// Creates the output port: the swirled image.
    pub fn create_outputs() -> Vec<Output> {
        vec![
            Output::new("output".to_string(), Value::Image { data: default_image(), change_id: get_id() }, None)
                .with_description("Image twisted around its centre."),
        ]
    }

    /// Executes the swirl by inverse-rotating each output pixel before sampling.
    pub async fn run(inputs: &mut [Input]) -> Result<OperationResponse, OperationError> {
        let start_time = Instant::now();
        let mut input_errors: Vec<(usize, String)> = vec![];

        let image_converted = convert_input(inputs, 0, ValueType::Image, &mut input_errors);
        let angle_converted = convert_input(inputs, 1, ValueType::Decimal, &mut input_errors);
        let radius_converted = convert_input(inputs, 2, ValueType::Decimal, &mut input_errors);

        if !input_errors.is_empty() { return Err(OperationError { input_errors, node_error: None }); }

        let Value::Image { data, change_id: _ } = image_converted.unwrap() else { unreachable!() };
        let Value::Decimal(angle_deg) = angle_converted.unwrap() else { unreachable!() };
        let Value::Decimal(radius) = radius_converted.unwrap() else { unreachable!() };

        let (w, h) = data.dimensions();
        let ch = data.channels() as usize;
        // Centre on the pixel grid so a zero rotation samples integer source
        // coordinates exactly (a true identity rather than a half-pixel blur).
        let cx = (w as f32 - 1.0) / 2.0;
        let cy = (h as f32 - 1.0) / 2.0;
        let max_r = 0.5 * (((w * w + h * h) as f32).sqrt());
        let eff = (radius * max_r).max(1e-3);
        let ang = angle_deg.to_radians();

        let mut out = FloatImage::new(w, h, data.channels());
        let mut sp = vec![0.0f32; ch];

        for y in 0..h {
            for x in 0..w {
                let dx = x as f32 - cx;
                let dy = y as f32 - cy;
                let dist = (dx * dx + dy * dy).sqrt();
                // Quadratic falloff: full at the centre, zero at the radius.
                let t = (1.0 - dist / eff).clamp(0.0, 1.0);
                let rot = ang * t * t;
                let (s, c) = rot.sin_cos();
                // Inverse rotation by -rot to find the source position.
                let src_dx = dx * c + dy * s;
                let src_dy = -dx * s + dy * c;
                data.bilinear_sample(cx + src_dx, cy + src_dy, &mut sp);
                out.put_pixel(x, y, &sp);
            }
        }

        Ok(OperationResponse {
            time: Instant::now().duration_since(start_time),
            responses: vec![OutputResponse { value: Value::Image { data: Arc::new(out), change_id: get_id() } }],
        })
    }
}

#[cfg(test)]
#[path = "swirl_tests.rs"]
mod tests;
