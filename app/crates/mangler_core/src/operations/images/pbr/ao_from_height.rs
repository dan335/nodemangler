//! Ambient occlusion generation from a height map.
//!
//! Approximates ambient occlusion by sampling height differences at evenly
//! spaced angles around each pixel. Higher neighboring surfaces contribute
//! more occlusion, producing darker values in concavities and crevices.

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

/// Operation that computes ambient occlusion from a grayscale height map.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpImagePbrAoFromHeight {}

impl OpImagePbrAoFromHeight {
    pub fn settings() -> NodeSettings {
        NodeSettings { name: "ao from height".to_string(), description: "Computes ambient occlusion from a height map.".to_string(), help: "Approximates ambient occlusion by sampling the height at the given number of angles around each pixel within a search radius in pixels. Positive neighbor-minus-center height differences divided by distance are averaged; the result subtracts from 1.0 to produce a greyscale RGBA AO image where crevices are dark and high points are bright.\n\nThe source is read as linear height: if the input has three or more channels it is Rec.709-luminance weighted. intensity scales the darkening, samples trades quality for speed, and the whole pass runs in parallel via rayon.".to_string() }
    }

    pub fn create_inputs() -> Vec<Input> {
        vec![
            Input::new("image".to_string(), Value::Image { data: default_image(), change_id: get_id() }, None, None)
                .with_description("Grayscale height map used as the source surface."),
            Input::new("radius".to_string(), Value::Integer(8), Some(InputSettings::DragValue { speed: None, clamp: Some((1.0, 64.0)) }), None)
                .with_description("Sampling radius in pixels that controls the scale of the occlusion."),
            Input::new("intensity".to_string(), Value::Decimal(1.0), Some(InputSettings::Slider { range: (0.1, 10.0), step_by: Some(0.1), clamp_to_range: true }), None)
                .with_description("Strength of the occlusion darkening applied in concavities."),
            Input::new("samples".to_string(), Value::Integer(16), Some(InputSettings::DragValue { speed: None, clamp: Some((4.0, 64.0)) }), None)
                .with_description("Number of radial samples taken per pixel; higher values are smoother but slower."),
        ]
    }

    pub fn create_outputs() -> Vec<Output> {
        vec![Output::new("output".to_string(), Value::Image { data: default_image(), change_id: get_id() }, None)
                .with_description("Grayscale ambient-occlusion map where dark areas are occluded.")]
    }

    /// Computes ambient occlusion from the input height map by radial sampling.
    pub async fn run(inputs: &mut [Input]) -> Result<OperationResponse, OperationError> {
        let start_time = Instant::now();
        let mut input_errors: Vec<(usize, String)> = vec![];

        let image_converted = convert_input(inputs, 0, ValueType::Image, &mut input_errors);
        let radius_converted = convert_input(inputs, 1, ValueType::Integer, &mut input_errors);
        let intensity_converted = convert_input(inputs, 2, ValueType::Decimal, &mut input_errors);
        let samples_converted = convert_input(inputs, 3, ValueType::Integer, &mut input_errors);

        if !input_errors.is_empty() { return Err(OperationError { input_errors, node_error: None }); }

        let Value::Image { data, change_id: _ } = image_converted.unwrap() else { unreachable!() };
        let Value::Integer(radius) = radius_converted.unwrap() else { unreachable!() };
        let Value::Decimal(intensity) = intensity_converted.unwrap() else { unreachable!() };
        let Value::Integer(samples) = samples_converted.unwrap() else { unreachable!() };

        let width = data.width() as usize;
        let height = data.height() as usize;
        let ch = data.channels() as usize;
        let radius = (radius as i64).clamp(1, 64) as usize;
        let samples = (samples as i64).clamp(4, 64) as usize;

        // Extract luminance as height values
        let mut heights: Vec<f32> = Vec::with_capacity(width * height);
        for pixel in data.pixels() {
            let h = if ch >= 3 { 0.2126 * pixel[0] + 0.7152 * pixel[1] + 0.0722 * pixel[2] } else { pixel[0] };
            heights.push(h);
        }

        let two_pi = std::f32::consts::TAU;
        let heights_ref = &heights;

        // The sample ring is identical for every pixel: precompute each
        // sample's offset and its interior distance (from the rounded offset)
        // once instead of re-deriving cos/sin/sqrt per pixel.
        let ring: Vec<(f32, f32, f32)> = (0..samples).map(|i| {
            let angle = i as f32 * two_pi / samples as f32;
            let ddx = angle.cos() * radius as f32;
            let ddy = angle.sin() * radius as f32;
            let rdx = ddx.round();
            let rdy = ddy.round();
            let dist = (rdx * rdx + rdy * rdy).sqrt().max(1.0);
            (ddx, ddy, dist)
        }).collect();
        let ring_ref = &ring;
        let wm1 = (width - 1) as f32;
        let hm1 = (height - 1) as f32;

        let pixels: Vec<f32> = (0..height).into_par_iter().flat_map_iter(move |y| {
            (0..width).flat_map(move |x| {
                let h = heights_ref[y * width + x];
                let mut occlusion = 0.0f32;
                for &(ddx, ddy, ring_dist) in ring_ref {
                    let sxf = (x as f32 + ddx).round();
                    let syf = (y as f32 + ddy).round();
                    let (sx, sy, dist) = if sxf >= 0.0 && sxf <= wm1 && syf >= 0.0 && syf <= hm1 {
                        (sxf as usize, syf as usize, ring_dist)
                    } else {
                        // Clamped at the border: the effective offset shrank,
                        // so the precomputed ring distance no longer applies.
                        let sx = sxf.clamp(0.0, wm1) as usize;
                        let sy = syf.clamp(0.0, hm1) as usize;
                        let dist = ((sx as f32 - x as f32).powi(2) + (sy as f32 - y as f32).powi(2)).sqrt().max(1.0);
                        (sx, sy, dist)
                    };
                    let nh = heights_ref[sy * width + sx];
                    occlusion += (nh - h).max(0.0) / dist;
                }
                occlusion /= samples as f32;
                let ao = (1.0 - occlusion * intensity).clamp(0.0, 1.0);
                [ao, ao, ao, 1.0]
            })
        }).collect();

        let output = FloatImage::from_raw(width as u32, height as u32, 4, pixels).unwrap();

        Ok(OperationResponse { 
            time: Instant::now().duration_since(start_time),
            responses: vec![OutputResponse { value: Value::Image { data: Arc::new(output), change_id: get_id() } }],
        })
    }
}

#[cfg(test)]
#[path = "ao_from_height_tests.rs"]
mod tests;
