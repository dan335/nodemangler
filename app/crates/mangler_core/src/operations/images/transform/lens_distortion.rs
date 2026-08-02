//! Lens distortion: Brown–Conrady radial barrel/pincushion correction.
//!
//! Inverse-mapped per pixel with bilinear sampling, same shape as
//! [`super::transform::OpImageTransformAffine`]: a normalized radius about the
//! image centre drives a polynomial distortion factor, `scale` compensates
//! for the resulting edge crop, and `edge`/`fill color` decide what fills any
//! space the mapping exposes.

use crate::color::Color;
use crate::float_image::FloatImage;
use crate::get_id;
use crate::input::{Input, InputSettings};
use crate::node_settings::NodeSettings;
use crate::operations::images::transform::transform::sample_bilinear;
use crate::operations::{OperationResponse, OperationError, OutputResponse, default_image, convert_input};
use crate::output::Output;
use crate::value::{EdgeMode, Value, ValueType};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use std::time::Instant;

/// Radial lens distortion (barrel / pincushion) via the Brown–Conrady model.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpImageTransformLensDistortion {}

impl OpImageTransformLensDistortion {
    pub fn settings() -> NodeSettings {
        NodeSettings {
            name: "lens distortion".to_string(),
            description: "Barrel/pincushion radial lens distortion (Brown–Conrady k1/k2 model).".to_string(),
            help: "Brown–Conrady radial distortion model (k1/k2 terms): negative k = barrel correction, positive = pincushion; scale compensates edge cropping.\n\nEach destination pixel is mapped back to a source coordinate through a polynomial of the normalized radius r from the image centre (r is normalized by half the longer of width/height on both axes, so distortion stays radially symmetric instead of stretching on the narrow axis): factor = 1 + k1·r² + k2·r⁴. Positive `k1`/`k2` samples further out from the centre as r grows (pincushion); negative samples closer to the centre (barrel correction, e.g. undoing a wide-angle lens's bulge). `scale` then zooms about the centre — above 1 zooms in and crops the edges, below 1 zooms out — handy for cropping back in the black/fill corners that heavy barrel correction exposes.\n\n`edge` and `fill color` control what appears in any space the mapping exposes (see the `transform` node for the same four modes). Sampling is bilinear and alpha-carrying images are resampled premultiplied, so transparent regions never bleed hidden colour into the distorted edges. Output dimensions and channel count match the input.".to_string(),
        }
    }

    pub fn create_inputs() -> Vec<Input> {
        vec![
            Input::new("image".to_string(), Value::Image { data: default_image(), change_id: get_id() }, None, None)
                .with_description("Source image to distort."),
            Input::new("k1".to_string(), Value::Decimal(0.0), Some(InputSettings::Slider { range: (-1.0, 1.0), step_by: Some(0.01), clamp_to_range: false }), None)
                .with_description("Quadratic (r²) distortion coefficient; negative = barrel, positive = pincushion."),
            Input::new("k2".to_string(), Value::Decimal(0.0), Some(InputSettings::Slider { range: (-1.0, 1.0), step_by: Some(0.01), clamp_to_range: false }), None)
                .with_description("Quartic (r⁴) distortion coefficient; shapes distortion further from the centre than k1 alone."),
            Input::new("scale".to_string(), Value::Decimal(1.0), Some(InputSettings::Slider { range: (0.5, 2.0), step_by: Some(0.01), clamp_to_range: false }), None)
                .with_description("Zoom about the centre; above 1 zooms in (cropping the edges), below 1 zooms out. Use it to crop out corners the distortion exposes."),
            Input::new("edge mode".to_string(), Value::EdgeMode(EdgeMode::Fill), None, None)
                .with_description("What fills space the distortion exposes: fill colour, wrap, extend, or mirror."),
            Input::new("fill color".to_string(), Value::Color(Color { r: 0.0, g: 0.0, b: 0.0, a: 0.0 }), None, None)
                .with_description("Colour used for exposed space when edge mode = fill (default transparent)."),
        ]
    }

    pub fn create_outputs() -> Vec<Output> {
        vec![
            Output::new("output".to_string(), Value::Image { data: default_image(), change_id: get_id() }, None)
                .with_description("The distorted image, same size and channel count as the input."),
        ]
    }

    pub async fn run(inputs: &mut [Input]) -> Result<OperationResponse, OperationError> {
        let start_time = Instant::now();
        let mut input_errors: Vec<(usize, String)> = vec![];

        let image_converted = convert_input(inputs, 0, ValueType::Image, &mut input_errors);
        let k1_converted = convert_input(inputs, 1, ValueType::Decimal, &mut input_errors);
        let k2_converted = convert_input(inputs, 2, ValueType::Decimal, &mut input_errors);
        let scale_converted = convert_input(inputs, 3, ValueType::Decimal, &mut input_errors);
        let edge_converted = convert_input(inputs, 4, ValueType::EdgeMode, &mut input_errors);
        let fill_converted = convert_input(inputs, 5, ValueType::Color, &mut input_errors);

        if !input_errors.is_empty() { return Err(OperationError { input_errors, node_error: None }); }

        let Value::Image { data, change_id: _ } = image_converted.unwrap() else { unreachable!() };
        let Value::Decimal(k1) = k1_converted.unwrap() else { unreachable!() };
        let Value::Decimal(k2) = k2_converted.unwrap() else { unreachable!() };
        let Value::Decimal(scale) = scale_converted.unwrap() else { unreachable!() };
        let Value::EdgeMode(edge) = edge_converted.unwrap() else { unreachable!() };
        let Value::Color(fill) = fill_converted.unwrap() else { unreachable!() };

        // Degenerate params: no distortion, no zoom — passthrough.
        if k1 == 0.0 && k2 == 0.0 && scale == 1.0 {
            return Ok(OperationResponse {
                time: Instant::now().duration_since(start_time),
                responses: vec![OutputResponse { value: Value::Image { data, change_id: get_id() } }],
            });
        }

        let (width, height) = data.dimensions();
        let nch = data.channels() as usize;

        // Fill colour reduced to the source's channel layout (see transform.rs).
        let luma = 0.2126 * fill.r + 0.7152 * fill.g + 0.0722 * fill.b;
        let mut fill_px: Vec<f32> = match nch {
            1 => vec![luma],
            2 => vec![luma, fill.a],
            3 => vec![fill.r, fill.g, fill.b],
            _ => vec![fill.r, fill.g, fill.b, fill.a],
        };

        // Premultiply so transparent pixels' hidden colour can't bleed into
        // interpolated edge pixels.
        let premul = data.has_alpha();
        let src = if premul { Arc::new(data.premultiply_alpha()) } else { Arc::clone(&data) };
        if premul {
            let a = *fill_px.last().unwrap();
            for c in &mut fill_px[..nch - 1] { *c *= a; }
        }

        // Guard against a zero scale (division below).
        let safe_scale = if scale.abs() < 1e-3 { 1e-3 } else { scale };

        let cx = width as f32 / 2.0;
        let cy = height as f32 / 2.0;
        // Normalize both axes by the same radius so distortion stays radially
        // symmetric instead of anisotropic on the narrow axis.
        let r_ref = width.max(height) as f32 / 2.0;

        let mut output = FloatImage::new(width, height, data.channels());
        let mut acc = vec![0.0f32; nch];
        for y in 0..height {
            for x in 0..width {
                let nx = (x as f32 + 0.5 - cx) / r_ref;
                let ny = (y as f32 + 0.5 - cy) / r_ref;
                let r2 = nx * nx + ny * ny;
                let f = 1.0 + k1 * r2 + k2 * r2 * r2;

                let sx = cx + nx * f * r_ref / safe_scale - 0.5;
                let sy = cy + ny * f * r_ref / safe_scale - 0.5;

                sample_bilinear(&src, sx, sy, edge, &fill_px, &mut acc);
                output.put_pixel(x, y, &acc);
            }
        }

        if premul { output.unpremultiply_alpha(); }

        Ok(OperationResponse {
            time: Instant::now().duration_since(start_time),
            responses: vec![
                OutputResponse { value: Value::Image { data: Arc::new(output), change_id: get_id() } },
            ],
        })
    }
}

#[cfg(test)]
#[path = "lens_distortion_tests.rs"]
mod tests;
