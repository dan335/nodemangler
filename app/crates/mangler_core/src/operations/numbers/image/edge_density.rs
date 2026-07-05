//! Edge density: the fraction of pixels that sit on an edge.
//!
//! Runs a Sobel operator over luminance and reports what fraction of the
//! interior pixels have a normalized gradient magnitude above a threshold.
//! Higher values mean busier, more detailed images.

use crate::get_id;
use crate::input::{Input, InputSettings};
use crate::node_settings::NodeSettings;
use crate::operations::{OperationResponse, OperationError, OutputResponse, default_image, convert_input};
use crate::output::Output;
use crate::value::{Value, ValueType};
use serde::{Deserialize, Serialize};
use std::time::Instant;

/// Operation that reports the fraction of pixels lying on an edge.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpNumberImageEdgeDensity {}

impl OpNumberImageEdgeDensity {
    /// Returns the node metadata (name, description, help).
    pub fn settings() -> NodeSettings {
        NodeSettings {
            name: "edge density".to_string(),
            description: "Fraction of pixels sitting on a Sobel edge above a threshold.".to_string(),
            help: "Runs a 3x3 Sobel operator on luminance at every interior pixel, then counts the fraction whose gradient magnitude exceeds the threshold. Near 0 means flat/smooth; near 1 means busy and highly detailed.\n\nThe magnitude is divided by 4 so that a full black-to-white step reads as roughly 1 — this /4 factor is a convenience heuristic, not a calibrated unit, so treat the number as relative. Border pixels are skipped and images smaller than 3x3 report 0.".to_string(),
        }
    }

    /// Creates the input ports: the image and an edge-magnitude threshold.
    pub fn create_inputs() -> Vec<Input> {
        vec![
            Input::new("image".to_string(), Value::Image { data: default_image(), change_id: get_id() }, None, None)
                .with_description("Image whose edge density is measured."),
            Input::new("threshold".to_string(), Value::Decimal(0.2), Some(InputSettings::Slider { range: (0.0, 1.0), step_by: None, clamp_to_range: true }), None)
                .with_description("A pixel counts as an edge when its normalized Sobel magnitude exceeds this."),
        ]
    }

    /// Creates the output port: the edge density fraction.
    pub fn create_outputs() -> Vec<Output> {
        vec![
            Output::new("density".to_string(), Value::Decimal(0.0), None)
                .with_description("Edge pixels divided by interior pixels (0..1)."),
        ]
    }

    /// Executes the edge-density computation.
    pub async fn run(inputs: &mut [Input]) -> Result<OperationResponse, OperationError> {
        let start_time = Instant::now();
        let mut input_errors: Vec<(usize, String)> = vec![];

        let image_converted = convert_input(inputs, 0, ValueType::Image, &mut input_errors);
        let threshold_converted = convert_input(inputs, 1, ValueType::Decimal, &mut input_errors);

        if !input_errors.is_empty() { return Err(OperationError { input_errors, node_error: None }); }

        let Value::Image { data, change_id: _ } = image_converted.unwrap() else { unreachable!() };
        let Value::Decimal(threshold) = threshold_converted.unwrap() else { unreachable!() };

        let (w, h) = data.dimensions();
        if w < 3 || h < 3 {
            return Ok(OperationResponse {
                time: Instant::now().duration_since(start_time),
                responses: vec![OutputResponse { value: Value::Decimal(0.0) }],
            });
        }

        let l = super::luma_values(&data);
        let wu = w as usize;
        let mut count = 0usize;
        for y in 1..h - 1 {
            for x in 1..w - 1 {
                let (xu, yu) = (x as usize, y as usize);
                let tl = (yu - 1) * wu + (xu - 1);
                let t = (yu - 1) * wu + xu;
                let tr = (yu - 1) * wu + (xu + 1);
                let left = yu * wu + (xu - 1);
                let r = yu * wu + (xu + 1);
                let bl = (yu + 1) * wu + (xu - 1);
                let b = (yu + 1) * wu + xu;
                let br = (yu + 1) * wu + (xu + 1);

                let gx = (l[tr] + 2.0 * l[r] + l[br]) - (l[tl] + 2.0 * l[left] + l[bl]);
                let gy = (l[bl] + 2.0 * l[b] + l[br]) - (l[tl] + 2.0 * l[t] + l[tr]);
                let mag = (gx * gx + gy * gy).sqrt() / 4.0;
                if mag > threshold { count += 1; }
            }
        }

        let interior = ((w - 2) * (h - 2)) as f32;
        let density = if interior > 0.0 { count as f32 / interior } else { 0.0 };

        Ok(OperationResponse {
            time: Instant::now().duration_since(start_time),
            responses: vec![OutputResponse { value: Value::Decimal(density) }],
        })
    }
}

#[cfg(test)]
#[path = "edge_density_tests.rs"]
mod tests;
