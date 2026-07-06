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
use crate::operations::{OperationResponse, OperationError, OutputResponse, default_image, convert_input, scale_to_resolution};
use crate::output::Output;
use crate::value::{Value, ValueType};
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
            Input::new("image".to_string(), Value::Image { data: default_image(), change_id: get_id() }, None, None)
                .with_description("Source image whose luminance is sharpened without shifting chroma."),
            Input::new("radius".to_string(), Value::Decimal(4.0), Some(InputSettings::DragValue { speed: None, clamp: Some((0.0, 256.0)) }), None)
                .with_description("Blur radius in pixels at a 1024px reference (scales with image size), for the low-pass component subtracted from luminance."),
        ]
    }

    pub fn create_outputs() -> Vec<Output> {
        vec![
            Output::new("output".to_string(), Value::Image { data: default_image(), change_id: get_id() }, None)
                .with_description("Image with the luminance high-pass delta added back into each color channel."),
        ]
    }

    pub async fn run(inputs: &mut [Input]) -> Result<OperationResponse, OperationError> {
        let start_time = Instant::now();
        let mut input_errors: Vec<(usize, String)> = vec![];

        let image_converted = convert_input(inputs, 0, ValueType::Image, &mut input_errors);
        let radius_converted = convert_input(inputs, 1, ValueType::Decimal, &mut input_errors);

        if !input_errors.is_empty() { return Err(OperationError { input_errors, node_error: None }); }

        let Value::Image { data, change_id: _ } = image_converted.unwrap() else { unreachable!() };
        let Value::Decimal(radius) = radius_converted.unwrap() else { unreachable!() };

        let (width, height) = data.dimensions();
        let ch = data.channels() as usize;

        // Radius is authored in reference pixels (at 1024px) and scaled to the
        // actual image, so the sharpening scale looks the same at any resolution.
        let radius = scale_to_resolution(radius.max(0.0), width, height);
        let blurred = gaussian_blur_image(&data, radius);

        let mut output = FloatImage::new(width, height, data.channels());

        // Single-channel inputs collapse to the plain highpass case.
        if ch < 3 {
            let mut buf = [0.0f32; 4];
            for y in 0..height {
                for x in 0..width {
                    let src = data.get_pixel(x, y);
                    let blur = blurred.get_pixel(x, y);
                    buf[0] = (src[0] - blur[0] + 0.5).clamp(0.0, 1.0);
                    if ch == 2 { buf[1] = src[1]; }
                    output.put_pixel(x, y, &buf[..ch]);
                }
            }
        } else {
            // For RGB(A), compute luminance on source + blur, take the
            // delta, and add it to each colour channel uniformly — this
            // sharpens brightness without shifting hue.
            let mut buf = [0.0f32; 4];
            for y in 0..height {
                for x in 0..width {
                    let src = data.get_pixel(x, y);
                    let blur = blurred.get_pixel(x, y);
                    let lum_src = 0.2126 * src[0] + 0.7152 * src[1] + 0.0722 * src[2];
                    let lum_blur = 0.2126 * blur[0] + 0.7152 * blur[1] + 0.0722 * blur[2];
                    let delta = lum_src - lum_blur;
                    buf[0] = (src[0] + delta).clamp(0.0, 1.0);
                    buf[1] = (src[1] + delta).clamp(0.0, 1.0);
                    buf[2] = (src[2] + delta).clamp(0.0, 1.0);
                    if ch == 4 { buf[3] = src[3]; }
                    output.put_pixel(x, y, &buf[..ch]);
                }
            }
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
