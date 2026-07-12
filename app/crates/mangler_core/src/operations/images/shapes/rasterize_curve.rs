//! Rasterize a drawn curve into a grayscale image mask.
//!
//! Bakes a [`Curve`] (open path or closed shape) into a 1-channel FloatImage:
//! an anti-aliased stroke along the path, optionally filled when the curve is
//! closed. Feeds mask inputs such as `carve_river`'s river-guide mask.

use crate::curve::Curve;
use crate::float_image::FloatImage;
use crate::get_id;
use crate::input::{Input, InputSettings};
use crate::node_settings::NodeSettings;
use crate::operations::{OperationResponse, OperationError, OutputResponse, default_image, convert_input, scale_to_resolution};
use crate::output::Output;
use crate::value::{Value, ValueType};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use std::time::Instant;

/// Operation that rasterizes a drawn [`Curve`] into a grayscale mask image.
///
/// The stroke width and feather are authored as pixels at a 1024px reference
/// and scaled to the actual output resolution (see [`scale_to_resolution`]),
/// so the same values give the same relative stroke at any size.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpImageShapeRasterizeCurve {}

impl OpImageShapeRasterizeCurve {
    /// Returns the node metadata (name, description, help).
    pub fn settings() -> NodeSettings {
        NodeSettings {
            name: "rasterize curve".to_string(),
            description: "Bakes a drawn curve into a grayscale mask.".to_string(),
            help: "Renders a curve into a 1-channel grayscale image: an anti-aliased stroke along the path, white on black. When the curve is closed and 'fill' is on, the enclosed area is filled too; open paths are always stroke-only.\n\nThe curve's normalized 0-1 points are mapped onto the output dimensions, so a non-square target stretches the curve to fit. Stroke width and feather are in pixels at a 1024px reference and scale with resolution. An empty or single-point curve produces a black image.".to_string(),
        }
    }

    /// Creates the default inputs: curve, width, height, stroke width, feather, fill.
    pub fn create_inputs() -> Vec<Input> {
        vec![
            Input::new("curve".to_string(), Value::Curve(Curve::default()), None, None)
                .with_description("The curve to rasterize; usually connected from a curve node."),
            Input::new("width".to_string(), Value::Integer(512), Some(InputSettings::DragValue { clamp: Some((1.0, 10000.0)), speed: None }), None)
                .with_description("Width of the generated image in pixels."),
            Input::new("height".to_string(), Value::Integer(512), Some(InputSettings::DragValue { clamp: Some((1.0, 10000.0)), speed: None }), None)
                .with_description("Height of the generated image in pixels."),
            Input::new("stroke width".to_string(), Value::Decimal(8.0), Some(InputSettings::DragValue { clamp: Some((0.0, 512.0)), speed: Some(0.1) }), None)
                .with_description("Stroke width in pixels at a 1024px reference; scales with resolution."),
            Input::new("feather".to_string(), Value::Decimal(0.0), Some(InputSettings::DragValue { clamp: Some((0.0, 512.0)), speed: Some(0.1) }), None)
                .with_description("Edge softness in pixels at a 1024px reference; 0 = crisp anti-aliased edge."),
            Input::new("fill".to_string(), Value::Bool(true), None, None)
                .with_description("Fill the enclosed area (applies only when the curve is closed)."),
        ]
    }

    /// Creates the default output: a single grayscale image.
    pub fn create_outputs() -> Vec<Output> {
        vec![
            Output::new("output".to_string(), Value::Image { data: default_image(), change_id: get_id() }, None)
                .with_description("Grayscale mask with the curve drawn white on a black background."),
        ]
    }

    /// Rasterizes the curve input into a 1-channel grayscale mask image.
    pub async fn run(inputs: &mut [Input]) -> Result<OperationResponse, OperationError> {
        let start_time = Instant::now();
        let mut input_errors: Vec<(usize, String)> = vec![];

        // convert inputs
        let curve_converted = convert_input(inputs, 0, ValueType::Curve, &mut input_errors);
        let width_converted = convert_input(inputs, 1, ValueType::Integer, &mut input_errors);
        let height_converted = convert_input(inputs, 2, ValueType::Integer, &mut input_errors);
        let stroke_converted = convert_input(inputs, 3, ValueType::Decimal, &mut input_errors);
        let feather_converted = convert_input(inputs, 4, ValueType::Decimal, &mut input_errors);
        let fill_converted = convert_input(inputs, 5, ValueType::Bool, &mut input_errors);

        // return if error
        if !input_errors.is_empty() { return Err(OperationError { input_errors, node_error: None }); }

        // get values
        let Value::Curve(curve) = curve_converted.unwrap() else { unreachable!() };
        let Value::Integer(mut width) = width_converted.unwrap() else { unreachable!() };
        let Value::Integer(mut height) = height_converted.unwrap() else { unreachable!() };
        let Value::Decimal(stroke_width) = stroke_converted.unwrap() else { unreachable!() };
        let Value::Decimal(feather) = feather_converted.unwrap() else { unreachable!() };
        let Value::Bool(fill) = fill_converted.unwrap() else { unreachable!() };

        // run node
        width = width.max(1);
        height = height.max(1);
        let w = width as u32;
        let h = height as u32;

        // Stroke width is a full width; the rasterizer takes a half-width radius.
        let radius = scale_to_resolution(stroke_width.max(0.0), w, h) * 0.5;
        let feather_px = scale_to_resolution(feather.max(0.0), w, h);

        let pixels = curve.rasterize(w, h, radius, feather_px, fill);
        let image = FloatImage::from_raw(w, h, 1, pixels).unwrap();

        Ok(OperationResponse {
            time: Instant::now().duration_since(start_time),
            responses: vec![
                OutputResponse { value: Value::Image { data: Arc::new(image), change_id: get_id() } },
            ],
        })
    }
}


#[cfg(test)]
#[path = "rasterize_curve_tests.rs"]
mod tests;
