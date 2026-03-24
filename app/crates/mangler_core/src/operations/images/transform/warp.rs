//! Displacement-map-based warp operation.
//!
//! Displaces pixels using a separate displacement map image. Uses
//! [`FloatImage::bilinear_sample`] for channel-agnostic interpolation.

use crate::get_id;
use crate::value::ValueType;
use crate::input::{Input, InputSettings};
use crate::node_settings::NodeSettings;
use crate::operations::{OperationResponse, OperationError, OutputResponse, default_image, convert_input};
use crate::output::Output;
use crate::value::Value;
use crate::float_image::FloatImage;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use std::time::Instant;

/// Displaces image pixels using a separate displacement map.
///
/// The first channel of the displacement map controls horizontal offset and the
/// second channel (if present) controls vertical offset. Values of 0.5 (mid-gray)
/// produce zero displacement; lower and higher values push pixels in opposite
/// directions. The intensity parameter scales the displacement magnitude.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpImageTransformWarp {}

impl OpImageTransformWarp {
    /// Returns the node metadata (name and description) for this operation.
    pub fn settings() -> NodeSettings {
        NodeSettings {
            name: "warp".to_string(),
            description: "Displaces pixels using a displacement map. Red channel offsets X, green channel offsets Y.".to_string(),
        }
    }

    /// Creates the default inputs: source image, displacement map, and intensity scalar.
    pub fn create_inputs() -> Vec<Input> {
        vec![
            Input::new("image".to_string(), Value::Image { data: default_image(), change_id: get_id() }, None, None),
            Input::new("displacement".to_string(), Value::Image { data: default_image(), change_id: get_id() }, None, None),
            Input::new("intensity".to_string(), Value::Decimal(10.0), Some(InputSettings::Slider { range: (0.0, 200.0), step_by: Some(0.1), clamp_to_range: false }), None),
        ]
    }

    /// Creates the default outputs: the warped image.
    pub fn create_outputs() -> Vec<Output> {
        vec![
            Output::new("output".to_string(), Value::Image { data: default_image(), change_id: get_id() }, None),
        ]
    }

    /// Executes the warp by sampling the displacement map for each output pixel.
    pub async fn run(inputs: &mut [Input]) -> Result<OperationResponse, OperationError> {
        let start_time = Instant::now();
        let mut input_errors: Vec<(usize, String)> = vec![];

        let image_converted = convert_input(inputs, 0, ValueType::Image, &mut input_errors);
        let disp_converted = convert_input(inputs, 1, ValueType::Image, &mut input_errors);
        let intensity_converted = convert_input(inputs, 2, ValueType::Decimal, &mut input_errors);

        if !input_errors.is_empty() { return Err(OperationError { input_errors, node_error: None }); }

        let Value::Image { data: src_data, change_id: _ } = image_converted.unwrap() else { unreachable!() };
        let Value::Image { data: disp_data, change_id: _ } = disp_converted.unwrap() else { unreachable!() };
        let Value::Decimal(intensity) = intensity_converted.unwrap() else { unreachable!() };

        let (w, h) = src_data.dimensions();
        // Output preserves the source image's channel count
        let mut output = FloatImage::new(w, h, src_data.channels());

        // Temporary buffers for bilinear sampling
        let disp_ch = disp_data.channels() as usize;
        let src_ch = src_data.channels() as usize;
        let mut dp = vec![0.0f32; disp_ch];
        let mut sp = vec![0.0f32; src_ch];

        for y in 0..h {
            for x in 0..w {
                // Sample displacement map, mapping output coords to displacement map coords
                // to handle mismatched dimensions between source and displacement
                let dx = x as f32 * disp_data.width() as f32 / w as f32;
                let dy = y as f32 * disp_data.height() as f32 / h as f32;
                disp_data.bilinear_sample(dx, dy, &mut dp);

                // Map 0.0..1.0 to -0.5..0.5, then multiply by intensity.
                // For 1-channel displacement, use the same value for both X and Y.
                let offset_x = (dp[0] - 0.5) * intensity;
                let offset_y = if disp_ch >= 2 { (dp[1] - 0.5) * intensity } else { offset_x };

                let sx = x as f32 + offset_x;
                let sy = y as f32 + offset_y;

                // Sample source image at displaced coordinates
                src_data.bilinear_sample(sx, sy, &mut sp);
                output.put_pixel(x, y, &sp);
            }
        }

        Ok(OperationResponse { ai_cost_usd: None,
            time: Instant::now().duration_since(start_time),
            responses: vec![
                OutputResponse { value: Value::Image { data: Arc::new(output), change_id: get_id() } },
            ],
        })
    }
}

#[cfg(test)]
#[path = "warp_tests.rs"]
mod tests;
