//! Tone curve adjustment operation for images (Photoshop-style curves).
//!
//! Maps pixel values through a user-drawn [`Curve`]: the curve's x axis is the
//! input value and its (flipped) y axis is the output value, exactly like the
//! curves dialog in Photoshop. The curve is edited as an embedded box in the
//! node settings panel (see `InputSettings::ToneCurve`), with the source
//! image's histogram drawn behind it.

use crate::curve::Curve;
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

// The LUT machinery lives in the shared module (other operations remap
// values through tone curves too); re-exported here so existing callers
// keep compiling.
pub use crate::operations::images::tone_curve::{sample_lut, tone_curve_lut, TONE_LUT_SIZE};
use crate::operations::images::tone_curve::{identity_tone_curve, optional_lut, tone_curve_input};

/// Tone curve adjustment mapping pixel values through a user-drawn spline.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpImageAdjustmentCurves{}

impl OpImageAdjustmentCurves {
    /// Returns the node metadata (name and description) for the curves operation.
    pub fn settings() -> NodeSettings {
        NodeSettings {
            name: "curves".to_string(),
            description: "Maps image values through a user-drawn tone curve.".to_string(),
            help: "A Photoshop-style curves adjustment. Each curve is a function from input value (horizontal axis, 0 = black on the left) to output value (vertical axis, 0 = black at the bottom). The default diagonal leaves the image unchanged; dragging the middle up brightens, down darkens, and an S shape adds contrast.\n\nThere are four curves, each with its own box in this settings panel: the master curve applies to every colour channel, while the red, green and blue curves apply to just that channel — raising the blue curve's midtones cools the image, lowering it warms it, exactly like Photoshop's per-channel curves. Per-channel curves run first, then the master curve on top; untouched curves cost nothing.\n\nEdit a curve in its box: drag points to move them, click the line to add a point, double- or right-click a point to remove it (minimum 2). Points keep their left-to-right order while dragging, like Photoshop. The source image's histogram is drawn behind the grid when the image input is connected.\n\nAlpha is left alone, and grayscale images (1 or 2 channels) use the master curve only. Left of the first point and right of the last, the curve extends flat at that point's output value. Curve nodes can also be connected to drive any of the mappings.".to_string(),
        }
    }

    /// The identity tone curve: a straight diagonal from input 0 → output 0
    /// (bottom-left in y-down curve coordinates is `[0, 1]`) to input 1 →
    /// output 1 (`[1, 0]`). Applying it leaves the image unchanged.
    pub fn identity_curve() -> Curve {
        identity_tone_curve()
    }

    /// Creates the input ports: image, the master tone curve, and one tone
    /// curve per colour channel.
    pub fn create_inputs() -> Vec<Input> {
        vec![
            Input::new("image".to_string(),  Value::Image { data:default_image(), change_id:get_id() }, None, None)
                .with_description("Source image to apply the tone curve to."),
            Input::new("curve".to_string(), Value::Curve(Self::identity_curve()), Some(InputSettings::ToneCurve), None)
                .with_description("The master tone curve, applied to every colour channel; edited in the box below, or connected from a curve node."),
            tone_curve_input("red", "Tone curve applied to the red channel before the master curve; ignored for grayscale images."),
            tone_curve_input("green", "Tone curve applied to the green channel before the master curve; ignored for grayscale images."),
            tone_curve_input("blue", "Tone curve applied to the blue channel before the master curve; ignored for grayscale images."),
        ]
    }

    /// Creates the output port: the curve-adjusted image.
    pub fn create_outputs() -> Vec<Output> {
        vec![
            Output::new("output".to_string(), Value::Image { data:default_image(), change_id:get_id()}, None)
                .with_description("Image with the tone curve applied per colour channel."),
        ]
    }

    /// Executes the curves adjustment: builds a LUT from each touched curve and
    /// maps every colour-channel value through the per-channel curve and then
    /// the master curve (alpha untouched).
    pub async fn run(inputs: &mut [Input]) -> Result<OperationResponse, OperationError> {
        let start_time = Instant::now();
        let mut input_errors: Vec<(usize, String)> = vec![];

        // convert inputs
        let image_converted = convert_input(inputs, 0, ValueType::Image, &mut input_errors);
        let curve_converted = convert_input(inputs, 1, ValueType::Curve, &mut input_errors);
        let red_converted = convert_input(inputs, 2, ValueType::Curve, &mut input_errors);
        let green_converted = convert_input(inputs, 3, ValueType::Curve, &mut input_errors);
        let blue_converted = convert_input(inputs, 4, ValueType::Curve, &mut input_errors);

        // return if error
        if !input_errors.is_empty() { return Err(OperationError { input_errors, node_error: None }); }

        // get values
        let Value::Image{data, change_id:_} = image_converted.unwrap() else { unreachable!() };
        let Value::Curve(curve) = curve_converted.unwrap() else { unreachable!() };
        let Value::Curve(red) = red_converted.unwrap() else { unreachable!() };
        let Value::Curve(green) = green_converted.unwrap() else { unreachable!() };
        let Value::Curve(blue) = blue_converted.unwrap() else { unreachable!() };

        // Build a LUT once per touched curve; untouched (identity) curves skip
        // the remap entirely so a freshly dropped node is a true no-op.
        let master = optional_lut(&curve);
        let channel_luts = [optional_lut(&red), optional_lut(&green), optional_lut(&blue)];

        let ch = data.channels() as usize;
        let color_ch = if ch == 2 || ch == 4 { ch - 1 } else { ch };
        // Grayscale carries no channel identity, so only the master applies.
        let use_channel_luts = color_ch >= 3;

        if master.is_none() && (!use_channel_luts || channel_luts.iter().all(|l| l.is_none())) {
            return Ok(OperationResponse {
                time: Instant::now().duration_since(start_time),
                responses: vec![
                    OutputResponse { value: Value::Image { data, change_id: get_id() } },
                ],
            });
        }

        let mut result = (*data).clone();
        for pixel in result.pixels_mut() {
            for (i, val) in pixel.iter_mut().take(color_ch).enumerate() {
                // Per-channel curve first, then the master curve on top.
                if use_channel_luts {
                    if let Some(lut) = channel_luts.get(i).and_then(|l| l.as_ref()) {
                        *val = sample_lut(lut, *val);
                    }
                }
                if let Some(lut) = &master {
                    *val = sample_lut(lut, *val);
                }
            }
            // alpha unchanged
        }

        Ok(OperationResponse {
            time: Instant::now().duration_since(start_time),
            responses: vec![
                OutputResponse {value: Value::Image { data:Arc::new(result), change_id:get_id() }},
            ],
        })
    }
}

#[cfg(test)]
#[path = "curves_tests.rs"]
mod tests;
