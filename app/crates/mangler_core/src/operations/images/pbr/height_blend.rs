//! Height-based material blending.
//!
//! Blends two materials (color + height) using their height maps to determine
//! which material is visible at each pixel.

use crate::float_image::FloatImage;
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

/// Operation that blends two materials based on their height maps.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpImagePbrHeightBlend {}

impl OpImagePbrHeightBlend {
    pub fn settings() -> NodeSettings {
        NodeSettings { name: "height blend".to_string(), description: "Blends two materials using their height maps for realistic layering.".to_string(), help: "At each pixel the overlay's relative height is remapped by blend amount and contrast to a 0-1 weight; that weight linearly interpolates between base and overlay color, and between base and overlay height. Raising contrast produces a sharper, more mask-like boundary so individual bumps of the overlay poke through cleanly.\n\nOutputs both the blended color (RGBA) and the combined greyscale height map (packed as RGBA) so it can feed a subsequent normal_from_height or ao_from_height. Height maps are read as Rec.709 luminance when given RGB input; color images are padded to RGBA.".to_string() }
    }

    pub fn create_inputs() -> Vec<Input> {
        vec![
            Input::new("base color".to_string(), Value::Image { data: default_image(), change_id: get_id() }, None, None)
                .with_description("Albedo/color image of the base material layer."),
            Input::new("base height".to_string(), Value::Image { data: default_image(), change_id: get_id() }, None, None)
                .with_description("Height map describing the surface of the base material."),
            Input::new("overlay color".to_string(), Value::Image { data: default_image(), change_id: get_id() }, None, None)
                .with_description("Albedo/color image of the material layered on top."),
            Input::new("overlay height".to_string(), Value::Image { data: default_image(), change_id: get_id() }, None, None)
                .with_description("Height map describing the surface of the overlay material."),
            Input::new("blend amount".to_string(), Value::Decimal(0.5), Some(InputSettings::Slider { range: (0.0, 1.0), step_by: Some(0.01), clamp_to_range: true }), None)
                .with_description("Shifts how much of the overlay shows through, 0 all base to 1 all overlay."),
            Input::new("contrast".to_string(), Value::Decimal(0.5), Some(InputSettings::Slider { range: (0.0, 1.0), step_by: Some(0.01), clamp_to_range: true }), None)
                .with_description("Sharpens the transition between the two heights."),
        ]
    }

    pub fn create_outputs() -> Vec<Output> {
        vec![
            Output::new("output".to_string(), Value::Image { data: default_image(), change_id: get_id() }, None)
                .with_description("Color image produced by height-masked blending of the two materials."),
            Output::new("height".to_string(), Value::Image { data: default_image(), change_id: get_id() }, None)
                .with_description("Combined height map of the two materials after blending."),
        ]
    }

    /// Blends two materials using height-based masking and outputs both color and height.
    pub async fn run(inputs: &mut [Input]) -> Result<OperationResponse, OperationError> {
        let start_time = Instant::now();
        let mut input_errors: Vec<(usize, String)> = vec![];

        let base_color_converted = convert_input(inputs, 0, ValueType::Image, &mut input_errors);
        let base_height_converted = convert_input(inputs, 1, ValueType::Image, &mut input_errors);
        let overlay_color_converted = convert_input(inputs, 2, ValueType::Image, &mut input_errors);
        let overlay_height_converted = convert_input(inputs, 3, ValueType::Image, &mut input_errors);
        let blend_amount_converted = convert_input(inputs, 4, ValueType::Decimal, &mut input_errors);
        let contrast_converted = convert_input(inputs, 5, ValueType::Decimal, &mut input_errors);

        if !input_errors.is_empty() { return Err(OperationError { input_errors, node_error: None }); }

        let Value::Image { data: base_color_data, change_id: _ } = base_color_converted.unwrap() else { unreachable!() };
        let Value::Image { data: base_height_data, change_id: _ } = base_height_converted.unwrap() else { unreachable!() };
        let Value::Image { data: overlay_color_data, change_id: _ } = overlay_color_converted.unwrap() else { unreachable!() };
        let Value::Image { data: overlay_height_data, change_id: _ } = overlay_height_converted.unwrap() else { unreachable!() };
        let Value::Decimal(blend_amount) = blend_amount_converted.unwrap() else { unreachable!() };
        let Value::Decimal(contrast) = contrast_converted.unwrap() else { unreachable!() };

        let width = base_color_data.width();
        let height = base_color_data.height();

        let mut color_output = FloatImage::new(width, height, 4);
        let mut height_output = FloatImage::new(width, height, 4);

        // Helper: get luminance from a FloatImage pixel
        let lum = |img: &FloatImage, x: u32, y: u32| -> f32 {
            let px = img.get_pixel(x.min(img.width() - 1), y.min(img.height() - 1));
            let ch = img.channels() as usize;
            if ch >= 3 { 0.2126 * px[0] + 0.7152 * px[1] + 0.0722 * px[2] } else { px[0] }
        };

        // Helper: get RGBA from any channel count, padding with defaults
        let get_rgba = |img: &FloatImage, x: u32, y: u32| -> [f32; 4] {
            let px = img.get_pixel(x.min(img.width() - 1), y.min(img.height() - 1));
            let ch = img.channels() as usize;
            match ch {
                1 => [px[0], px[0], px[0], 1.0],
                2 => [px[0], px[0], px[0], px[1]],
                3 => [px[0], px[1], px[2], 1.0],
                _ => [px[0], px[1], px[2], px[3]],
            }
        };

        for y in 0..height {
            for x in 0..width {
                let base_c = get_rgba(&base_color_data, x, y);
                let overlay_c = get_rgba(&overlay_color_data, x, y);
                let bh = lum(&base_height_data, x, y);
                let oh = lum(&overlay_height_data, x, y);

                let height_diff = oh - bh;
                let depth = (1.0 - contrast).max(0.001);
                let t = ((height_diff + blend_amount * 2.0 - 1.0) / depth * 0.5 + 0.5).clamp(0.0, 1.0);

                let r = base_c[0] * (1.0 - t) + overlay_c[0] * t;
                let g = base_c[1] * (1.0 - t) + overlay_c[1] * t;
                let b = base_c[2] * (1.0 - t) + overlay_c[2] * t;
                let a = base_c[3] * (1.0 - t) + overlay_c[3] * t;
                color_output.put_pixel(x, y, &[r, g, b, a]);

                let blended_h = bh * (1.0 - t) + oh * t;
                height_output.put_pixel(x, y, &[blended_h, blended_h, blended_h, 1.0]);
            }
        }

        Ok(OperationResponse { 
            time: Instant::now().duration_since(start_time),
            responses: vec![
                OutputResponse { value: Value::Image { data: Arc::new(color_output), change_id: get_id() } },
                OutputResponse { value: Value::Image { data: Arc::new(height_output), change_id: get_id() } },
            ],
        })
    }
}

#[cfg(test)]
#[path = "height_blend_tests.rs"]
mod tests;
