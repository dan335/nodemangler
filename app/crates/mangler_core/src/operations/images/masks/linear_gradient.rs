//! Linear gradient mask — Lightroom-style graduated filter as a 1-channel mask.
//!
//! Projects every pixel onto a directed axis (angle) and maps the projection
//! through a smoothstep band controlled by `position` and `softness`. Output
//! is a single-channel FloatImage in [0, 1], ready for `blend`'s alpha input.

use crate::float_image::FloatImage;
use crate::get_id;
use crate::input::{Input, InputSettings};
use crate::node_settings::NodeSettings;
use crate::operations::images::adjustments::common::smoothstep;
use crate::operations::{
    OperationError, OperationResponse, OutputResponse, convert_input, default_image,
};
use crate::output::Output;
use crate::value::{Value, ValueType};
use rayon::prelude::*;
use serde::{Deserialize, Serialize};
use std::f32::consts::PI;
use std::sync::Arc;
use std::time::Instant;

/// Linear (graduated) gradient mask generator.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpImageMaskLinearGradient {}

impl OpImageMaskLinearGradient {
    /// Returns the node metadata (name, description, help).
    pub fn settings() -> NodeSettings {
        NodeSettings {
            name: "linear gradient mask".to_string(),
            description: "Soft linear gradient mask — Lightroom-style graduated filter.".to_string(),
            help: "Generates a single-channel mask by projecting each pixel onto an axis set by `angle` and fading through a smoothstep band around `position`.\n\nAngle is in degrees (0 = left→right, 90 = top→bottom, increasing clockwise with the image's y-down convention). `position` (0–1) is where the mid-transition sits along that axis; `softness` (0–1) is the width of the fade (0 = hard step). Invert flips the mask so the selected side swaps.\n\nSet width/height to match the photo you will blend against. Output is raw linear 1-channel — feed it into blend's alpha input (or mask combine) to apply any adjustment locally.".to_string(),
        }
    }

    /// Creates inputs: width, height, angle, position, softness, invert.
    pub fn create_inputs() -> Vec<Input> {
        vec![
            Input::new(
                "width".to_string(),
                Value::Integer(512),
                Some(InputSettings::DragValue {
                    clamp: Some((1.0, 10000.0)),
                    speed: None,
                }),
                None,
            )
            .with_description("Width of the mask in pixels; match your photo."),
            Input::new(
                "height".to_string(),
                Value::Integer(512),
                Some(InputSettings::DragValue {
                    clamp: Some((1.0, 10000.0)),
                    speed: None,
                }),
                None,
            )
            .with_description("Height of the mask in pixels; match your photo."),
            Input::new(
                "angle".to_string(),
                Value::Decimal(90.0),
                Some(InputSettings::Slider {
                    range: (0.0, 360.0),
                    step_by: Some(1.0),
                    clamp_to_range: false,
                }),
                None,
            )
            .with_description("Gradient direction in degrees; 0 = left→right, 90 = top→bottom."),
            Input::new(
                "position".to_string(),
                Value::Decimal(0.5),
                Some(InputSettings::Slider {
                    range: (0.0, 1.0),
                    step_by: Some(0.01),
                    clamp_to_range: true,
                }),
                None,
            )
            .with_description("Location of the mid-transition along the gradient axis (0–1)."),
            Input::new(
                "softness".to_string(),
                Value::Decimal(0.25),
                Some(InputSettings::Slider {
                    range: (0.0, 1.0),
                    step_by: Some(0.01),
                    clamp_to_range: true,
                }),
                None,
            )
            .with_description("Width of the soft fade around position; 0 is a hard step."),
            Input::new("invert".to_string(), Value::Bool(false), None, None)
                .with_description("Flip the mask so the selected side swaps."),
        ]
    }

    /// Creates the single mask output.
    pub fn create_outputs() -> Vec<Output> {
        vec![Output::new(
            "output".to_string(),
            Value::Image {
                data: default_image(),
                change_id: get_id(),
            },
            None,
        )
        .with_description("Single-channel linear gradient mask.")]
    }

    /// Generates the linear gradient mask.
    pub async fn run(inputs: &mut [Input]) -> Result<OperationResponse, OperationError> {
        let start_time = Instant::now();
        let mut input_errors: Vec<(usize, String)> = vec![];

        let width_converted = convert_input(inputs, 0, ValueType::Integer, &mut input_errors);
        let height_converted = convert_input(inputs, 1, ValueType::Integer, &mut input_errors);
        let angle_converted = convert_input(inputs, 2, ValueType::Decimal, &mut input_errors);
        let position_converted = convert_input(inputs, 3, ValueType::Decimal, &mut input_errors);
        let softness_converted = convert_input(inputs, 4, ValueType::Decimal, &mut input_errors);
        let invert_converted = convert_input(inputs, 5, ValueType::Bool, &mut input_errors);

        if !input_errors.is_empty() {
            return Err(OperationError {
                input_errors,
                node_error: None,
            });
        }

        let Value::Integer(mut width) = width_converted.unwrap() else {
            unreachable!()
        };
        let Value::Integer(mut height) = height_converted.unwrap() else {
            unreachable!()
        };
        let Value::Decimal(angle) = angle_converted.unwrap() else {
            unreachable!()
        };
        let Value::Decimal(position) = position_converted.unwrap() else {
            unreachable!()
        };
        let Value::Decimal(softness) = softness_converted.unwrap() else {
            unreachable!()
        };
        let Value::Bool(invert) = invert_converted.unwrap() else {
            unreachable!()
        };

        width = width.clamp(1, 10000);
        height = height.clamp(1, 10000);
        let position = position.clamp(0.0, 1.0);
        let softness = softness.clamp(0.0, 1.0);

        let rad = angle * PI / 180.0;
        let nx = rad.cos();
        let ny = rad.sin();
        // Half-softness band around `position`; zero softness → hard step.
        let half = softness * 0.5;
        let e0 = position - half;
        let e1 = position + half;

        let pixels: Vec<f32> = (0..height)
            .into_par_iter()
            .flat_map_iter(move |y| {
                let fy = (y as f32 + 0.5) / height as f32;
                (0..width).map(move |x| {
                    let fx = (x as f32 + 0.5) / width as f32;
                    // Project from image centre so position 0.5 is neutral.
                    let t = (fx - 0.5) * nx + (fy - 0.5) * ny + 0.5;
                    let mut m = smoothstep(e0, e1, t);
                    if invert {
                        m = 1.0 - m;
                    }
                    m
                })
            })
            .collect();

        let image = FloatImage::from_raw(width as u32, height as u32, 1, pixels).unwrap();

        Ok(OperationResponse {
            time: Instant::now().duration_since(start_time),
            responses: vec![OutputResponse {
                value: Value::Image {
                    data: Arc::new(image),
                    change_id: get_id(),
                },
            }],
        })
    }
}

#[cfg(test)]
#[path = "linear_gradient_tests.rs"]
mod tests;
