//! Displacement-map-based warp operation.
//!
//! Displaces pixels using a separate displacement map image. Uses
//! [`FloatImage::bilinear_sample`] for channel-agnostic interpolation.

use crate::get_id;
use crate::value::ValueType;
use crate::input::{Input, InputSettings};
use crate::node_settings::NodeSettings;
use crate::operations::{OperationResponse, OperationError, OutputResponse, default_image, convert_input, scale_to_resolution};
use crate::output::Output;
use crate::value::Value;
use crate::float_image::FloatImage;
use rayon::prelude::*;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use std::time::Instant;

/// Displaces image pixels using a separate displacement map.
///
/// The first channel of the displacement map controls horizontal offset and the
/// second channel (if present) controls vertical offset. Values of 0.5 (mid-gray)
/// produce zero displacement; lower and higher values push pixels in opposite
/// directions. The intensity parameter scales the displacement magnitude.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpImageTransformWarp {}

impl OpImageTransformWarp {
    /// Returns the node metadata (name and description) for this operation.
    pub fn settings() -> NodeSettings {
        NodeSettings {
            name: "warp".to_string(),
            description: "Displaces pixels using a displacement map. Red channel offsets X, green channel offsets Y.".to_string(),
            help: "For each output pixel, the displacement map is bilinearly sampled (with coordinate scaling so the map need not match source dimensions). The red channel drives horizontal offset and green drives vertical: 0.5 is zero displacement, with values above/below pushing pixels in opposite directions. The final displacement is (channel - 0.5) * intensity pixels.\n\nSingle-channel maps use the same value for both X and Y. Source sampling is bilinear and coordinates are clamped at image edges (not wrapped), so strong warps near borders can produce streaking. Output channel count matches the source image. Intensity is measured in pixels at a 1024px reference and scales with the image, so the effect looks the same at any resolution.".to_string(),
        }
    }

    /// Creates the default inputs: source image, displacement map, and intensity scalar.
    pub fn create_inputs() -> Vec<Input> {
        vec![
            Input::new("image".to_string(), Value::Image { data: default_image(), change_id: get_id() }, None, None)
                .with_description("Source image to displace."),
            Input::new("displacement".to_string(), Value::Image { data: default_image(), change_id: get_id() }, None, None)
                .with_description("Displacement map: red channel drives X offset, green drives Y offset."),
            Input::new("intensity".to_string(), Value::Decimal(10.0), Some(InputSettings::Slider { range: (0.0, 200.0), step_by: Some(0.1), clamp_to_range: false }), None)
                .with_description("Maximum displacement in pixels at a 1024px reference (scales with image size, so the effect looks the same at any resolution), scaled by the map values."),
        ]
    }

    /// Creates the default outputs: the warped image.
    pub fn create_outputs() -> Vec<Output> {
        vec![
            Output::new("output".to_string(), Value::Image { data: default_image(), change_id: get_id() }, None)
                .with_description("Image with pixels displaced according to the displacement map."),
        ]
    }

    /// Executes the warp by sampling the displacement map for each output pixel.
    pub async fn run(inputs: &mut [Input]) -> Result<OperationResponse, OperationError> {
        let start_time = Instant::now();
        let mut input_errors: Vec<(usize, String)> = vec![];

        let image_converted = convert_input(inputs, 0, ValueType::Image, &mut input_errors);
        let disp_converted = convert_input(inputs, 1, ValueType::Image, &mut input_errors);
        let intensity_converted = convert_input(inputs, 2, ValueType::Decimal, &mut input_errors);

        if !input_errors.is_empty() { return Err(OperationError { input_errors, node_error: None }); }

        let Value::Image { data: src_data, change_id: _ } = image_converted.unwrap() else { unreachable!() };
        let Value::Image { data: disp_data, change_id: _ } = disp_converted.unwrap() else { unreachable!() };
        let Value::Decimal(intensity) = intensity_converted.unwrap() else { unreachable!() };

        let (w, h) = src_data.dimensions();
        // Intensity is authored in reference pixels (at 1024px) and scaled to the
        // actual image, so the same value displaces the same amount relative to
        // the content at any resolution.
        let intensity = scale_to_resolution(intensity, w, h);
        // Output preserves the source image's channel count
        let mut output = FloatImage::new(w, h, src_data.channels());

        let disp_ch = disp_data.channels() as usize;
        let src_ch = src_data.channels() as usize;
        // Premultiply only the sampled source so transparent pixels' hidden
        // colour can't bleed into interpolated edge pixels; the displacement map
        // is guidance data and must stay untouched.
        let premul = src_data.has_alpha();
        let src_img = if premul { Arc::new(src_data.premultiply_alpha()) } else { Arc::clone(&src_data) };
        let src = &*src_img;
        let disp = &*disp_data;
        let disp_w = disp.width() as f32;
        let disp_h = disp.height() as f32;
        let row_len = (w as usize * src_ch).max(1);

        output.as_raw_mut().par_chunks_mut(row_len).enumerate().for_each(|(y, row)| {
            // Temporary buffers for bilinear sampling
            let mut dp = vec![0.0f32; disp_ch];
            let mut sp = vec![0.0f32; src_ch];
            let dy = y as f32 * disp_h / h as f32;
            for x in 0..w as usize {
                // Sample displacement map, mapping output coords to displacement map coords
                // to handle mismatched dimensions between source and displacement
                let dx = x as f32 * disp_w / w as f32;
                disp.bilinear_sample(dx, dy, &mut dp);

                // Map 0.0..1.0 to -0.5..0.5, then multiply by intensity.
                // For 1-channel displacement, use the same value for both X and Y.
                let offset_x = (dp[0] - 0.5) * intensity;
                let offset_y = if disp_ch >= 2 { (dp[1] - 0.5) * intensity } else { offset_x };

                let sx = x as f32 + offset_x;
                let sy = y as f32 + offset_y;

                // Sample source image at displaced coordinates
                src.bilinear_sample(sx, sy, &mut sp);
                row[x * src_ch..(x + 1) * src_ch].copy_from_slice(&sp);
            }
        });

        // Back to straight alpha for downstream nodes / display.
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
#[path = "warp_tests.rs"]
mod tests;
