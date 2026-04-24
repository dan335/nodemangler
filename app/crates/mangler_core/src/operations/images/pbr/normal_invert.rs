//! Flips X and/or Y on a normal map.
//!
//! Essential for moving a normal map between OpenGL (Y-up) and DirectX (Y-down)
//! conventions, or for mirrored UV handling. Operates on the packed `[0, 1]`
//! representation directly — no unpack/repack needed, just `1.0 - v`.

use crate::float_image::FloatImage;
use crate::get_id;
use crate::input::Input;
use crate::node_settings::NodeSettings;
use crate::operations::{OperationResponse, OperationError, OutputResponse, default_image, convert_input};
use crate::output::Output;
use crate::value::{Value, ValueType};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use std::time::Instant;

/// Normal-map axis inverter.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpImagePbrNormalInvert {}

impl OpImagePbrNormalInvert {
    pub fn settings() -> NodeSettings {
        NodeSettings {
            name: "normal invert".to_string(),
            description: "Flips the X and/or Y components of a normal map (OpenGL ↔ DirectX).".to_string(),
        }
    }

    pub fn create_inputs() -> Vec<Input> {
        vec![
            Input::new("image".to_string(), Value::Image { data: default_image(), change_id: get_id() }, None, None),
            Input::new("invert x".to_string(), Value::Bool(false), None, None),
            Input::new("invert y".to_string(), Value::Bool(true), None, None),
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
        let invert_x_converted = convert_input(inputs, 1, ValueType::Bool, &mut input_errors);
        let invert_y_converted = convert_input(inputs, 2, ValueType::Bool, &mut input_errors);

        if !input_errors.is_empty() { return Err(OperationError { input_errors, node_error: None }); }

        let Value::Image { data, change_id: _ } = image_converted.unwrap() else { unreachable!() };
        let Value::Bool(invert_x) = invert_x_converted.unwrap() else { unreachable!() };
        let Value::Bool(invert_y) = invert_y_converted.unwrap() else { unreachable!() };

        // Mirror each selected axis in packed space: v → 1 - v maps a signed
        // component n → -n after unpack. Alpha / z components are preserved.
        let (w, h) = data.dimensions();
        let ch = data.channels();
        let mut output = FloatImage::new(w, h, ch);
        let ch_usize = ch as usize;
        for y in 0..h {
            for x in 0..w {
                let src = data.get_pixel(x, y);
                let mut px = [0.0f32; 4];
                for c in 0..ch_usize {
                    px[c] = src[c];
                }
                if invert_x && ch_usize >= 1 { px[0] = 1.0 - px[0]; }
                if invert_y && ch_usize >= 2 { px[1] = 1.0 - px[1]; }
                output.put_pixel(x, y, &px[..ch_usize]);
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
#[path = "normal_invert_tests.rs"]
mod tests;
