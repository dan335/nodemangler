//! Curvature detection from a normal map.
//!
//! Computes surface curvature by measuring the divergence of the normal field.
//! Output encodes curvature as a grayscale value: 0.5 = flat, >0.5 = convex, <0.5 = concave.

use crate::float_image::FloatImage;
use crate::get_id;
use crate::value::ValueType;
use rayon::prelude::*;
use crate::input::{Input, InputSettings};
use crate::node_settings::NodeSettings;
use crate::operations::{OperationResponse, OperationError, OutputResponse, default_image, convert_input};
use crate::output::Output;
use crate::value::Value;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use std::time::Instant;

/// Operation that detects surface curvature from a normal map.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpImagePbrCurvature {}

impl OpImagePbrCurvature {
    pub fn settings() -> NodeSettings {
        NodeSettings { name: "curvature".to_string(), description: "Detects convex and concave areas from a normal map.".to_string(), help: "Unpacks the X and Y components of the input tangent-space normal map, then measures their central-difference divergence per pixel (horizontal change of X plus vertical change of Y). That signed curvature is offset to a 0.5 midpoint and scaled by intensity to produce a greyscale RGBA image.\n\nInterpretation: 0.5 is flat, values above 0.5 are convex peaks/edges, values below are concave pits/valleys. Useful as a mask input for edge wear, dirt in crevices, and other PBR surface detailing. Expects a properly encoded (not inverted) OpenGL-style normal map.".to_string() }
    }
    pub fn create_inputs() -> Vec<Input> {
        vec![
            Input::new("image".to_string(), Value::Image { data: default_image(), change_id: get_id() }, None, None)
                .with_description("Tangent-space normal map used to compute curvature."),
            Input::new("intensity".to_string(), Value::Decimal(1.0), Some(InputSettings::Slider { range: (0.1, 10.0), step_by: Some(0.1), clamp_to_range: true }), None)
                .with_description("Scales the curvature signal away from the flat midpoint of 0.5."),
        ]
    }
    pub fn create_outputs() -> Vec<Output> {
        vec![Output::new("output".to_string(), Value::Image { data: default_image(), change_id: get_id() }, None)
                .with_description("Grayscale curvature map: 0.5 flat, >0.5 convex, <0.5 concave.")]
    }

    /// Computes curvature from the input normal map using divergence of the normal field.
    pub async fn run(inputs: &mut [Input]) -> Result<OperationResponse, OperationError> {
        let start_time = Instant::now();
        let mut input_errors: Vec<(usize, String)> = vec![];

        let image_converted = convert_input(inputs, 0, ValueType::Image, &mut input_errors);
        let intensity_converted = convert_input(inputs, 1, ValueType::Decimal, &mut input_errors);
        if !input_errors.is_empty() { return Err(OperationError { input_errors, node_error: None }); }

        let Value::Image { data, change_id: _ } = image_converted.unwrap() else { unreachable!() };
        let Value::Decimal(intensity) = intensity_converted.unwrap() else { unreachable!() };

        let width = data.width() as i32;
        let height = data.height() as i32;
        let img = &*data;

        let pixels: Vec<f32> = (0..height).into_par_iter().flat_map_iter(move |y| {
            let top_y = (y - 1).max(0) as u32;
            let bottom_y = (y + 1).min(height - 1) as u32;
            (0..width).flat_map(move |x| {
                let left_x = (x - 1).max(0) as u32;
                let right_x = (x + 1).min(width - 1) as u32;

                // Decode normal X/Y from the [0,1] encoded normal map
                let left_nx = img.get_pixel(left_x, y as u32)[0] * 2.0 - 1.0;
                let right_nx = img.get_pixel(right_x, y as u32)[0] * 2.0 - 1.0;
                let top_ny = img.get_pixel(x as u32, top_y)[1] * 2.0 - 1.0;
                let bottom_ny = img.get_pixel(x as u32, bottom_y)[1] * 2.0 - 1.0;

                // Divergence of normal field
                let dnx_dx = right_nx - left_nx;
                let dny_dy = bottom_ny - top_ny;
                let curvature_raw = (dnx_dx + dny_dy) * 0.5;

                let output = (0.5 + curvature_raw * intensity).clamp(0.0, 1.0);
                [output, output, output, 1.0]
            })
        }).collect();

        let buffer = FloatImage::from_raw(width as u32, height as u32, 4, pixels).unwrap();

        Ok(OperationResponse { 
            time: Instant::now().duration_since(start_time),
            responses: vec![OutputResponse { value: Value::Image { data: Arc::new(buffer), change_id: get_id() } }],
        })
    }
}

#[cfg(test)]
#[path = "curvature_tests.rs"]
mod tests;
