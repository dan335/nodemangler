//! Halftone dot-screen filter.
//!
//! Simulates the classic printing technique where continuous tones are
//! reproduced with a grid of dots whose size scales with local darkness.
//! Bright regions → tiny dots (mostly white paper); dark regions → large
//! dots (mostly ink). Output is binary: 0 or 1 per color channel.
//!
//! Implementation:
//!   1. Rotate a regular grid by `angle` degrees.
//!   2. For each output pixel, find the center of its rotated cell.
//!   3. Sample the input's average luminance around that cell center.
//!   4. Inside the cell, draw a filled disk whose radius encodes 1 - lum.
//!
//! The rotation angle is how real halftone screens avoid moiré when stacking
//! multiple color plates — here it's a single channel, but exposing angle
//! keeps the output distinct from a plain regular-grid dot pattern.

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

/// Halftone dot-screen stylization filter.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpImageAdjustmentHalftone {}

impl OpImageAdjustmentHalftone {
    /// Returns the node metadata for halftone.
    pub fn settings() -> NodeSettings {
        NodeSettings {
            name: "halftone".to_string(),
            description: "Halftone dot-screen: reproduces tone with a rotated grid of size-modulated dots.".to_string(),
        }
    }

    /// Creates input ports: image, cell size (grid period in pixels), and
    /// screen rotation angle in degrees (classic value ≈ 45°).
    pub fn create_inputs() -> Vec<Input> {
        vec![
            Input::new("image".to_string(), Value::Image { data: default_image(), change_id: get_id() }, None, None),
            // Grid period in pixels — size of each halftone cell
            Input::new("cell size".to_string(), Value::Integer(8), Some(InputSettings::Slider { range: (2.0, 64.0), step_by: Some(1.0), clamp_to_range: true }), None),
            // Screen rotation in degrees
            Input::new("angle".to_string(), Value::Decimal(45.0), Some(InputSettings::Slider { range: (0.0, 180.0), step_by: Some(1.0), clamp_to_range: true }), None),
        ]
    }

    /// Creates the output port: the halftoned binary image.
    pub fn create_outputs() -> Vec<Output> {
        vec![
            Output::new("output".to_string(), Value::Image { data: default_image(), change_id: get_id() }, None),
        ]
    }

    /// Runs the halftone filter.
    pub async fn run(inputs: &mut [Input]) -> Result<OperationResponse, OperationError> {
        let start_time = Instant::now();
        let mut input_errors: Vec<(usize, String)> = vec![];

        let image_converted = convert_input(inputs, 0, ValueType::Image, &mut input_errors);
        let cell_converted = convert_input(inputs, 1, ValueType::Integer, &mut input_errors);
        let angle_converted = convert_input(inputs, 2, ValueType::Decimal, &mut input_errors);

        if !input_errors.is_empty() { return Err(OperationError { input_errors, node_error: None }); }

        let Value::Image { data, change_id: _ } = image_converted.unwrap() else { unreachable!() };
        let Value::Integer(cell_size) = cell_converted.unwrap() else { unreachable!() };
        let Value::Decimal(angle_deg) = angle_converted.unwrap() else { unreachable!() };

        let cell = cell_size.max(2) as f32;
        let angle = angle_deg.to_radians();
        let (sin_a, cos_a) = angle.sin_cos();

        let (width, height) = data.dimensions();
        let ch = data.channels() as usize;

        // Helper: sample luminance at a floating-point position using the
        // nearest pixel with edge clamping. Precise interpolation is
        // unnecessary — halftone averages over a whole cell anyway.
        let lum_at = |fx: f32, fy: f32| -> f32 {
            let x = (fx.round() as i32).clamp(0, width as i32 - 1) as u32;
            let y = (fy.round() as i32).clamp(0, height as i32 - 1) as u32;
            let p = data.get_pixel(x, y);
            if ch >= 3 { 0.2126 * p[0] + 0.7152 * p[1] + 0.0722 * p[2] } else { p[0] }
        };

        let mut out = FloatImage::new(width, height, ch as u32);
        let max_r = cell * 0.5 * std::f32::consts::SQRT_2;

        for y in 0..height {
            for x in 0..width {
                // Transform output pixel into the rotated halftone grid frame.
                // Using the inverse rotation (rotate by -angle) maps screen
                // coords into grid coords where the cells are axis-aligned.
                let fx = x as f32;
                let fy = y as f32;
                let gx = cos_a * fx + sin_a * fy;
                let gy = -sin_a * fx + cos_a * fy;

                // Cell index in the rotated frame
                let cx = (gx / cell).floor();
                let cy = (gy / cell).floor();
                // Center of that cell, in grid coords
                let ccx = (cx + 0.5) * cell;
                let ccy = (cy + 0.5) * cell;
                // Rotate the cell center back to screen coords to sample luminance
                let scx = cos_a * ccx - sin_a * ccy;
                let scy = sin_a * ccx + cos_a * ccy;
                let lum = lum_at(scx, scy).clamp(0.0, 1.0);

                // Distance from the current pixel to the cell center (in grid coords)
                let dx = gx - ccx;
                let dy = gy - ccy;
                let dist = (dx * dx + dy * dy).sqrt();

                // Dot radius scales with darkness: lum=1 → no dot, lum=0 → full cell
                let radius = (1.0 - lum) * max_r;
                let v = if dist <= radius { 0.0 } else { 1.0 };

                // Write the binary value to color channels; preserve alpha
                let src = data.get_pixel(x, y);
                let mut pixel = [0.0f32; 4];
                for c in 0..ch.min(3) { pixel[c] = v; }
                if ch == 2 || ch == 4 { pixel[ch - 1] = src[ch - 1]; }
                out.put_pixel(x, y, &pixel[..ch]);
            }
        }

        Ok(OperationResponse {
            time: Instant::now().duration_since(start_time),
            responses: vec![
                OutputResponse { value: Value::Image { data: Arc::new(out), change_id: get_id() } },
            ],
        })
    }
}

#[cfg(test)]
#[path = "halftone_tests.rs"]
mod tests;
