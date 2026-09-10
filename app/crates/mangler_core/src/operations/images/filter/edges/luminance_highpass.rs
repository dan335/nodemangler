//! Luminance-only highpass filter.
//!
//! Same idea as `highpass`, but only the luminance component is high-passed;
//! chroma is preserved. Useful for sharpening without introducing the colored
//! ringing a naïve per-channel highpass produces.

use crate::float_image::FloatImage;
use crate::get_id;
use crate::input::{Input, InputSettings};
use crate::node_settings::NodeSettings;
use crate::operations::images::blur::blur::gaussian_blur_image;
use crate::convert_inputs;
use crate::operations::{OperationResponse, OperationError, OutputResponse, scale_to_resolution, image_input, image_output};
use crate::output::Output;
use crate::value::Value;
use rayon::prelude::*;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use std::time::Instant;

/// Luminance-only highpass filter.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpImageAdjustmentLuminanceHighpass {}

impl OpImageAdjustmentLuminanceHighpass {
    pub fn settings() -> NodeSettings {
        NodeSettings {
            name: "luminance highpass".to_string(),
            description: "Highpass applied only to the luminance channel. Chroma is preserved to avoid colored sharpening halos.".to_string(),
            help: "Gaussian-blurs the image, computes the Rec. 709 luminance delta `lum(src) - lum(blur)`, and adds that scalar delta uniformly to each RGB channel. This sharpens brightness variation without shifting hue, avoiding the colored ringing a naive per-channel highpass produces near saturated edges.\n\nSingle-channel or gray+alpha inputs fall back to the plain highpass formulation. Unlike `highpass`, the output is the sharpened image itself (ready to use), not a mid-grey-centered detail layer.".to_string(),
        }
    }

    pub fn create_inputs() -> Vec<Input> {
        vec![
            image_input("image")
                .with_description("Source image whose luminance is sharpened without shifting chroma."),
            Input::new("radius".to_string(), Value::Decimal(4.0), Some(InputSettings::DragValue { speed: None, clamp: Some((0.0, 256.0)) }), None)
                .with_description("Blur radius in pixels at a 1024px reference (scales with image size), for the low-pass component subtracted from luminance."),
        ]
    }

    pub fn create_outputs() -> Vec<Output> {
        vec![
            image_output("output")
                .with_description("Image with the luminance high-pass delta added back into each color channel."),
        ]
    }

    pub async fn run(inputs: &mut [Input]) -> Result<OperationResponse, OperationError> {
        let start_time = Instant::now();

        convert_inputs! { inputs;
            Image(data) = 0,
            Decimal(radius) = 1,
        }

        let (width, height) = data.dimensions();
        let ch = data.channels() as usize;

        // Radius is authored in reference pixels (at 1024px) and scaled to the
        // actual image, so the sharpening scale looks the same at any resolution.
        let radius = scale_to_resolution(radius.max(0.0), width, height);
        let blurred = gaussian_blur_image(&data, radius);

        let mut output = FloatImage::new(width, height, data.channels());

        // Single-channel inputs collapse to the plain highpass case.
        let src_img = &*data;
        if ch < 3 {
            output.par_enumerate_pixels_mut().for_each(|(x, y, out_px)| {
                let mut buf = [0.0f32; 4];
                let src = src_img.get_pixel(x, y);
                let blur = blurred.get_pixel(x, y);
                buf[0] = (src[0] - blur[0] + 0.5).clamp(0.0, 1.0);
                if ch == 2 { buf[1] = src[1]; }
                out_px.copy_from_slice(&buf[..ch]);
            });
        } else {
            // For RGB(A), compute luminance on source + blur, take the
            // delta, and add it to each colour channel uniformly — this
            // sharpens brightness without shifting hue.
            output.par_enumerate_pixels_mut().for_each(|(x, y, out_px)| {
                let mut buf = [0.0f32; 4];
                let src = src_img.get_pixel(x, y);
                let blur = blurred.get_pixel(x, y);
                let lum_src = crate::luma::rec709(src[0], src[1], src[2]);
                let lum_blur = crate::luma::rec709(blur[0], blur[1], blur[2]);
                let delta = lum_src - lum_blur;
                buf[0] = (src[0] + delta).clamp(0.0, 1.0);
                buf[1] = (src[1] + delta).clamp(0.0, 1.0);
                buf[2] = (src[2] + delta).clamp(0.0, 1.0);
                if ch == 4 { buf[3] = src[3]; }
                out_px.copy_from_slice(&buf[..ch]);
            });
        }

        Ok(OperationResponse {
            time: Instant::now().duration_since(start_time),
            responses: vec![
                OutputResponse { value: Value::Image { data: Arc::new(output), change_id: get_id() } },
            ],
        })
    }
}

#[cfg(test)]
#[path = "luminance_highpass_tests.rs"]
mod tests;
