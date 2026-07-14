//! Parameter-`t` gradient painted outward from a curve.
//!
//! Stamps the curve's polyline into a site mask, tagging each site pixel with
//! its arc-length fraction `t` (0 at the start, 1 at the end). Every output
//! pixel then takes the `t` of its nearest site pixel, so the curve's
//! parameterization spreads outward — a flow / depth ramp along a river
//! centerline, for example.

use crate::curve::Curve;
use crate::float_image::FloatImage;
use crate::get_id;
use crate::input::{Input, InputSettings};
use crate::node_settings::NodeSettings;
use crate::operations::curves::common::{cumulative_arc, flatten_f64};
use crate::operations::images::simulation::distance_field_labeled;
use crate::operations::{convert_input, default_image, scale_to_resolution, OperationError, OperationResponse, OutputResponse};
use crate::output::Output;
use crate::value::{Value, ValueType};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use std::time::Instant;

#[cfg(test)]
#[path = "curve_gradient_tests.rs"]
mod tests;

/// Operation that paints the curve's arc-length parameter outward as a ramp.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpImageShapeCurveGradient {}

impl OpImageShapeCurveGradient {
    /// Returns the node metadata (name, description, help).
    pub fn settings() -> NodeSettings {
        NodeSettings {
            name: "curve gradient".to_string(),
            description: "Paints a curve's arc-length parameter outward as a ramp.".to_string(),
            help: "Stamps the curve into a site mask, tagging each on-curve pixel with its arc-length fraction: 0 at the start of the curve, 1 at the end. Every output pixel then reads the fraction of its nearest on-curve pixel, so the curve's t-parameter spreads outward through the image.\n\nWith 'max distance' > 0 (pixels at a 1024px reference), pixels farther than that from the curve output 0. A degenerate or empty curve produces a black image. Pairs with meander's centerline for a flow or depth ramp.".to_string(),
        }
    }

    /// Creates the default inputs: curve, width, height, max distance.
    pub fn create_inputs() -> Vec<Input> {
        vec![
            Input::new("curve".to_string(), Value::Curve(Curve::default()), None, None)
                .with_description("The curve whose arc-length parameter is painted outward."),
            Input::new("width".to_string(), Value::Integer(512), Some(InputSettings::DragValue { clamp: Some((1.0, 10000.0)), speed: None }), None)
                .with_description("Output image width in pixels."),
            Input::new("height".to_string(), Value::Integer(512), Some(InputSettings::DragValue { clamp: Some((1.0, 10000.0)), speed: None }), None)
                .with_description("Output image height in pixels."),
            Input::new("max distance".to_string(), Value::Decimal(0.0), Some(InputSettings::DragValue { clamp: Some((0.0, 2048.0)), speed: Some(1.0) }), None)
                .with_description("Beyond this distance (pixels at a 1024px reference) pixels are 0; 0 = unlimited."),
        ]
    }

    /// Creates the default output: a single grayscale image.
    pub fn create_outputs() -> Vec<Output> {
        vec![
            Output::new("output".to_string(), Value::Image { data: default_image(), change_id: get_id() }, None)
                .with_description("Grayscale image carrying the curve's t-parameter, spread from the nearest point."),
        ]
    }

    /// Paints the curve's arc-length parameter outward into a grayscale image.
    pub async fn run(inputs: &mut [Input]) -> Result<OperationResponse, OperationError> {
        let start_time = Instant::now();
        let mut input_errors: Vec<(usize, String)> = vec![];

        let curve_converted = convert_input(inputs, 0, ValueType::Curve, &mut input_errors);
        let width_converted = convert_input(inputs, 1, ValueType::Integer, &mut input_errors);
        let height_converted = convert_input(inputs, 2, ValueType::Integer, &mut input_errors);
        let max_dist_converted = convert_input(inputs, 3, ValueType::Decimal, &mut input_errors);

        if !input_errors.is_empty() {
            return Err(OperationError { input_errors, node_error: None });
        }

        let Value::Curve(curve) = curve_converted.unwrap() else { unreachable!() };
        let Value::Integer(mut width) = width_converted.unwrap() else { unreachable!() };
        let Value::Integer(mut height) = height_converted.unwrap() else { unreachable!() };
        let Value::Decimal(max_distance) = max_dist_converted.unwrap() else { unreachable!() };

        width = width.max(1);
        height = height.max(1);
        let w = width as usize;
        let h = height as usize;

        let mut pixels = vec![0.0f32; w * h];

        // Flatten into pixel space so segment stepping and distances are in px.
        let poly_norm = flatten_f64(&curve, 48);
        let poly: Vec<[f64; 2]> = poly_norm
            .iter()
            .map(|p| [p[0] * width as f64, p[1] * height as f64])
            .collect();

        if poly.len() >= 2 {
            let mut arc: Vec<f64> = Vec::new();
            cumulative_arc(&poly, &mut arc);
            let total = *arc.last().unwrap();

            if total > 0.0 {
                let mut sites = vec![false; w * h];
                let mut tvals = vec![0.0f32; w * h];

                // Stamp each segment densely (<= 0.5px steps) so the site line
                // is 8-connected; last write wins on overlapping pixels.
                for i in 0..poly.len() - 1 {
                    let a = poly[i];
                    let b = poly[i + 1];
                    let seg_px = ((b[0] - a[0]).powi(2) + (b[1] - a[1]).powi(2)).sqrt();
                    let steps = ((seg_px / 0.5).ceil() as usize).max(1);
                    let arc_a = arc[i];
                    let seg_norm = arc[i + 1] - arc[i];
                    for s in 0..=steps {
                        let f = s as f64 / steps as f64;
                        let x = a[0] + f * (b[0] - a[0]);
                        let y = a[1] + f * (b[1] - a[1]);
                        let xi = (x.floor() as i64).clamp(0, w as i64 - 1) as usize;
                        let yi = (y.floor() as i64).clamp(0, h as i64 - 1) as usize;
                        let t = ((arc_a + f * seg_norm) / total) as f32;
                        let idx = yi * w + xi;
                        sites[idx] = true;
                        tvals[idx] = t;
                    }
                }

                if sites.iter().any(|&s| s) {
                    let (d2, label) = distance_field_labeled(&sites, w, h);
                    let max_dist_px = if max_distance > 0.0 {
                        scale_to_resolution(max_distance, width as u32, height as u32) as f64
                    } else {
                        f64::INFINITY
                    };
                    for idx in 0..w * h {
                        let lab = label[idx];
                        if lab == u32::MAX || d2[idx].sqrt() > max_dist_px {
                            pixels[idx] = 0.0;
                        } else {
                            pixels[idx] = tvals[lab as usize];
                        }
                    }
                }
            }
        }

        let image = FloatImage::from_raw(width as u32, height as u32, 1, pixels).unwrap();

        Ok(OperationResponse {
            time: Instant::now().duration_since(start_time),
            responses: vec![OutputResponse { value: Value::Image { data: Arc::new(image), change_id: get_id() } }],
        })
    }
}
