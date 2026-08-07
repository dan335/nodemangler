//! Hue range mask — select pixels by hue band with optional chroma floor.
//!
//! Complements `color to mask` (RGB Euclidean distance to a target). This node
//! uses circular hue distance in HSL plus a minimum-saturation gate so near-gray
//! pixels can be rejected even when their arbitrary hue happens to fall in-band.

use crate::float_image::FloatImage;
use crate::get_id;
use crate::input::{Input, InputSettings};
use crate::node_settings::NodeSettings;
use crate::operations::images::adjustments::common::{rgb_to_hsl, smoothstep};
use crate::operations::{
    OperationError, OperationResponse, OutputResponse, convert_input, default_image,
};
use crate::output::Output;
use crate::value::{Value, ValueType};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use std::time::Instant;

/// Circular absolute difference between two hues in degrees, result in [0, 180].
#[inline]
fn hue_distance(a: f32, b: f32) -> f32 {
    let mut d = (a - b).abs() % 360.0;
    if d > 180.0 {
        d = 360.0 - d;
    }
    d
}

/// Hue-band selection mask.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpImageMaskHueRange {}

impl OpImageMaskHueRange {
    /// Returns the node metadata (name, description, help).
    pub fn settings() -> NodeSettings {
        NodeSettings {
            name: "hue range mask".to_string(),
            description: "Single-channel mask selecting a hue band, with optional chroma floor.".to_string(),
            help: "Converts each RGB pixel to HSL and measures circular hue distance to the target `hue` (degrees). Pixels within `range` degrees are fully selected; beyond `range + softness` they are fully rejected, with a smoothstep fade between. Distance wraps at 0/360 so a band centred on red works cleanly.\n\n`min chroma` (HSL saturation) gates near-gray pixels: below the floor the mask is zero, so sky/skin/foliage selections don't pick up neutral shadows just because their undefined hue happens to land in-band. Set min chroma to 0 to disable the gate. Grayscale inputs (fewer than 3 channels) have no hue and emit an all-zero mask.\n\nComplements color to mask, which selects by RGB Euclidean distance to a target colour rather than by hue.".to_string(),
        }
    }

    /// Creates inputs: image, hue, range, softness, min chroma, invert.
    pub fn create_inputs() -> Vec<Input> {
        vec![
            Input::new(
                "image".to_string(),
                Value::Image {
                    data: default_image(),
                    change_id: get_id(),
                },
                None,
                None,
            )
            .with_description("Source image whose hue is selected into a mask."),
            Input::new(
                "hue".to_string(),
                Value::Decimal(0.0),
                Some(InputSettings::Slider {
                    range: (0.0, 360.0),
                    step_by: Some(1.0),
                    clamp_to_range: false,
                }),
                None,
            )
            .with_description("Centre hue of the selection band in degrees (0 = red, 120 = green, 240 = blue)."),
            Input::new(
                "range".to_string(),
                Value::Decimal(30.0),
                Some(InputSettings::Slider {
                    range: (0.0, 180.0),
                    step_by: Some(1.0),
                    clamp_to_range: true,
                }),
                None,
            )
            .with_description("Half-width of the fully-selected hue band in degrees."),
            Input::new(
                "softness".to_string(),
                Value::Decimal(10.0),
                Some(InputSettings::Slider {
                    range: (0.0, 180.0),
                    step_by: Some(1.0),
                    clamp_to_range: true,
                }),
                None,
            )
            .with_description("Degrees of smooth fade past the range edge."),
            Input::new(
                "min chroma".to_string(),
                Value::Decimal(0.05),
                Some(InputSettings::Slider {
                    range: (0.0, 1.0),
                    step_by: Some(0.01),
                    clamp_to_range: true,
                }),
                None,
            )
            .with_description("Minimum HSL saturation; pixels below this are rejected. 0 disables."),
            Input::new("invert".to_string(), Value::Bool(false), None, None)
                .with_description("Flip the mask so out-of-band hues are selected."),
        ]
    }

    /// Creates the single mask output.
    pub fn create_outputs() -> Vec<Output> {
        vec![Output::new(
            "output".to_string(),
            Value::Image {
                data: default_image(),
                change_id: get_id(),
            },
            None,
        )
        .with_description("Single-channel mask; 1 = in hue band, 0 = rejected.")]
    }

    /// Builds the hue-range mask from the source image.
    pub async fn run(inputs: &mut [Input]) -> Result<OperationResponse, OperationError> {
        let start_time = Instant::now();
        let mut input_errors: Vec<(usize, String)> = vec![];

        let image_converted = convert_input(inputs, 0, ValueType::Image, &mut input_errors);
        let hue_converted = convert_input(inputs, 1, ValueType::Decimal, &mut input_errors);
        let range_converted = convert_input(inputs, 2, ValueType::Decimal, &mut input_errors);
        let softness_converted = convert_input(inputs, 3, ValueType::Decimal, &mut input_errors);
        let min_chroma_converted = convert_input(inputs, 4, ValueType::Decimal, &mut input_errors);
        let invert_converted = convert_input(inputs, 5, ValueType::Bool, &mut input_errors);

        if !input_errors.is_empty() {
            return Err(OperationError {
                input_errors,
                node_error: None,
            });
        }

        let Value::Image { data, change_id: _ } = image_converted.unwrap() else {
            unreachable!()
        };
        let Value::Decimal(hue) = hue_converted.unwrap() else {
            unreachable!()
        };
        let Value::Decimal(range) = range_converted.unwrap() else {
            unreachable!()
        };
        let Value::Decimal(softness) = softness_converted.unwrap() else {
            unreachable!()
        };
        let Value::Decimal(min_chroma) = min_chroma_converted.unwrap() else {
            unreachable!()
        };
        let Value::Bool(invert) = invert_converted.unwrap() else {
            unreachable!()
        };

        let hue = hue.rem_euclid(360.0);
        let range = range.clamp(0.0, 180.0);
        let softness = softness.clamp(0.0, 180.0);
        let min_chroma = min_chroma.clamp(0.0, 1.0);
        let e0 = range;
        let e1 = range + softness;

        let (w, h) = data.dimensions();
        let ch = data.channels() as usize;
        let mut output = FloatImage::new(w, h, 1);

        // Grayscale has no meaningful hue — emit zeros (or ones if inverted).
        if ch < 3 {
            let fill = if invert { 1.0 } else { 0.0 };
            for y in 0..h {
                for x in 0..w {
                    output.put_pixel(x, y, &[fill]);
                }
            }
            return Ok(OperationResponse {
                time: Instant::now().duration_since(start_time),
                responses: vec![OutputResponse {
                    value: Value::Image {
                        data: Arc::new(output),
                        change_id: get_id(),
                    },
                }],
            });
        }

        for y in 0..h {
            for x in 0..w {
                let p = data.get_pixel(x, y);
                let (ph, ps, _pl) = rgb_to_hsl(p[0], p[1], p[2]);
                let dist = hue_distance(ph, hue);
                // 1 inside range, fade to 0 over softness.
                let mut m = 1.0 - smoothstep(e0, e1, dist);
                // Soft chroma gate: fully reject below min_chroma; above is untouched.
                // A tiny ramp (0.02) avoids a hard cut when min_chroma is small.
                if min_chroma > 0.0 {
                    let gate = smoothstep(min_chroma * 0.5, min_chroma, ps);
                    m *= gate;
                }
                if invert {
                    m = 1.0 - m;
                }
                output.put_pixel(x, y, &[m]);
            }
        }

        Ok(OperationResponse {
            time: Instant::now().duration_since(start_time),
            responses: vec![OutputResponse {
                value: Value::Image {
                    data: Arc::new(output),
                    change_id: get_id(),
                },
            }],
        })
    }
}

#[cfg(test)]
#[path = "hue_range_tests.rs"]
mod tests;
