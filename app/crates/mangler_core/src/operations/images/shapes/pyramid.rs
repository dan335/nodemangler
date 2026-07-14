//! Pyramid height shape: peaks at the centre, linearly falls off along the
//! max-axis distance (Chebyshev distance). Square footprint with optional
//! step banding.

use crate::float_image::FloatImage;
use crate::get_id;
use crate::input::{Input, InputSettings};
use crate::node_settings::NodeSettings;
use crate::operations::images::tone_curve::{anti_diagonal_tone_curve, sample_lut, tone_curve_lut, TONE_LUT_SIZE};
use crate::operations::{OperationResponse, OperationError, OutputResponse, default_image, convert_input};
use crate::output::Output;
use crate::value::{Value, ValueType};
use rayon::prelude::*;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use std::time::Instant;

/// Square-based pyramid height shape.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpImageShapePyramid {}

impl OpImageShapePyramid {
    pub fn settings() -> NodeSettings {
        NodeSettings {
            name: "pyramid".to_string(),
            description: "Square pyramid height shape. Optional step count produces a banded, Mayan-style stepped pyramid.".to_string(),
            help: "Emits a 1-channel greyscale height shape whose value falls off linearly with the Chebyshev (max-axis) distance from the centre, giving a pyramid with a square footprint. size is the half-extent of the base; pixels outside it clamp to 0 and the apex sits at 1.0.\n\nSetting steps above 0 quantises the continuous slope into that many bands, producing a stepped, Mayan-style pyramid. Rotation is applied in sample space so the pyramid rotates in place around its peak. Output is linear, ready for normal or AO generation.\n\nprofile is a tone curve mapping normalised Chebyshev distance from the apex (x = 0) to the base edge (x = 1) onto height, applied before step quantisation; the drawn curve reads left-to-right as the pyramid's silhouette from peak to rim. The default is a straight descending ramp, reproducing the plain linear pyramid. Regardless of the curve, height is hard 0 past the base edge — a curve whose value at x = 1 is above 0 creates a visible cliff at the edge instead of a smooth taper.".to_string(),
        }
    }

    pub fn create_inputs() -> Vec<Input> {
        vec![
            Input::new("width".to_string(), Value::Integer(512), Some(InputSettings::DragValue { clamp: Some((1.0, 10000.0)), speed: None }), None)
                .with_description("Width of the generated height map in pixels."),
            Input::new("height".to_string(), Value::Integer(512), Some(InputSettings::DragValue { clamp: Some((1.0, 10000.0)), speed: None }), None)
                .with_description("Height of the generated height map in pixels."),
            Input::new("size".to_string(), Value::Decimal(0.5), Some(InputSettings::Slider { range: (0.01, 1.0), step_by: None, clamp_to_range: false }), None)
                .with_description("Half-size of the square pyramid base in normalised units."),
            Input::new("steps".to_string(), Value::Integer(0), Some(InputSettings::DragValue { clamp: Some((0.0, 32.0)), speed: None }), None)
                .with_description("Number of stepped bands; 0 produces a smooth slope."),
            Input::new("rotation".to_string(), Value::Decimal(0.0), Some(InputSettings::Slider { range: (0.0, 360.0), step_by: None, clamp_to_range: false }), None)
                .with_description("Rotation of the pyramid around its center in degrees."),
            Input::new("profile".to_string(), Value::Curve(anti_diagonal_tone_curve()), Some(InputSettings::ToneCurve), None)
                .with_description("Height profile from apex (x = 0) to base edge (x = 1); default is a straight linear falloff."),
        ]
    }

    pub fn create_outputs() -> Vec<Output> {
        vec![
            Output::new("output".to_string(), Value::Image { data: default_image(), change_id: get_id() }, None)
                .with_description("Grayscale pyramid height shape, 1.0 at the peak and 0.0 outside the base."),
        ]
    }

    pub async fn run(inputs: &mut [Input]) -> Result<OperationResponse, OperationError> {
        let start_time = Instant::now();
        let mut input_errors: Vec<(usize, String)> = vec![];

        let w_c = convert_input(inputs, 0, ValueType::Integer, &mut input_errors);
        let h_c = convert_input(inputs, 1, ValueType::Integer, &mut input_errors);
        let size_c = convert_input(inputs, 2, ValueType::Decimal, &mut input_errors);
        let steps_c = convert_input(inputs, 3, ValueType::Integer, &mut input_errors);
        let rot_c = convert_input(inputs, 4, ValueType::Decimal, &mut input_errors);
        let profile_c = convert_input(inputs, 5, ValueType::Curve, &mut input_errors);

        if !input_errors.is_empty() { return Err(OperationError { input_errors, node_error: None }); }

        let Value::Integer(mut width) = w_c.unwrap() else { unreachable!() };
        let Value::Integer(mut height) = h_c.unwrap() else { unreachable!() };
        let Value::Decimal(size) = size_c.unwrap() else { unreachable!() };
        let Value::Integer(steps) = steps_c.unwrap() else { unreachable!() };
        let Value::Decimal(rotation) = rot_c.unwrap() else { unreachable!() };
        let Value::Curve(profile) = profile_c.unwrap() else { unreachable!() };

        width = width.max(1);
        height = height.max(1);
        let size = (size as f64).max(0.001);
        let angle = (rotation as f64).to_radians();
        let cos_a = angle.cos();
        let sin_a = angle.sin();
        let lut = tone_curve_lut(&profile, TONE_LUT_SIZE);
        let lut = &lut;

        let pixels: Vec<f32> = (0..height).into_par_iter().flat_map_iter(move |y| {
            let ny = (y as f64 / (height as f64 - 1.0).max(1.0)) * 2.0 - 1.0;
            (0..width).map(move |x| {
                let nx = (x as f64 / (width as f64 - 1.0).max(1.0)) * 2.0 - 1.0;
                let rx = nx * cos_a + ny * sin_a;
                let ry = -nx * sin_a + ny * cos_a;
                let d = rx.abs().max(ry.abs()) / size;
                let mut h = if d >= 1.0 { 0.0 } else { sample_lut(lut, d as f32) as f64 };
                if steps > 0 && h > 0.0 {
                    // Quantise to `steps` bands.
                    let s = steps as f64;
                    h = (h * s).ceil() / s;
                }
                h as f32
            })
        }).collect();

        let img = FloatImage::from_raw(width as u32, height as u32, 1, pixels).unwrap();

        Ok(OperationResponse {
            time: Instant::now().duration_since(start_time),
            responses: vec![
                OutputResponse { value: Value::Image { data: Arc::new(img), change_id: get_id() } },
            ],
        })
    }
}

#[cfg(test)]
#[path = "pyramid_tests.rs"]
mod tests;
