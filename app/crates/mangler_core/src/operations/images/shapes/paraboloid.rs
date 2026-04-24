//! Paraboloid height shape: `h = 1 - (d / size)^2` inside, 0 outside.
//!
//! A smooth, rounded dome. Useful as a height source for bumps, lens-shaped
//! reflections, or as input to normal/AO nodes.

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

/// Paraboloid (smooth dome) height shape.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpImageShapeParaboloid {}

impl OpImageShapeParaboloid {
    pub fn settings() -> NodeSettings {
        NodeSettings {
            name: "paraboloid".to_string(),
            description: "Paraboloid dome height shape centered in the image. h = 1 - (d/size)^2.".to_string(),
            help: "Generates a smooth 1-channel greyscale dome whose height falls off with the power of the normalised radial distance. size sets the radius in normalised units; falloff is the exponent applied to distance, where 2.0 gives a true paraboloid and higher values produce a flatter plateau with steeper shoulders.\n\nValues sit in linear space and peak at 1.0, making this a clean source for normal_from_height, AO, or blob-style masks. Pixels outside the radius are clamped to 0.".to_string(),
        }
    }

    pub fn create_inputs() -> Vec<Input> {
        vec![
            Input::new("width".to_string(), Value::Integer(512), Some(InputSettings::DragValue { clamp: Some((1.0, 10000.0)), speed: None }), None)
                .with_description("Width of the generated height map in pixels."),
            Input::new("height".to_string(), Value::Integer(512), Some(InputSettings::DragValue { clamp: Some((1.0, 10000.0)), speed: None }), None)
                .with_description("Height of the generated height map in pixels."),
            Input::new("size".to_string(), Value::Decimal(0.5), Some(InputSettings::Slider { range: (0.01, 1.0), step_by: None, clamp_to_range: false }), None)
                .with_description("Radius of the paraboloid dome in normalised units."),
            Input::new("falloff".to_string(), Value::Decimal(2.0), Some(InputSettings::Slider { range: (0.5, 6.0), step_by: Some(0.1), clamp_to_range: false }), None)
                .with_description("Power controlling the sharpness of the dome; 2.0 is standard."),
        ]
    }

    pub fn create_outputs() -> Vec<Output> {
        vec![
            Output::new("output".to_string(), Value::Image { data: default_image(), change_id: get_id() }, None)
                .with_description("Grayscale dome height shape, 1.0 at the center and 0.0 outside."),
        ]
    }

    pub async fn run(inputs: &mut [Input]) -> Result<OperationResponse, OperationError> {
        let start_time = Instant::now();
        let mut input_errors: Vec<(usize, String)> = vec![];

        let w_c = convert_input(inputs, 0, ValueType::Integer, &mut input_errors);
        let h_c = convert_input(inputs, 1, ValueType::Integer, &mut input_errors);
        let size_c = convert_input(inputs, 2, ValueType::Decimal, &mut input_errors);
        let falloff_c = convert_input(inputs, 3, ValueType::Decimal, &mut input_errors);

        if !input_errors.is_empty() { return Err(OperationError { input_errors, node_error: None }); }

        let Value::Integer(mut width) = w_c.unwrap() else { unreachable!() };
        let Value::Integer(mut height) = h_c.unwrap() else { unreachable!() };
        let Value::Decimal(size) = size_c.unwrap() else { unreachable!() };
        let Value::Decimal(falloff) = falloff_c.unwrap() else { unreachable!() };

        width = width.max(1);
        height = height.max(1);
        let size = (size as f64).max(0.001);
        let falloff = (falloff as f64).max(0.1);

        let mut img = FloatImage::new(width as u32, height as u32, 1);
        for y in 0..height {
            for x in 0..width {
                // Normalise to [-1, 1] square. Distance measured from centre.
                let nx = (x as f64 / (width as f64 - 1.0).max(1.0)) * 2.0 - 1.0;
                let ny = (y as f64 / (height as f64 - 1.0).max(1.0)) * 2.0 - 1.0;
                let d = (nx * nx + ny * ny).sqrt() / size;
                let h = if d >= 1.0 { 0.0 } else {
                    // falloff controls sharpness — 2.0 is the standard paraboloid.
                    (1.0 - d.powf(falloff)).max(0.0)
                };
                img.put_pixel(x as u32, y as u32, &[h as f32]);
            }
        }

        Ok(OperationResponse {
            time: Instant::now().duration_since(start_time),
            responses: vec![
                OutputResponse { value: Value::Image { data: Arc::new(img), change_id: get_id() } },
            ],
        })
    }
}

#[cfg(test)]
#[path = "paraboloid_tests.rs"]
mod tests;
