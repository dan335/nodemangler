//! Distance field radiating out from a curve.
//!
//! Rasterizes a [`Curve`] to a thin site mask, computes the exact Euclidean
//! distance from every pixel to the nearest on-curve pixel, and maps it to a
//! grayscale ramp: white on the curve fading to black over `falloff` pixels
//! (or, with `normalize`, over the image's own maximum distance).

use crate::curve::Curve;
use crate::float_image::FloatImage;
use crate::get_id;
use crate::input::{Input, InputSettings};
use crate::node_settings::NodeSettings;
use crate::operations::images::simulation::distance_field_labeled;
use crate::operations::images::tone_curve::{optional_lut, sample_lut, tone_curve_input};
use crate::operations::{convert_input, default_image, scale_to_resolution, OperationError, OperationResponse, OutputResponse};
use crate::output::Output;
use crate::value::{Value, ValueType};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use std::time::Instant;

#[cfg(test)]
#[path = "curve_distance_field_tests.rs"]
mod tests;

/// Operation that renders a distance field around a drawn curve.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpImageShapeCurveDistanceField {}

impl OpImageShapeCurveDistanceField {
    /// Returns the node metadata (name, description, help).
    pub fn settings() -> NodeSettings {
        NodeSettings {
            name: "curve distance field".to_string(),
            description: "Grayscale distance field radiating out from a curve.".to_string(),
            help: "Rasterizes the curve into a thin mask, then computes each pixel's distance to the nearest point on the curve. The output is white (1) on the curve and fades to black over 'falloff' pixels (at a 1024px reference, scaled with resolution). Turn on 'normalize' to fade over the image's own maximum distance instead of a fixed falloff, and 'invert' to flip white and black.\n\nA degenerate or empty curve produces a black image. Pairs with 'curve gradient' and feeds masks or height blends.\n\n'profile' reshapes the falloff ramp itself, after normalize/falloff but before invert (x: 0 = far from the curve, 1 = on the curve) — a Photoshop-contour-style shaping curve. The default diagonal leaves the linear ramp unchanged.".to_string(),
        }
    }

    /// Creates the default inputs: curve, width, height, falloff, normalize, invert, profile.
    ///
    /// Two different `Value::Curve` editors coexist on this node: `curve` is a
    /// *spatial* path edited via the Preview2D overlay (see the Curve bullet in
    /// CLAUDE.md), while `profile` is a *tone* curve — a value-mapping function
    /// edited as an embedded box in the settings panel (`InputSettings::ToneCurve`).
    pub fn create_inputs() -> Vec<Input> {
        vec![
            Input::new("curve".to_string(), Value::Curve(Curve::default()), None, None)
                .with_description("The curve the distance field radiates from."),
            Input::new("width".to_string(), Value::Integer(512), Some(InputSettings::DragValue { clamp: Some((1.0, 10000.0)), speed: None }), None)
                .with_description("Output image width in pixels."),
            Input::new("height".to_string(), Value::Integer(512), Some(InputSettings::DragValue { clamp: Some((1.0, 10000.0)), speed: None }), None)
                .with_description("Output image height in pixels."),
            Input::new("falloff".to_string(), Value::Decimal(128.0), Some(InputSettings::DragValue { clamp: Some((1.0, 1024.0)), speed: Some(1.0) }), None)
                .with_description("Fade distance in pixels at a 1024px reference; scales with resolution. Ignored when 'normalize' is on."),
            Input::new("normalize".to_string(), Value::Bool(false), None, None)
                .with_description("Fade over the image's maximum distance instead of the fixed falloff."),
            Input::new("invert".to_string(), Value::Bool(false), None, None)
                .with_description("Flip the ramp: black on the curve fading to white."),
            tone_curve_input("profile", "Reshapes the falloff ramp, applied after falloff/normalize but before invert (x: 0 = far from the curve, 1 = on the curve). Default diagonal leaves the linear ramp unchanged."),
        ]
    }

    /// Creates the default output: a single grayscale image.
    pub fn create_outputs() -> Vec<Output> {
        vec![
            Output::new("output".to_string(), Value::Image { data: default_image(), change_id: get_id() }, None)
                .with_description("Grayscale distance field, white on the curve fading outward."),
        ]
    }

    /// Renders the distance field of the curve input into a grayscale image.
    pub async fn run(inputs: &mut [Input]) -> Result<OperationResponse, OperationError> {
        let start_time = Instant::now();
        let mut input_errors: Vec<(usize, String)> = vec![];

        let curve_converted = convert_input(inputs, 0, ValueType::Curve, &mut input_errors);
        let width_converted = convert_input(inputs, 1, ValueType::Integer, &mut input_errors);
        let height_converted = convert_input(inputs, 2, ValueType::Integer, &mut input_errors);
        let falloff_converted = convert_input(inputs, 3, ValueType::Decimal, &mut input_errors);
        let normalize_converted = convert_input(inputs, 4, ValueType::Bool, &mut input_errors);
        let invert_converted = convert_input(inputs, 5, ValueType::Bool, &mut input_errors);
        let profile_converted = convert_input(inputs, 6, ValueType::Curve, &mut input_errors);

        if !input_errors.is_empty() {
            return Err(OperationError { input_errors, node_error: None });
        }

        let Value::Curve(curve) = curve_converted.unwrap() else { unreachable!() };
        let Value::Integer(mut width) = width_converted.unwrap() else { unreachable!() };
        let Value::Integer(mut height) = height_converted.unwrap() else { unreachable!() };
        let Value::Decimal(falloff) = falloff_converted.unwrap() else { unreachable!() };
        let Value::Bool(normalize) = normalize_converted.unwrap() else { unreachable!() };
        let Value::Bool(invert) = invert_converted.unwrap() else { unreachable!() };
        let Value::Curve(profile_curve) = profile_converted.unwrap() else { unreachable!() };
        let lut = optional_lut(&profile_curve);

        width = width.max(1);
        height = height.max(1);
        let w = width as usize;
        let h = height as usize;

        // Rasterize a thin stroke, then treat lit pixels as distance-field sites.
        let mask = curve.rasterize(width as u32, height as u32, 0.75, 0.0, false);
        let sites: Vec<bool> = mask.iter().map(|&v| v > 0.5).collect();

        let mut pixels = vec![0.0f32; w * h];
        if sites.iter().any(|&s| s) {
            let (d2, _label) = distance_field_labeled(&sites, w, h);
            let falloff_px = scale_to_resolution(falloff.max(1.0), width as u32, height as u32) as f64;

            let denom = if normalize {
                d2.iter().cloned().fold(0.0f64, f64::max).sqrt().max(1e-6)
            } else {
                falloff_px.max(1e-6)
            };

            for (out, &dd) in pixels.iter_mut().zip(d2.iter()) {
                let d = dd.sqrt();
                let mut val = 1.0 - (d / denom).clamp(0.0, 1.0);
                if let Some(lut) = &lut {
                    val = sample_lut(lut, val as f32) as f64;
                }
                if invert {
                    val = 1.0 - val;
                }
                *out = val as f32;
            }
        }
        // Empty sites (degenerate/empty curve): leave the image black.

        let image = FloatImage::from_raw(width as u32, height as u32, 1, pixels).unwrap();

        Ok(OperationResponse {
            time: Instant::now().duration_since(start_time),
            responses: vec![OutputResponse { value: Value::Image { data: Arc::new(image), change_id: get_id() } }],
        })
    }
}
