//! Histogram select — extracts a luminance band as a soft-edged mask.
//!
//! Given a target position and range on the luminance axis, the output is
//! brightest where the input's luminance is at `position` and fades to black
//! at the edges of the band. `contrast` controls how sharp that fade is: 0
//! gives a fully soft transition, 1 gives a hard rectangular mask.

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

/// Histogram select: isolate a luminance band as a mask.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpImageAdjustmentHistogramSelect {}

impl OpImageAdjustmentHistogramSelect {
    pub fn settings() -> NodeSettings {
        NodeSettings {
            name: "histogram select".to_string(),
            description: "Outputs a soft-edged mask where input luminance falls within a chosen band.".to_string(),
        }
    }

    pub fn create_inputs() -> Vec<Input> {
        vec![
            Input::new("image".to_string(), Value::Image { data: default_image(), change_id: get_id() }, None, None),
            Input::new("position".to_string(), Value::Decimal(0.5), Some(InputSettings::Slider { range: (0.0, 1.0), step_by: Some(0.01), clamp_to_range: true }), None),
            Input::new("range".to_string(), Value::Decimal(0.2), Some(InputSettings::Slider { range: (0.0, 1.0), step_by: Some(0.01), clamp_to_range: true }), None),
            Input::new("contrast".to_string(), Value::Decimal(0.0), Some(InputSettings::Slider { range: (0.0, 1.0), step_by: Some(0.01), clamp_to_range: true }), None),
        ]
    }

    pub fn create_outputs() -> Vec<Output> {
        vec![
            Output::new("output".to_string(), Value::Image { data: default_image(), change_id: get_id() }, None),
        ]
    }

    pub async fn run(inputs: &mut [Input]) -> Result<OperationResponse, OperationError> {
        let start_time = Instant::now();
        let mut input_errors: Vec<(usize, String)> = vec![];

        let image_converted = convert_input(inputs, 0, ValueType::Image, &mut input_errors);
        let position_converted = convert_input(inputs, 1, ValueType::Decimal, &mut input_errors);
        let range_converted = convert_input(inputs, 2, ValueType::Decimal, &mut input_errors);
        let contrast_converted = convert_input(inputs, 3, ValueType::Decimal, &mut input_errors);

        if !input_errors.is_empty() { return Err(OperationError { input_errors, node_error: None }); }

        let Value::Image { data, change_id: _ } = image_converted.unwrap() else { unreachable!() };
        let Value::Decimal(position) = position_converted.unwrap() else { unreachable!() };
        let Value::Decimal(range) = range_converted.unwrap() else { unreachable!() };
        let Value::Decimal(contrast) = contrast_converted.unwrap() else { unreachable!() };

        let half_range = (range * 0.5).max(1e-6);
        let contrast = contrast.clamp(0.0, 1.0);
        // `contrast` widens the fully-opaque interior of the band:
        // - 0 → soft gradient across the full half-range (triangular mask)
        // - 1 → rectangular mask (entire band is 1.0, nothing in between)
        let soft_edge = half_range * contrast;
        let hard_edge = half_range;

        let (width, height) = data.dimensions();
        let ch = data.channels() as usize;
        let color_ch = if ch == 2 || ch == 4 { ch - 1 } else { ch };

        let mut output = FloatImage::new(width, height, 1);
        for y in 0..height {
            for x in 0..width {
                let p = data.get_pixel(x, y);
                let lum = if color_ch >= 3 {
                    0.2126 * p[0] + 0.7152 * p[1] + 0.0722 * p[2]
                } else {
                    p[0]
                };
                let d = (lum - position).abs();
                let mask = if d <= soft_edge {
                    1.0
                } else if d >= hard_edge {
                    0.0
                } else {
                    // Smoothstep from 1 at soft_edge to 0 at hard_edge. The
                    // gap is strictly positive here: d > soft_edge already
                    // implies hard_edge > soft_edge.
                    let t = ((d - soft_edge) / (hard_edge - soft_edge)).clamp(0.0, 1.0);
                    1.0 - (t * t * (3.0 - 2.0 * t))
                };
                output.put_pixel(x, y, &[mask]);
            }
        }

        Ok(OperationResponse {
            time: Instant::now().duration_since(start_time),
            responses: vec![
                OutputResponse { value: Value::Image { data: Arc::new(output), change_id: get_id() } },
            ],
        })
    }
}

#[cfg(test)]
#[path = "histogram_select_tests.rs"]
mod tests;
