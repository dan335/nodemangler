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
use crate::operations::images::tone_curve::identity_tone_curve;

/// Tone curve adjustment mapping pixel values through a user-drawn spline.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpImageAdjustmentCurves{}

impl OpImageAdjustmentCurves {
    /// Returns the node metadata (name and description) for the curves operation.
    pub fn settings() -> NodeSettings {
        NodeSettings {
            name: "curves".to_string(),
            description: "Maps image values through a user-drawn tone curve.".to_string(),
            help: "A Photoshop-style curves adjustment. The curve is a function from input value (horizontal axis, 0 = black on the left) to output value (vertical axis, 0 = black at the bottom). The default diagonal leaves the image unchanged; dragging the middle up brightens, down darkens, and an S shape adds contrast.\n\nEdit the curve in the box in this settings panel: drag points to move them, click the line to add a point, double- or right-click a point to remove it (minimum 2). Points keep their left-to-right order while dragging, like Photoshop. The source image's histogram is drawn behind the grid when the image input is connected.\n\nThe same curve is applied to each colour channel; alpha is left alone. Left of the first point and right of the last, the curve extends flat at that point's output value. A curve node can also be connected to drive the mapping.".to_string(),
        }
    }

    /// The identity tone curve: a straight diagonal from input 0 → output 0
    /// (bottom-left in y-down curve coordinates is `[0, 1]`) to input 1 →
    /// output 1 (`[1, 0]`). Applying it leaves the image unchanged.
    pub fn identity_curve() -> Curve {
        identity_tone_curve()
    }

    /// Creates the input ports: image and the tone curve.
    pub fn create_inputs() -> Vec<Input> {
        vec![
            Input::new("image".to_string(),  Value::Image { data:default_image(), change_id:get_id() }, None, None)
                .with_description("Source image to apply the tone curve to."),
            Input::new("curve".to_string(), Value::Curve(Self::identity_curve()), Some(InputSettings::ToneCurve), None)
                .with_description("The tone mapping curve; edited in the box below, or connected from a curve node."),
        ]
    }

    /// Creates the output port: the curve-adjusted image.
    pub fn create_outputs() -> Vec<Output> {
        vec![
            Output::new("output".to_string(), Value::Image { data:default_image(), change_id:get_id()}, None)
                .with_description("Image with the tone curve applied per colour channel."),
        ]
    }

    /// Executes the curves adjustment: builds a LUT from the curve and maps
    /// every colour-channel value through it (alpha untouched).
    pub async fn run(inputs: &mut [Input]) -> Result<OperationResponse, OperationError> {
        let start_time = Instant::now();
        let mut input_errors: Vec<(usize, String)> = vec![];

        // convert inputs
        let image_converted = convert_input(inputs, 0, ValueType::Image, &mut input_errors);
        let curve_converted = convert_input(inputs, 1, ValueType::Curve, &mut input_errors);

        // return if error
        if !input_errors.is_empty() { return Err(OperationError { input_errors, node_error: None }); }

        // get values
        let Value::Image{data, change_id:_} = image_converted.unwrap() else { unreachable!() };
        let Value::Curve(curve) = curve_converted.unwrap() else { unreachable!() };

        // run node — build the LUT once, then remap every colour channel.
        let lut = tone_curve_lut(&curve, TONE_LUT_SIZE);
        let mut result = (*data).clone();
        let ch = result.channels() as usize;
        let color_ch = if ch == 2 || ch == 4 { ch - 1 } else { ch };

        for pixel in result.pixels_mut() {
            for val in pixel.iter_mut().take(color_ch) {
                *val = sample_lut(&lut, *val);
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
