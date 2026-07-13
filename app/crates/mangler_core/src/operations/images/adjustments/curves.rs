//! Tone curve adjustment operation for images (Photoshop-style curves).
//!
//! Maps pixel values through a user-drawn [`Curve`]: the curve's x axis is the
//! input value and its (flipped) y axis is the output value, exactly like the
//! curves dialog in Photoshop. The curve is edited as an embedded box in the
//! node settings panel (see `InputSettings::ToneCurve`), with the source
//! image's histogram drawn behind it.

use crate::curve::{Curve, CurveInterpolation};
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

/// Number of entries in the lookup table built from the curve. 1024 keeps
/// interpolation error invisible on f32 images while staying cheap to build.
const LUT_SIZE: usize = 1024;

/// Samples per spline segment when flattening the curve for LUT rasterization.
/// Matches `Curve`'s standard tolerance; far denser than the LUT bin spacing
/// for typical point counts, so no interior bins are left unfilled.
const FLATTEN_SAMPLES: usize = 48;

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
        Curve {
            points: vec![[0.0, 1.0], [1.0, 0.0]],
            closed: false,
            interpolation: CurveInterpolation::Smooth,
            handles: Vec::new(),
        }
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
        let lut = tone_curve_lut(&curve, LUT_SIZE);
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

/// Build an `n`-entry lookup table (input value → output value, both in
/// `[0,1]`) from a tone curve.
///
/// The curve is in y-down `[0,1]²` coordinates, so `output = 1 - y`. The
/// flattened spline is rasterized into bins: entry `i` samples the curve at
/// input `i / (n-1)`. Bins left of the first point / right of the last are
/// filled flat with the nearest endpoint's output (Photoshop's clamp
/// behaviour). A locally x-reversed spline (possible with extreme control
/// points) stays a function — later segments simply overwrite earlier bins.
/// Degenerate curves fall back gracefully: no points → identity ramp, a
/// single point → a constant.
pub fn tone_curve_lut(curve: &Curve, n: usize) -> Vec<f32> {
    debug_assert!(n >= 2);
    let poly = curve.flatten(FLATTEN_SAMPLES);

    if poly.is_empty() {
        // No curve at all — identity ramp.
        return (0..n).map(|i| i as f32 / (n - 1) as f32).collect();
    }
    if poly.len() == 1 {
        // A single point maps everything to its output value.
        return vec![(1.0 - poly[0][1]).clamp(0.0, 1.0); n];
    }

    // NaN marks "not yet written"; filled from the segments, then extended.
    let mut lut = vec![f32::NAN; n];
    let last_bin = (n - 1) as f32;

    for seg in poly.windows(2) {
        // Convert to (input x, output value) with the y-down flip.
        let (mut x0, mut v0) = (seg[0][0], 1.0 - seg[0][1]);
        let (mut x1, mut v1) = (seg[1][0], 1.0 - seg[1][1]);
        if x1 < x0 {
            // Walk every segment left→right so the bin fill below is a
            // simple ascending range regardless of spline direction.
            std::mem::swap(&mut x0, &mut x1);
            std::mem::swap(&mut v0, &mut v1);
        }

        // Bins whose sample position i/(n-1) falls inside [x0, x1].
        let i0 = (x0.clamp(0.0, 1.0) * last_bin).ceil() as usize;
        let i1 = (x1.clamp(0.0, 1.0) * last_bin).floor() as usize;
        for i in i0..=i1.min(n - 1) {
            let x = i as f32 / last_bin;
            // Degenerate (vertical) segments write the far endpoint's value.
            let t = if x1 > x0 { (x - x0) / (x1 - x0) } else { 1.0 };
            lut[i] = (v0 + t * (v1 - v0)).clamp(0.0, 1.0);
        }
    }

    // Extend flat past the curve's ends, and paper over any interior gap by
    // carrying the previous value (gaps can only appear if the flattening is
    // coarser than the bin spacing, which the constants above prevent).
    let first_valid = lut.iter().copied().find(|v| !v.is_nan());
    let Some(first_valid) = first_valid else {
        // Nothing landed in [0,1] at all — identity ramp.
        return (0..n).map(|i| i as f32 / (n - 1) as f32).collect();
    };
    let mut prev = first_valid;
    for v in lut.iter_mut() {
        if v.is_nan() {
            *v = prev;
        } else {
            prev = *v;
        }
    }
    lut
}

/// Sample a LUT built by [`tone_curve_lut`] at input `val` (clamped to
/// `[0,1]`), linearly interpolating between adjacent entries.
pub fn sample_lut(lut: &[f32], val: f32) -> f32 {
    let last = lut.len() - 1;
    let t = val.clamp(0.0, 1.0) * last as f32;
    let i = (t.floor() as usize).min(last);
    let f = t - i as f32;
    let a = lut[i];
    let b = lut[(i + 1).min(last)];
    a + f * (b - a)
}

#[cfg(test)]
#[path = "curves_tests.rs"]
mod tests;
