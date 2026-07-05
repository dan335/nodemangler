//! Average hue, saturation, and value of an image.
//!
//! Reports the dominant hue as a saturation-weighted circular mean, plus the
//! mean saturation and value. Averaging hue on the color wheel (rather than
//! as a plain number) avoids the wrap-around artifact where red near 0 and red
//! near 360 would otherwise average to cyan.

use crate::color::Color;
use crate::get_id;
use crate::input::Input;
use crate::node_settings::NodeSettings;
use crate::operations::{OperationResponse, OperationError, OutputResponse, default_image, convert_input};
use crate::output::Output;
use crate::value::{Value, ValueType};
use serde::{Deserialize, Serialize};
use std::f32::consts::PI;
use std::time::Instant;

/// Operation that reports an image's saturation-weighted mean hue, plus mean
/// saturation and value.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpNumberImageAverageHue {}

impl OpNumberImageAverageHue {
    /// Returns the node metadata (name, description, help).
    pub fn settings() -> NodeSettings {
        NodeSettings {
            name: "average hue".to_string(),
            description: "Saturation-weighted mean hue, plus mean saturation and value.".to_string(),
            help: "Converts every pixel to HSV and reports the average hue in degrees (0..360), the average saturation (0..1), and the average value (0..1).\n\nHue is averaged as a circular mean weighted by each pixel's saturation: colors are summed as vectors on the color wheel, so unsaturated (gray) pixels barely nudge the result and the answer never wraps incorrectly between red-at-0 and red-at-360. A fully gray image has no hue direction and reports 0.".to_string(),
        }
    }

    /// Creates the input port: a single image to measure.
    pub fn create_inputs() -> Vec<Input> {
        vec![
            Input::new("image".to_string(), Value::Image { data: default_image(), change_id: get_id() }, None, None)
                .with_description("Image whose average hue/saturation/value is measured."),
        ]
    }

    /// Creates the output ports: hue, saturation, value.
    pub fn create_outputs() -> Vec<Output> {
        vec![
            Output::new("hue".to_string(), Value::Decimal(0.0), None)
                .with_description("Saturation-weighted circular-mean hue in degrees (0..360)."),
            Output::new("saturation".to_string(), Value::Decimal(0.0), None)
                .with_description("Mean HSV saturation (0..1)."),
            Output::new("value".to_string(), Value::Decimal(0.0), None)
                .with_description("Mean HSV value/brightness (0..1)."),
        ]
    }

    /// Executes the average-hue computation.
    pub async fn run(inputs: &mut [Input]) -> Result<OperationResponse, OperationError> {
        let start_time = Instant::now();
        let mut input_errors: Vec<(usize, String)> = vec![];

        let image_converted = convert_input(inputs, 0, ValueType::Image, &mut input_errors);

        if !input_errors.is_empty() { return Err(OperationError { input_errors, node_error: None }); }

        let Value::Image { data, change_id: _ } = image_converted.unwrap() else { unreachable!() };

        let (mut sx, mut sy, mut ssum, mut vsum) = (0.0f64, 0.0f64, 0.0f64, 0.0f64);
        let mut count = 0f64;
        for px in data.pixels() {
            let (r, g, b, a) = super::pixel_rgba(px);
            let (hue, s, v, _) = Color::from_srgb_float(r, g, b, a).to_hsv();
            let rad = hue * PI / 180.0;
            sx += (s * rad.cos()) as f64;
            sy += (s * rad.sin()) as f64;
            ssum += s as f64;
            vsum += v as f64;
            count += 1.0;
        }

        if count == 0.0 {
            return Ok(OperationResponse {
                time: Instant::now().duration_since(start_time),
                responses: vec![
                    OutputResponse { value: Value::Decimal(0.0) },
                    OutputResponse { value: Value::Decimal(0.0) },
                    OutputResponse { value: Value::Decimal(0.0) },
                ],
            });
        }

        let hue = if sx.abs() < 1e-12 && sy.abs() < 1e-12 {
            0.0
        } else {
            let mut deg = sy.atan2(sx).to_degrees() as f32;
            if deg < 0.0 { deg += 360.0; }
            deg
        };
        let saturation = (ssum / count) as f32;
        let value = (vsum / count) as f32;

        Ok(OperationResponse {
            time: Instant::now().duration_since(start_time),
            responses: vec![
                OutputResponse { value: Value::Decimal(hue) },
                OutputResponse { value: Value::Decimal(saturation) },
                OutputResponse { value: Value::Decimal(value) },
            ],
        })
    }
}

#[cfg(test)]
#[path = "average_hue_tests.rs"]
mod tests;
