//! Curl (flow) noise generator.
//!
//! Takes the 2D curl of a value-noise scalar potential to produce a smooth,
//! divergence-free vector field. The field is written as an RGB flow map: red
//! and green encode the unit flow direction centred on 0.5 (so it plugs into
//! the warp node), and blue carries the flow magnitude. Tiles seamlessly.

use crate::float_image::FloatImage;
use crate::get_id;
use crate::input::{Input, InputSettings};
use crate::node_settings::NodeSettings;
use crate::operations::{OperationResponse, OperationError, OutputResponse, default_image, convert_input};
use crate::output::Output;
use crate::value::{Value, ValueType};
use rayon::prelude::*;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use std::time::Instant;
use super::{periodic_value_2d, build_perm_tables};

/// Operation that generates a seamlessly tiling curl-noise flow map.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpImageNoiseCurl {}

impl OpImageNoiseCurl {
    /// Returns the node metadata (name and description) for curl noise.
    pub fn settings() -> NodeSettings {
        NodeSettings {
            name: "curl noise".to_string(),
            description: "Creates a divergence-free flow field (curl of value noise) as an RGB flow map.".to_string(),
            help: "Builds a scalar potential from lattice-periodic value noise and takes its 2D curl — (∂φ/∂y, -∂φ/∂x) — which is divergence-free, so the resulting vectors swirl without sources or sinks. This is the classic 'flow noise' used to drive smoke, fur, and particle motion.\n\nThe vector field is encoded as an RGB image: red and green hold the unit flow direction remapped to [0,1] (0.5 = zero), matching what the warp node expects on its red/green channels, and blue holds the flow magnitude. Scale sets the lattice period; the field tiles seamlessly at that period. Direction channels are linear (not sRGB-encoded) so 0.5 stays exactly neutral.".to_string(),
        }
    }

    /// Creates the default inputs: seed, width, height, and scale.
    pub fn create_inputs() -> Vec<Input> {
        vec![
            Input::new("seed".to_string(), Value::Integer(1), Some(InputSettings::DragValue { clamp: None, speed: None }), None)
                .with_description("Random seed for the potential field; change to reshape the flow."),
            Input::new("width".to_string(), Value::Integer(512), Some(InputSettings::DragValue { clamp: Some((1.0, 10000.0)), speed: None }), None)
                .with_description("Output image width in pixels."),
            Input::new("height".to_string(), Value::Integer(512), Some(InputSettings::DragValue { clamp: Some((1.0, 10000.0)), speed: None }), None)
                .with_description("Output image height in pixels."),
            Input::new("scale".to_string(), Value::Integer(8), Some(InputSettings::DragValue { clamp: Some((1.0, 1000.0)), speed: None }), None)
                .with_description("Lattice period; higher values pack more, smaller swirls across the tile."),
        ]
    }

    /// Creates the default output: a 3-channel flow map image.
    pub fn create_outputs() -> Vec<Output> {
        vec![
            Output::new("output".to_string(), Value::Image { data: default_image(), change_id: get_id() }, None)
                .with_description("RGB flow map: R/G are the unit flow direction (0.5 = zero), B is magnitude."),
        ]
    }

    /// Generates the curl-noise flow map from the given inputs.
    pub async fn run(inputs: &mut [Input]) -> Result<OperationResponse, OperationError> {
        let start_time = Instant::now();
        let mut input_errors: Vec<(usize, String)> = vec![];

        let seed_converted = convert_input(inputs, 0, ValueType::Integer, &mut input_errors);
        let width_converted = convert_input(inputs, 1, ValueType::Integer, &mut input_errors);
        let height_converted = convert_input(inputs, 2, ValueType::Integer, &mut input_errors);
        let scale_converted = convert_input(inputs, 3, ValueType::Integer, &mut input_errors);

        if !input_errors.is_empty() { return Err(OperationError { input_errors, node_error: None }); }

        let Value::Integer(mut seed) = seed_converted.unwrap() else { unreachable!() };
        let Value::Integer(mut width) = width_converted.unwrap() else { unreachable!() };
        let Value::Integer(mut height) = height_converted.unwrap() else { unreachable!() };
        let Value::Integer(scale) = scale_converted.unwrap() else { unreachable!() };

        width = width.max(1);
        height = height.max(1);
        seed = seed.max(1);
        let period = scale.max(1) as isize;
        let perm = &build_perm_tables(seed as u32, 1)[0];

        let w = width as u32;
        let h = height as u32;
        // Finite-difference step in lattice units (a fraction of a cell).
        let eps = 1e-2_f64;

        let pixels: Vec<f32> = (0..h).into_par_iter().flat_map_iter(move |y| {
            let v = y as f64 / h as f64 * period as f64;
            (0..w).flat_map(move |x| {
                let u = x as f64 / w as f64 * period as f64;
                // Curl of the scalar potential φ: (∂φ/∂y, -∂φ/∂x).
                let dphidx = (periodic_value_2d(u + eps, v, period, period, perm)
                    - periodic_value_2d(u - eps, v, period, period, perm))
                    / (2.0 * eps);
                let dphidy = (periodic_value_2d(u, v + eps, period, period, perm)
                    - periodic_value_2d(u, v - eps, period, period, perm))
                    / (2.0 * eps);
                let vx = dphidy as f32;
                let vy = -dphidx as f32;
                let mag = (vx * vx + vy * vy).sqrt();
                let (dx, dy) = if mag > 1e-6 { (vx / mag, vy / mag) } else { (0.0, 0.0) };
                [
                    0.5 + 0.5 * dx,
                    0.5 + 0.5 * dy,
                    mag.min(1.0),
                ]
            })
        }).collect();

        let img = FloatImage::from_raw(w, h, 3, pixels).unwrap();

        Ok(OperationResponse {
            time: Instant::now().duration_since(start_time),
            responses: vec![OutputResponse { value: Value::Image { data: Arc::new(img), change_id: get_id() } }],
        })
    }
}

#[cfg(test)]
#[path = "curl_tests.rs"]
mod tests;
