//! Emboss effect operation for images.
//!
//! Creates a 3D-relief emboss effect by computing the difference between
//! pixels sampled along a configurable angle direction. The result is
//! centered around mid-grey (0.5), with raised/lowered areas appearing
//! lighter/darker.

use crate::get_id;
use crate::value::ValueType;
use crate::input::{Input, InputSettings};
use crate::node_settings::NodeSettings;
use crate::operations::{OperationResponse, OperationError, OutputResponse, default_image, convert_input};
use crate::output::Output;
use crate::value::Value;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use std::time::Instant;

/// Emboss operation that creates a 3D-relief effect using directional pixel differences.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpImageAdjustmentEmboss {}

impl OpImageAdjustmentEmboss {
    /// Returns the node metadata (name and description) for the emboss operation.
    pub fn settings() -> NodeSettings {
        NodeSettings {
            name: "emboss".to_string(),
            description: "Applies an emboss effect.".to_string(),
            help: "For each pixel samples one step forward and one step backward along the light angle, then outputs `0.5 + intensity * (forward - backward)` clamped to [0, 1]. Flat regions render as mid-grey; rising and falling edges shade light and dark relative to the angle.\n\nThe angle is the direction from which the simulated light arrives; rotating it flips the apparent relief. Color channels are processed independently; alpha is preserved. Edges are handled by clamping.".to_string(),
        }
    }

    /// Creates the input ports: image, intensity, and angle (in degrees) controlling the emboss direction.
    pub fn create_inputs() -> Vec<Input> {
        vec![
            Input::new("image".to_string(), Value::Image { data: default_image(), change_id: get_id() }, None, None)
                .with_description("Source image to convert into a 3D-relief emboss."),
            Input::new("intensity".to_string(), Value::Decimal(1.0), Some(InputSettings::Slider { range: (0.0, 10.0), step_by: Some(0.1), clamp_to_range: true }), None)
                .with_description("Multiplier on the directional difference; higher values deepen the relief."),
            Input::new("angle".to_string(), Value::Decimal(135.0), Some(InputSettings::Slider { range: (0.0, 360.0), step_by: Some(1.0), clamp_to_range: true }), None)
                .with_description("Light direction in degrees controlling where the relief appears to come from."),
        ]
    }

    /// Creates the output port: the embossed image.
    pub fn create_outputs() -> Vec<Output> {
        vec![Output::new("output".to_string(), Value::Image { data: default_image(), change_id: get_id() }, None)
            .with_description("Embossed image centered at mid-grey with relief along the chosen angle.")]
    }

    /// Executes the emboss effect. Samples forward and backward along the angle direction
    /// and outputs the scaled difference centered at 0.5.
    pub async fn run(inputs: &mut [Input]) -> Result<OperationResponse, OperationError> {
        let start_time = Instant::now();
        let mut input_errors: Vec<(usize, String)> = vec![];

        let image_converted = convert_input(inputs, 0, ValueType::Image, &mut input_errors);
        let intensity_converted = convert_input(inputs, 1, ValueType::Decimal, &mut input_errors);
        let angle_converted = convert_input(inputs, 2, ValueType::Decimal, &mut input_errors);

        if !input_errors.is_empty() { return Err(OperationError { input_errors, node_error: None }); }

        let Value::Image { data, change_id: _ } = image_converted.unwrap() else { unreachable!() };
        let Value::Decimal(intensity) = intensity_converted.unwrap() else { unreachable!() };
        let Value::Decimal(angle) = angle_converted.unwrap() else { unreachable!() };

        // run node — work directly on FloatImage
        let (width, height) = (data.width(), data.height());
        let mut output = (*data).clone();
        let ch = data.channels() as usize;
        let color_ch = if ch == 2 || ch == 4 { ch - 1 } else { ch };
        let angle_rad = angle.to_radians();
        let dx = angle_rad.cos();
        let dy = angle_rad.sin();

        for y in 0..height {
            for x in 0..width {
                let fx = x as f32;
                let fy = y as f32;

                let pixel = output.get_pixel_mut(x, y);
                for (c, val) in pixel.iter_mut().enumerate().take(color_ch) {
                    // Sample forward pixel
                    let fpx = (fx + dx).round().clamp(0.0, (width - 1) as f32) as u32;
                    let fpy = (fy + dy).round().clamp(0.0, (height - 1) as f32) as u32;
                    let forward = data.get_pixel(fpx, fpy)[c];
                    // Sample backward pixel
                    let bpx = (fx - dx).round().clamp(0.0, (width - 1) as f32) as u32;
                    let bpy = (fy - dy).round().clamp(0.0, (height - 1) as f32) as u32;
                    let backward = data.get_pixel(bpx, bpy)[c];
                    *val = (0.5 + intensity * (forward - backward)).clamp(0.0, 1.0);
                }
                // alpha unchanged
            }
        }

        Ok(OperationResponse { 
            time: Instant::now().duration_since(start_time),
            responses: vec![OutputResponse { value: Value::Image { data: Arc::new(output), change_id: get_id() } }],
        })
    }
}

#[cfg(test)]
#[path = "emboss_tests.rs"]
mod tests;
