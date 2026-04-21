//! Blit (pixel-copy overlay) compositing operation.
//!
//! Overlays a foreground image onto a background image at a specified x/y
//! position using alpha-aware pixel copying. Reimplemented with FloatImage.

use crate::get_id;
use crate::input::{Input, InputSettings};
use crate::node_settings::NodeSettings;
use crate::operations::{OperationResponse, OperationError, OutputResponse, default_image, convert_input};
use crate::output::Output;
use crate::value::{Value, ValueType};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use std::time::Instant;

/// Operation that blits (overlays) a foreground image onto a background.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpImageCombineBlit {}

impl OpImageCombineBlit {
    pub fn settings() -> NodeSettings {
        NodeSettings { name: "composite".to_string(), description: "Pastes a foreground image onto a background at a given position.".to_string() }
    }

    pub fn create_inputs() -> Vec<Input> {
        vec![
            Input::new("background".to_string(),  Value::Image { data:default_image(), change_id:get_id() }, None, None),
            Input::new("foreground".to_string(),  Value::Image { data:default_image(), change_id:get_id() }, None, None),
            Input::new("position x".to_string(), Value::Integer(i32::default()), Some(InputSettings::DragValue { speed:None, clamp:None }), None),
            Input::new("position y".to_string(), Value::Integer(i32::default()), Some(InputSettings::DragValue { speed:None, clamp:None }), None),
        ]
    }

    pub fn create_outputs() -> Vec<Output> {
        vec![Output::new("output".to_string(), Value::Image { data:default_image(), change_id:get_id()}, None)]
    }

    /// Overlays the foreground onto the background at the given position using alpha compositing.
    pub async fn run(inputs: &mut [Input]) -> Result<OperationResponse, OperationError> {
        let start_time = Instant::now();
        let mut input_errors: Vec<(usize, String)> = vec![];

        let background_converted = convert_input(inputs, 0, ValueType::Image, &mut input_errors);
        let foreground_converted = convert_input(inputs, 1, ValueType::Image, &mut input_errors);
        let position_x_converted = convert_input(inputs, 2, ValueType::Integer, &mut input_errors);
        let position_y_converted = convert_input(inputs, 3, ValueType::Integer, &mut input_errors);

        if !input_errors.is_empty() { return Err(OperationError { input_errors, node_error: None }); }

        let Value::Image{data:background_arc, change_id:_} = background_converted.unwrap() else { unreachable!() };
        let Value::Image{data:foreground, change_id:_} = foreground_converted.unwrap() else { unreachable!() };
        let Value::Integer(x_off) = position_x_converted.unwrap() else { unreachable!() };
        let Value::Integer(y_off) = position_y_converted.unwrap() else { unreachable!() };

        // Try to take ownership to avoid cloning
        let mut background = Arc::try_unwrap(background_arc).unwrap_or_else(|a| (*a).clone());
        let bg_ch = background.channels() as usize;
        let fg_ch = foreground.channels() as usize;
        let (bg_w, bg_h) = background.dimensions();

        // Alpha-composite foreground onto background
        for fy in 0..foreground.height() {
            for fx in 0..foreground.width() {
                let bx = fx as i64 + x_off as i64;
                let by = fy as i64 + y_off as i64;
                if bx < 0 || by < 0 || bx >= bg_w as i64 || by >= bg_h as i64 { continue; }
                let bx = bx as u32;
                let by = by as u32;

                let fg_px = foreground.get_pixel(fx, fy);
                // Get foreground alpha (1.0 if no alpha channel)
                let fg_a = if fg_ch == 2 { fg_px[1] } else if fg_ch == 4 { fg_px[3] } else { 1.0 };

                let bg_px = background.get_pixel_mut(bx, by);
                let bg_a = if bg_ch == 2 { bg_px[1] } else if bg_ch == 4 { bg_px[3] } else { 1.0 };

                // Standard "over" alpha compositing for color channels
                let color_ch = bg_ch.min(fg_ch).min(3);
                for c in 0..color_ch {
                    bg_px[c] = fg_px[c] * fg_a + bg_px[c] * (1.0 - fg_a);
                }
                // Composite alpha
                let new_a = fg_a + bg_a * (1.0 - fg_a);
                if bg_ch == 2 { bg_px[1] = new_a; }
                else if bg_ch == 4 { bg_px[3] = new_a; }
            }
        }

        Ok(OperationResponse { 
            time: Instant::now().duration_since(start_time),
            responses: vec![OutputResponse {value: Value::Image { data: Arc::new(background), change_id:get_id() }}],
        })
    }
}

#[cfg(test)]
#[path = "blit_tests.rs"]
mod tests;
