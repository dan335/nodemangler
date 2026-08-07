//! Radial gradient mask — soft elliptical falloff as a 1-channel mask.
//!
//! Distinct from the hard-edged `circle`/`ellipse` shape SDFs and from
//! `vignette` (which darkens an image in place). Emits a soft disc that is
//! fully selected inside `radius` and fades over `softness`.

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
use std::sync::Arc;
use std::time::Instant;

/// Soft radial / elliptical gradient mask generator.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpImageMaskRadialGradient {}

impl OpImageMaskRadialGradient {
    /// Returns the node metadata (name, description, help).
    pub fn settings() -> NodeSettings {
        NodeSettings {
            name: "radial gradient mask".to_string(),
            description: "Soft radial or elliptical mask — Lightroom-style radial filter.".to_string(),
            help: "Generates a single-channel mask that is fully selected (1) inside `radius` from the centre and fades to 0 over `softness`. Distance is measured in aspect-corrected unit space so `aspect` ≠ 1 produces an ellipse (y stretched by aspect).\n\n`center x` / `center y` are normalized 0–1 (0.5 = middle). `radius` and `softness` are fractions of half the shorter image dimension, so a radius of 1 reaches the nearer edge on a square canvas. Invert flips selected/unselected.\n\nUnlike the circle/ellipse shape nodes (hard SDF edges with ~1.5 px AA) this is a soft graduated filter. Unlike vignette it emits a mask rather than darkening RGB. Match width/height to the photo you will blend against.".to_string(),
        }
    }

    /// Creates inputs: width, height, center, radius, softness, aspect, invert.
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
                "center x".to_string(),
                Value::Decimal(0.5),
                Some(InputSettings::Slider {
                    range: (0.0, 1.0),
                    step_by: Some(0.01),
                    clamp_to_range: false,
                }),
                None,
            )
            .with_description("Horizontal centre of the disc (0–1)."),
            Input::new(
                "center y".to_string(),
                Value::Decimal(0.5),
                Some(InputSettings::Slider {
                    range: (0.0, 1.0),
                    step_by: Some(0.01),
                    clamp_to_range: false,
                }),
                None,
            )
            .with_description("Vertical centre of the disc (0–1)."),
            Input::new(
                "radius".to_string(),
                Value::Decimal(0.35),
                Some(InputSettings::Slider {
                    range: (0.0, 2.0),
                    step_by: Some(0.01),
                    clamp_to_range: false,
                }),
                None,
            )
            .with_description(
                "Inner fully-selected radius as a fraction of half the shorter dimension.",
            ),
            Input::new(
                "softness".to_string(),
                Value::Decimal(0.25),
                Some(InputSettings::Slider {
                    range: (0.0, 2.0),
                    step_by: Some(0.01),
                    clamp_to_range: false,
                }),
                None,
            )
            .with_description("Falloff width beyond radius (same units as radius)."),
            Input::new(
                "aspect".to_string(),
                Value::Decimal(1.0),
                Some(InputSettings::Slider {
                    range: (0.1, 4.0),
                    step_by: Some(0.01),
                    clamp_to_range: false,
                }),
                None,
            )
            .with_description("Vertical scale of the distance metric; >1 stretches into a tall ellipse."),
            Input::new("invert".to_string(), Value::Bool(false), None, None)
                .with_description("Flip the mask so the outside is selected instead."),
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
        .with_description("Single-channel radial gradient mask.")]
    }

    /// Generates the radial gradient mask.
    pub async fn run(inputs: &mut [Input]) -> Result<OperationResponse, OperationError> {
        let start_time = Instant::now();
        let mut input_errors: Vec<(usize, String)> = vec![];

        let width_converted = convert_input(inputs, 0, ValueType::Integer, &mut input_errors);
        let height_converted = convert_input(inputs, 1, ValueType::Integer, &mut input_errors);
        let cx_converted = convert_input(inputs, 2, ValueType::Decimal, &mut input_errors);
        let cy_converted = convert_input(inputs, 3, ValueType::Decimal, &mut input_errors);
        let radius_converted = convert_input(inputs, 4, ValueType::Decimal, &mut input_errors);
        let softness_converted = convert_input(inputs, 5, ValueType::Decimal, &mut input_errors);
        let aspect_converted = convert_input(inputs, 6, ValueType::Decimal, &mut input_errors);
        let invert_converted = convert_input(inputs, 7, ValueType::Bool, &mut input_errors);

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
        let Value::Decimal(cx) = cx_converted.unwrap() else {
            unreachable!()
        };
        let Value::Decimal(cy) = cy_converted.unwrap() else {
            unreachable!()
        };
        let Value::Decimal(radius) = radius_converted.unwrap() else {
            unreachable!()
        };
        let Value::Decimal(softness) = softness_converted.unwrap() else {
            unreachable!()
        };
        let Value::Decimal(aspect) = aspect_converted.unwrap() else {
            unreachable!()
        };
        let Value::Bool(invert) = invert_converted.unwrap() else {
            unreachable!()
        };

        width = width.clamp(1, 10000);
        height = height.clamp(1, 10000);
        let radius = radius.max(0.0);
        let softness = softness.max(0.0);
        let aspect = aspect.max(1e-4);
        // Distance is measured in units of half the shorter side so radius 1
        // reaches the nearer edge on a square canvas.
        let scale = 0.5 * width.min(height) as f32;

        let e0 = radius;
        let e1 = radius + softness;

        let pixels: Vec<f32> = (0..height)
            .into_par_iter()
            .flat_map_iter(move |y| {
                let fy = (y as f32 + 0.5) / height as f32;
                let dy = (fy - cy) * height as f32 / scale / aspect;
                (0..width).map(move |x| {
                    let fx = (x as f32 + 0.5) / width as f32;
                    let dx = (fx - cx) * width as f32 / scale;
                    let d = (dx * dx + dy * dy).sqrt();
                    // 1 inside radius, fade to 0 over softness.
                    let mut m = 1.0 - smoothstep(e0, e1, d);
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
#[path = "radial_gradient_tests.rs"]
mod tests;
