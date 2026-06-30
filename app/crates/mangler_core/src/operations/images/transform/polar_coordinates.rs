//! Polar coordinate transform: wrap a rectangular image into a disk or unwrap.
//!
//! With `to polar` enabled the output disk's angle comes from the source's X
//! axis and its radius from the source's Y axis (rectangular → polar). With it
//! disabled the inverse mapping unwraps a polar image back to rectangular.
//! All sampling is bilinear and channel-agnostic.

use crate::get_id;
use crate::value::ValueType;
use crate::input::Input;
use crate::node_settings::NodeSettings;
use crate::operations::{OperationResponse, OperationError, OutputResponse, default_image, convert_input};
use crate::output::Output;
use crate::value::Value;
use crate::float_image::FloatImage;
use serde::{Deserialize, Serialize};
use std::f32::consts::PI;
use std::sync::Arc;
use std::time::Instant;

/// Rectangular ↔ polar coordinate remap.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpImageTransformPolarCoordinates {}

impl OpImageTransformPolarCoordinates {
    /// Returns the node metadata (name and description) for polar coordinates.
    pub fn settings() -> NodeSettings {
        NodeSettings {
            name: "polar coordinates".to_string(),
            description: "Wraps a rectangular image into a polar disk, or unwraps a polar image back to rectangular.".to_string(),
            help: "When `to polar` is on, each output pixel's angle (from the image centre) selects a source column and its distance from the centre selects a source row, wrapping the rectangle around into a disk. The inner disk radius spans half the shorter image dimension. When `to polar` is off the inverse mapping is applied, unwrapping a centred polar image back into a rectangle (angle along X, radius along Y).\n\nSampling is bilinear and clamps at the source edges, so the seam where angle wraps from 360 back to 0 stays continuous if the source tiles horizontally. Output dimensions and channel count match the input.".to_string(),
        }
    }

    /// Creates input ports: source image and a direction toggle.
    pub fn create_inputs() -> Vec<Input> {
        vec![
            Input::new("image".to_string(), Value::Image { data: default_image(), change_id: get_id() }, None, None)
                .with_description("Source image to remap between rectangular and polar space."),
            Input::new("to polar".to_string(), Value::Bool(true), None, None)
                .with_description("On: rectangular → polar (wrap into a disk). Off: polar → rectangular (unwrap)."),
        ]
    }

    /// Creates the output port: the remapped image.
    pub fn create_outputs() -> Vec<Output> {
        vec![
            Output::new("output".to_string(), Value::Image { data: default_image(), change_id: get_id() }, None)
                .with_description("Image remapped between rectangular and polar coordinates."),
        ]
    }

    /// Executes the polar remap by inverse-sampling the source for each output pixel.
    pub async fn run(inputs: &mut [Input]) -> Result<OperationResponse, OperationError> {
        let start_time = Instant::now();
        let mut input_errors: Vec<(usize, String)> = vec![];

        let image_converted = convert_input(inputs, 0, ValueType::Image, &mut input_errors);
        let to_polar_converted = convert_input(inputs, 1, ValueType::Bool, &mut input_errors);

        if !input_errors.is_empty() { return Err(OperationError { input_errors, node_error: None }); }

        let Value::Image { data, change_id: _ } = image_converted.unwrap() else { unreachable!() };
        let Value::Bool(to_polar) = to_polar_converted.unwrap() else { unreachable!() };

        let (w, h) = data.dimensions();
        let ch = data.channels() as usize;
        let cx = (w as f32 - 1.0) / 2.0;
        let cy = (h as f32 - 1.0) / 2.0;
        let max_r = 0.5 * (w.min(h) as f32).max(1.0);

        let mut out = FloatImage::new(w, h, data.channels());
        let mut sp = vec![0.0f32; ch];

        for y in 0..h {
            for x in 0..w {
                let (sx, sy) = if to_polar {
                    // Output is the polar disk; map its angle/radius back to a
                    // source column/row.
                    let dx = x as f32 - cx;
                    let dy = y as f32 - cy;
                    let r = (dx * dx + dy * dy).sqrt();
                    let angle = dy.atan2(dx); // [-PI, PI]
                    let u = (angle / (2.0 * PI) + 0.5) * w as f32;
                    let v = (r / max_r) * h as f32;
                    (u, v)
                } else {
                    // Output is rectangular; map its X/Y to a position in the
                    // source disk.
                    let angle = (x as f32 + 0.5) / w as f32 * 2.0 * PI - PI;
                    let r = (y as f32 + 0.5) / h as f32 * max_r;
                    (cx + r * angle.cos(), cy + r * angle.sin())
                };
                data.bilinear_sample(sx, sy, &mut sp);
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
#[path = "polar_coordinates_tests.rs"]
mod tests;
