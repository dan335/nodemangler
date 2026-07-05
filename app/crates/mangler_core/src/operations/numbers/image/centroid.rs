//! Luminance-weighted centroid (center of mass) of an image.
//!
//! Treats each pixel's luminance as a mass and reports where the balance point
//! sits — both in pixel coordinates and normalized to 0..1 across each axis.
//! A uniform or empty image falls back to the geometric center.

use crate::get_id;
use crate::input::Input;
use crate::node_settings::NodeSettings;
use crate::operations::{OperationResponse, OperationError, OutputResponse, default_image, convert_input};
use crate::output::Output;
use crate::value::{Value, ValueType};
use serde::{Deserialize, Serialize};
use std::time::Instant;

/// Operation that computes the luminance-weighted centroid of an image.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpNumberImageCentroid {}

impl OpNumberImageCentroid {
    /// Returns the node metadata (name, description, help).
    pub fn settings() -> NodeSettings {
        NodeSettings {
            name: "centroid".to_string(),
            description: "Finds the luminance-weighted center of mass of an image.".to_string(),
            help: "Weights every pixel by its Rec. 601 luminance and reports the balance point: bright regions pull the centroid toward themselves. Outputs the position both in pixels (x, y) and normalized to 0..1 (x normalized: 0 = left, 1 = right; y normalized: 0 = top, 1 = bottom).\n\nWhen the image carries no luminance at all (fully black), the centroid falls back to the geometric center. Use it to auto-center a shape or to steer effects toward where the brightness is.".to_string(),
        }
    }

    /// Creates the input port: a single image to measure.
    pub fn create_inputs() -> Vec<Input> {
        vec![
            Input::new("image".to_string(), Value::Image { data: default_image(), change_id: get_id() }, None, None)
                .with_description("Image whose center of mass is measured."),
        ]
    }

    /// Creates the output ports: centroid in pixels and normalized.
    pub fn create_outputs() -> Vec<Output> {
        vec![
            Output::new("x".to_string(), Value::Decimal(0.0), None)
                .with_description("Centroid x in pixels."),
            Output::new("y".to_string(), Value::Decimal(0.0), None)
                .with_description("Centroid y in pixels."),
            Output::new("x normalized".to_string(), Value::Decimal(0.0), None)
                .with_description("Centroid x as 0..1 (0 = left, 1 = right)."),
            Output::new("y normalized".to_string(), Value::Decimal(0.0), None)
                .with_description("Centroid y as 0..1 (0 = top, 1 = bottom)."),
        ]
    }

    /// Executes the centroid computation.
    pub async fn run(inputs: &mut [Input]) -> Result<OperationResponse, OperationError> {
        let start_time = Instant::now();
        let mut input_errors: Vec<(usize, String)> = vec![];

        let image_converted = convert_input(inputs, 0, ValueType::Image, &mut input_errors);

        if !input_errors.is_empty() { return Err(OperationError { input_errors, node_error: None }); }

        let Value::Image { data, change_id: _ } = image_converted.unwrap() else { unreachable!() };

        let (w, h) = data.dimensions();
        let (mut sumw, mut sumx, mut sumy) = (0.0f64, 0.0f64, 0.0f64);
        for y in 0..h {
            for x in 0..w {
                let wgt = super::pixel_luma(data.get_pixel(x, y)) as f64;
                sumw += wgt;
                sumx += x as f64 * wgt;
                sumy += y as f64 * wgt;
            }
        }

        let (cx, cy) = if sumw.abs() < 1e-12 {
            ((w.max(1) - 1) as f64 / 2.0, (h.max(1) - 1) as f64 / 2.0)
        } else {
            (sumx / sumw, sumy / sumw)
        };

        let xn = if w > 1 { cx / ((w - 1) as f64) } else { 0.0 };
        let yn = if h > 1 { cy / ((h - 1) as f64) } else { 0.0 };

        Ok(OperationResponse {
            time: Instant::now().duration_since(start_time),
            responses: vec![
                OutputResponse { value: Value::Decimal(cx as f32) },
                OutputResponse { value: Value::Decimal(cy as f32) },
                OutputResponse { value: Value::Decimal(xn as f32) },
                OutputResponse { value: Value::Decimal(yn as f32) },
            ],
        })
    }
}

#[cfg(test)]
#[path = "centroid_tests.rs"]
mod tests;
