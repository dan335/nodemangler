//! Photoshop-style Vibrance / Saturation adjustment for images.
//!
//! Exposes two independent controls that both act on a pixel's HSL saturation:
//!
//! - `saturation` is a uniform multiplier applied to every pixel's chroma,
//!   just like a classic saturation slider. It scales all colours equally,
//!   regardless of how vivid they already are.
//! - `vibrance` is a non-linear, protective boost. Its effect is weighted by
//!   `(1 - s)`, so muted / low-saturation colours are pushed much harder than
//!   already-saturated ones. This mirrors Photoshop's Vibrance control, whose
//!   whole purpose is to lift dull colours while sparing skin tones and
//!   colours that are already near full saturation (which would otherwise
//!   clip and look garish).
//!
//! This is a faithful *heuristic* approximation of Adobe's proprietary
//! Vibrance algorithm, not a bit-exact reproduction — Adobe's exact curve is
//! undocumented. The `(1 - s)` weighting captures the characteristic
//! behaviour, but the numeric response will not match Photoshop pixel-for-pixel.

use crate::get_id;
use crate::value::ValueType;
use crate::input::{Input, InputSettings};
use crate::node_settings::NodeSettings;
use crate::operations::{OperationResponse, OperationError, OutputResponse, default_image, convert_input};
use crate::operations::images::adjustments::common::{rgb_to_hsl, hsl_to_rgb};
use crate::output::Output;
use crate::value::Value;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use std::time::Instant;

/// Photoshop-style Vibrance + Saturation adjustment operating in HSL space.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpImageAdjustmentVibrance {}

impl OpImageAdjustmentVibrance {
    /// Returns the node metadata (name, description, and detailed help) for vibrance.
    pub fn settings() -> NodeSettings {
        NodeSettings {
            name: "vibrance".to_string(),
            description: "Photoshop-style vibrance and saturation. Vibrance protects already-vivid colours; saturation scales all colours equally.".to_string(),
            help: "Adjusts colour intensity with two controls that both act on each pixel's HSL saturation.\n\n\
The image is converted to HSL per pixel. The saturation is then modified as:\n\
  s' = s * (1 + saturation) + vibrance * (1 - s)\n\
and clamped to [0, 1] before converting back to RGB. Hue and lightness are preserved, and alpha is left untouched.\n\n\
The 'saturation' control is a uniform multiplier: (1 + saturation) scales every pixel's chroma equally, exactly like a conventional saturation slider. Negative values desaturate toward gray (saturation = -1 removes all chroma), positive values intensify.\n\n\
The 'vibrance' control is a protective, non-linear boost. Its contribution is weighted by (1 - s), so pixels that are already highly saturated (s near 1) receive almost no change, while muted, low-saturation pixels are pushed the most. This is what lets Vibrance lift dull colours and skies without over-cooking skin tones or colours that are already vivid (which would otherwise clip to garish extremes).\n\n\
This is a faithful HEURISTIC approximation of Adobe Photoshop's Vibrance, not a bit-exact reproduction — Adobe's exact response curve is proprietary and undocumented. The (1 - s) weighting reproduces the characteristic 'protect the vivid, lift the dull' behaviour, but numeric values will not match Photoshop exactly.\n\n\
Grayscale inputs (fewer than 3 channels) have no chroma and pass through unchanged.".to_string(),
        }
    }

    /// Creates input ports: the source image plus vibrance and saturation sliders.
    pub fn create_inputs() -> Vec<Input> {
        vec![
            Input::new("image".to_string(), Value::Image { data: default_image(), change_id: get_id() }, None, None)
                .with_description("Source colour image to adjust."),
            Input::new("vibrance".to_string(), Value::Decimal(0.0), Some(InputSettings::Slider { range: (-1.0, 1.0), step_by: Some(0.01), clamp_to_range: true }), None)
                .with_description("Protective saturation boost weighted by (1 - s): lifts muted colours, spares already-vivid ones. 0 is identity."),
            Input::new("saturation".to_string(), Value::Decimal(0.0), Some(InputSettings::Slider { range: (-1.0, 1.0), step_by: Some(0.01), clamp_to_range: true }), None)
                .with_description("Uniform saturation multiplier applied to all colours equally. 0 is identity, -1 grayscale."),
        ]
    }

    /// Creates the output port: the adjusted image.
    pub fn create_outputs() -> Vec<Output> {
        vec![
            Output::new("output".to_string(), Value::Image { data: default_image(), change_id: get_id() }, None)
                .with_description("Vibrance/saturation-adjusted image."),
        ]
    }

    /// Executes the vibrance/saturation adjustment by remapping HSL saturation per pixel.
    pub async fn run(inputs: &mut [Input]) -> Result<OperationResponse, OperationError> {
        let start_time = Instant::now();
        let mut input_errors: Vec<(usize, String)> = vec![];

        // Convert and validate all inputs up front.
        let image_converted = convert_input(inputs, 0, ValueType::Image, &mut input_errors);
        let vibrance_converted = convert_input(inputs, 1, ValueType::Decimal, &mut input_errors);
        let saturation_converted = convert_input(inputs, 2, ValueType::Decimal, &mut input_errors);

        if !input_errors.is_empty() { return Err(OperationError { input_errors, node_error: None }); }

        let Value::Image { data, change_id: _ } = image_converted.unwrap() else { unreachable!() };
        let Value::Decimal(vibrance) = vibrance_converted.unwrap() else { unreachable!() };
        let Value::Decimal(saturation) = saturation_converted.unwrap() else { unreachable!() };

        let ch = data.channels() as usize;
        if ch < 3 {
            // Fewer than 3 channels means no chroma exists; pass the image through unchanged.
            return Ok(OperationResponse {
                time: Instant::now().duration_since(start_time),
                responses: vec![OutputResponse { value: Value::Image { data, change_id: get_id() } }],
            });
        }

        let mut result = (*data).clone();
        for pixel in result.pixels_mut() {
            // Read the RGB colour channels (channel 3, if present, is alpha and is left alone).
            let (r, g, b) = (pixel[0], pixel[1], pixel[2]);

            // Work in HSL so we can operate directly on the saturation component.
            let (h, s, l) = rgb_to_hsl(r, g, b);

            // Uniform saturation multiplier: scales all pixels' chroma equally.
            let s_sat = s * (1.0 + saturation);

            // Vibrance: boosts LESS-saturated pixels more via the (1 - s) weight,
            // protecting colours that are already vivid (and thus skin tones).
            let mut s2 = s_sat + vibrance * (1.0 - s);

            // HSL saturation must stay within [0, 1] to be valid.
            s2 = s2.clamp(0.0, 1.0);

            // Convert back to RGB and write the colour channels, preserving alpha.
            let (nr, ng, nb) = hsl_to_rgb(h, s2, l);
            pixel[0] = nr;
            pixel[1] = ng;
            pixel[2] = nb;
        }

        Ok(OperationResponse {
            time: Instant::now().duration_since(start_time),
            responses: vec![OutputResponse { value: Value::Image { data: Arc::new(result), change_id: get_id() } }],
        })
    }
}

#[cfg(test)]
#[path = "vibrance_tests.rs"]
mod tests;
