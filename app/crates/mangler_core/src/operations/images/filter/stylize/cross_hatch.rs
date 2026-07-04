//! Cross-hatch pen-and-ink stylization.
//!
//! Reproduces continuous tone with a stack of parallel line screens at
//! different angles, as seen in engraving and pen drawings. Four hatch
//! layers at 45°, -45°, 0°, and 90° are progressively enabled as luminance
//! decreases — bright regions get no hatching, mid-tones get one layer of
//! diagonal strokes, and the darkest regions get all four crossing.
//!
//! A pixel belongs to a hatch line when its signed distance to the nearest
//! parallel of that layer is smaller than `line thickness`. The spacing
//! between parallels controls overall stroke density. Output is binary:
//! line = black, paper = white.

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

/// Cross-hatch pen-and-ink stylization filter.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpImageAdjustmentCrossHatch {}

impl OpImageAdjustmentCrossHatch {
    /// Returns the node metadata for the cross-hatch filter.
    pub fn settings() -> NodeSettings {
        NodeSettings {
            name: "cross hatch".to_string(),
            description: "Pen-and-ink cross-hatch stylization — four progressive hatch layers driven by luminance.".to_string(),
            help: "Stacks four parallel line screens at 45, -45, 0, and 90 degrees. A pixel belongs to a layer's line when the signed distance to the nearest parallel (via projection onto the layer's normal, modulo spacing) is within `thickness`. Layers activate progressively as luminance drops below each of the four thresholds, so bright areas stay clean and the darkest regions get all four layers crossing.\n\nSpacing sets stroke density; thickness sets stroke weight. Output is binary (black ink, white paper) with the input alpha preserved.".to_string(),
        }
    }

    /// Creates input ports: image, line spacing, stroke thickness, and four
    /// tone thresholds (one per hatch layer). Each threshold is the luminance
    /// below which that layer's strokes become active.
    pub fn create_inputs() -> Vec<Input> {
        vec![
            Input::new("image".to_string(), Value::Image { data: default_image(), change_id: get_id() }, None, None)
                .with_description("Source image whose luminance drives the density of hatch strokes."),
            // Distance (in pixels) between successive hatch lines in a layer
            Input::new("spacing".to_string(), Value::Decimal(6.0), Some(InputSettings::Slider { range: (2.0, 32.0), step_by: Some(0.5), clamp_to_range: true }), None)
                .with_description("Distance in pixels between parallel hatch lines in each layer."),
            // Half-width of each stroke (in pixels) — i.e. anti-alias-free radius
            Input::new("thickness".to_string(), Value::Decimal(0.8), Some(InputSettings::Slider { range: (0.1, 4.0), step_by: Some(0.1), clamp_to_range: true }), None)
                .with_description("Half-width of each stroke in pixels."),
            // Luminance cutoffs: below threshold_n → layer n draws ink. Ordered
            // so each subsequent darker tone keeps the previous layers and
            // adds another.
            Input::new("threshold 1".to_string(), Value::Decimal(0.8), Some(InputSettings::Slider { range: (0.0, 1.0), step_by: Some(0.01), clamp_to_range: true }), None)
                .with_description("Luminance cutoff below which the 45-degree hatch layer activates."),
            Input::new("threshold 2".to_string(), Value::Decimal(0.6), Some(InputSettings::Slider { range: (0.0, 1.0), step_by: Some(0.01), clamp_to_range: true }), None)
                .with_description("Luminance cutoff below which the -45-degree hatch layer activates."),
            Input::new("threshold 3".to_string(), Value::Decimal(0.4), Some(InputSettings::Slider { range: (0.0, 1.0), step_by: Some(0.01), clamp_to_range: true }), None)
                .with_description("Luminance cutoff below which the horizontal hatch layer activates."),
            Input::new("threshold 4".to_string(), Value::Decimal(0.2), Some(InputSettings::Slider { range: (0.0, 1.0), step_by: Some(0.01), clamp_to_range: true }), None)
                .with_description("Luminance cutoff below which the vertical hatch layer activates."),
        ]
    }

    /// Creates the output port: the cross-hatch binary image.
    pub fn create_outputs() -> Vec<Output> {
        vec![
            Output::new("output".to_string(), Value::Image { data: default_image(), change_id: get_id() }, None)
                .with_description("Binary pen-and-ink image with hatch strokes drawn in dark regions."),
        ]
    }

    /// Runs the cross-hatch filter.
    pub async fn run(inputs: &mut [Input]) -> Result<OperationResponse, OperationError> {
        let start_time = Instant::now();
        let mut input_errors: Vec<(usize, String)> = vec![];

        let image_converted = convert_input(inputs, 0, ValueType::Image, &mut input_errors);
        let spacing_converted = convert_input(inputs, 1, ValueType::Decimal, &mut input_errors);
        let thickness_converted = convert_input(inputs, 2, ValueType::Decimal, &mut input_errors);
        let t1_converted = convert_input(inputs, 3, ValueType::Decimal, &mut input_errors);
        let t2_converted = convert_input(inputs, 4, ValueType::Decimal, &mut input_errors);
        let t3_converted = convert_input(inputs, 5, ValueType::Decimal, &mut input_errors);
        let t4_converted = convert_input(inputs, 6, ValueType::Decimal, &mut input_errors);

        if !input_errors.is_empty() { return Err(OperationError { input_errors, node_error: None }); }

        let Value::Image { data, change_id: _ } = image_converted.unwrap() else { unreachable!() };
        let Value::Decimal(spacing) = spacing_converted.unwrap() else { unreachable!() };
        let Value::Decimal(thickness) = thickness_converted.unwrap() else { unreachable!() };
        let Value::Decimal(t1) = t1_converted.unwrap() else { unreachable!() };
        let Value::Decimal(t2) = t2_converted.unwrap() else { unreachable!() };
        let Value::Decimal(t3) = t3_converted.unwrap() else { unreachable!() };
        let Value::Decimal(t4) = t4_converted.unwrap() else { unreachable!() };

        let spacing = spacing.max(1.0);
        let thickness = thickness.max(0.1);

        let (width, height) = data.dimensions();
        let ch = data.channels() as usize;

        // The four hatch layers are parallel line-sets rotated by these angles.
        // Each layer's "line coordinate" is the projection of (x, y) onto its
        // normal direction; a pixel is on a line if that coordinate mod
        // spacing is within ±thickness of 0 (accounting for wrap-around).
        let angles = [
            std::f32::consts::FRAC_PI_4,          // 45°: primary diagonal
           -std::f32::consts::FRAC_PI_4,          // -45°: anti-diagonal
            0.0,                                  // 0°: horizontal lines (normal along Y)
            std::f32::consts::FRAC_PI_2,          // 90°: vertical lines (normal along X)
        ];
        let thresholds = [t1, t2, t3, t4];

        // Pre-compute normal vectors per layer so the hot loop stays cheap
        let normals: [(f32, f32); 4] = {
            let mut ns = [(0.0, 0.0); 4];
            for i in 0..4 {
                // Normal is perpendicular to the line direction
                let a = angles[i] + std::f32::consts::FRAC_PI_2;
                ns[i] = (a.cos(), a.sin());
            }
            ns
        };

        let mut out = FloatImage::new(width, height, ch as u32);
        for y in 0..height {
            for x in 0..width {
                let src = data.get_pixel(x, y);
                let lum = if ch >= 3 {
                    0.2126 * src[0] + 0.7152 * src[1] + 0.0722 * src[2]
                } else {
                    src[0]
                };

                // Decide whether this pixel lands on any active hatch line
                let fx = x as f32;
                let fy = y as f32;
                let mut on_line = false;
                for layer in 0..4 {
                    // Layer is active only if the local luminance is below its cutoff
                    if lum >= thresholds[layer] { continue; }

                    let (nx, ny) = normals[layer];
                    // Project onto the normal; parallels are at integer multiples of `spacing`
                    let proj = fx * nx + fy * ny;
                    // Distance to the nearest parallel, accounting for wrap via the midpoint
                    let mut d = proj.rem_euclid(spacing);
                    if d > spacing * 0.5 { d = spacing - d; }
                    if d <= thickness {
                        on_line = true;
                        break;
                    }
                }

                // Ink on line, paper elsewhere; alpha preserved from source
                let v = if on_line { 0.0 } else { 1.0 };
                let mut pixel = [0.0f32; 4];
                for val in pixel.iter_mut().take(ch.min(3)) { *val = v; }
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
#[path = "cross_hatch_tests.rs"]
mod tests;
