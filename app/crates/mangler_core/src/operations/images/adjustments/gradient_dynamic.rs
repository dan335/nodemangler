//! Gradient dynamic — per-pixel gradient lookup modulated by a vector field.
//!
//! Starts with `gradient_map`'s per-luminance lookup, then optionally adds a
//! direction-field offset: pixels near a high-X region of the field shift the
//! sampled gradient position forward, pixels near high-Y shift it backward
//! (or whichever direction the user picks with `angle`). Useful for
//! flow-aligned texturing and curvature-driven coloration.
//!
//! The vector field is expected to be a normal-map-style RGB image — R
//! mapped to `[-1, 1]` X, G mapped to `[-1, 1]` Y. If the field image is
//! smaller than the source, it's bilinear-sampled and stretched to fit.

use crate::float_image::FloatImage;
use crate::get_id;
use crate::input::{Input, InputSettings};
use crate::node_settings::NodeSettings;
use crate::operations::{OperationResponse, OperationError, OutputResponse, default_image, convert_input};
use crate::output::Output;
use crate::value::{Value, ValueType};
use rayon::prelude::*;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use std::time::Instant;

/// Gradient-map variant modulated by a vector field.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpImageAdjustmentGradientDynamic {}

impl OpImageAdjustmentGradientDynamic {
    pub fn settings() -> NodeSettings {
        NodeSettings {
            name: "gradient dynamic".to_string(),
            description: "Maps image luminance to a gradient, shifted per-pixel by a vector-field projection along a chosen angle.".to_string(),
            help: "Extends the plain gradient map by perturbing the sample position with a flow vector. Each pixel computes luminance (Rec. 709 for RGB) as the base gradient parameter, then projects the RG channels of the vector-field image (remapped from 0-1 to -1 to 1) onto the unit vector at angle degrees to produce a signed shift.\n\nThe final sample position is clamped (not wrapped) to 0-1 and bilinear-sampled along the horizontal axis of the gradient strip. The field image is stretched to fit the source with bilinear sampling, so smaller flow maps still drive large inputs. Strength scales the shift; values above 1 over-project and can push most pixels to the endpoints. Source alpha is multiplied onto the gradient's alpha.".to_string(),
        }
    }

    pub fn create_inputs() -> Vec<Input> {
        vec![
            Input::new("image".to_string(), Value::Image { data: default_image(), change_id: get_id() }, None, None)
                .with_description("Source image whose luminance drives the base gradient lookup."),
            Input::new("gradient".to_string(), Value::Image { data: default_image(), change_id: get_id() }, None, None)
                .with_description("Horizontal gradient strip sampled left-to-right based on luminance plus offset."),
            Input::new("vector field".to_string(), Value::Image { data: default_image(), change_id: get_id() }, None, None)
                .with_description("Normal-map-style RG field; R and G map to signed X/Y flow vectors."),
            Input::new("strength".to_string(), Value::Decimal(0.5), Some(InputSettings::Slider { range: (-2.0, 2.0), step_by: Some(0.01), clamp_to_range: false }), None)
                .with_description("How strongly the field projection shifts the gradient sample position."),
            Input::new("angle".to_string(), Value::Decimal(0.0), Some(InputSettings::Slider { range: (0.0, 360.0), step_by: None, clamp_to_range: false }), None)
                .with_description("Angle in degrees along which the vector field is projected into a scalar shift."),
        ]
    }

    pub fn create_outputs() -> Vec<Output> {
        vec![
            Output::new("output".to_string(), Value::Image { data: default_image(), change_id: get_id() }, None)
                .with_description("Gradient-mapped image with per-pixel sample position modulated by the field."),
        ]
    }

    pub async fn run(inputs: &mut [Input]) -> Result<OperationResponse, OperationError> {
        let start_time = Instant::now();
        let mut input_errors: Vec<(usize, String)> = vec![];

        let image_converted = convert_input(inputs, 0, ValueType::Image, &mut input_errors);
        let gradient_converted = convert_input(inputs, 1, ValueType::Image, &mut input_errors);
        let field_converted = convert_input(inputs, 2, ValueType::Image, &mut input_errors);
        let strength_converted = convert_input(inputs, 3, ValueType::Decimal, &mut input_errors);
        let angle_converted = convert_input(inputs, 4, ValueType::Decimal, &mut input_errors);

        if !input_errors.is_empty() { return Err(OperationError { input_errors, node_error: None }); }

        let Value::Image { data, change_id: _ } = image_converted.unwrap() else { unreachable!() };
        let Value::Image { data: gradient, change_id: _ } = gradient_converted.unwrap() else { unreachable!() };
        let Value::Image { data: field, change_id: _ } = field_converted.unwrap() else { unreachable!() };
        let Value::Decimal(strength) = strength_converted.unwrap() else { unreachable!() };
        let Value::Decimal(angle) = angle_converted.unwrap() else { unreachable!() };

        let (width, height) = data.dimensions();
        let ch = data.channels() as usize;
        let has_alpha = ch == 2 || ch == 4;
        let colour_ch = if has_alpha { ch - 1 } else { ch };

        let g_w = gradient.width();
        let g_h = gradient.height();
        let g_ch = gradient.channels() as usize;
        let g_y = if g_h == 0 { 0.0 } else { (g_h as f32 - 1.0) * 0.5 };

        let field_w = field.width();
        let field_h = field.height();
        let field_ch = field.channels() as usize;
        // Field image stretch factor — allows a smaller flow image to drive a larger source.
        let fx_scale = if field_w > 0 { field_w as f32 / width.max(1) as f32 } else { 0.0 };
        let fy_scale = if field_h > 0 { field_h as f32 / height.max(1) as f32 } else { 0.0 };

        let angle_rad = angle.to_radians();
        let cos_a = angle_rad.cos();
        let sin_a = angle_rad.sin();

        let img = &*data;
        let grad_img = &*gradient;
        let field_img = &*field;

        let pixels: Vec<f32> = (0..height).into_par_iter().flat_map_iter(move |y| {
            let mut grad_buf = [0.0f32; 4];
            let mut field_buf = [0.0f32; 4];
            (0..width).flat_map(move |x| {
                let src = img.get_pixel(x, y);
                let lum = if colour_ch >= 3 {
                    0.2126 * src[0] + 0.7152 * src[1] + 0.0722 * src[2]
                } else {
                    src[0]
                };
                // Signed field vector (R→x, G→y).
                let delta = if field_ch >= 2 && field_w > 0 && field_h > 0 {
                    field_img.bilinear_sample(x as f32 * fx_scale, y as f32 * fy_scale, &mut field_buf[..field_ch]);
                    let fx = field_buf[0] * 2.0 - 1.0;
                    let fy = field_buf[1] * 2.0 - 1.0;
                    fx * cos_a + fy * sin_a
                } else {
                    0.0
                };
                // Clamp rather than wrap so large field deflections don't
                // snap back to the gradient's opposite end.
                let t = (lum + delta * strength).clamp(0.0, 1.0);
                let gx = t * (g_w.saturating_sub(1).max(1)) as f32;
                grad_img.bilinear_sample(gx, g_y, &mut grad_buf[..g_ch]);
                let (r, g, b, a) = match g_ch {
                    1 => (grad_buf[0], grad_buf[0], grad_buf[0], 1.0),
                    2 => (grad_buf[0], grad_buf[0], grad_buf[0], grad_buf[1]),
                    3 => (grad_buf[0], grad_buf[1], grad_buf[2], 1.0),
                    _ => (grad_buf[0], grad_buf[1], grad_buf[2], grad_buf[3]),
                };
                let alpha_in = if has_alpha { src[ch - 1] } else { 1.0 };
                [r, g, b, a * alpha_in]
            })
        }).collect();

        let output = FloatImage::from_raw(width, height, 4, pixels).unwrap();

        Ok(OperationResponse {
            time: Instant::now().duration_since(start_time),
            responses: vec![
                OutputResponse { value: Value::Image { data: Arc::new(output), change_id: get_id() } },
            ],
        })
    }
}

#[cfg(test)]
#[path = "gradient_dynamic_tests.rs"]
mod tests;
