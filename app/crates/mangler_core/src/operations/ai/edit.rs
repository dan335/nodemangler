/// AI image editing (inpainting) operation.
///
/// Sends an image, mask, and text prompt to the OpenAI DALL-E 2 image edits API
/// and returns the edited image along with its dimensions.
///
/// The mask input is treated as a grayscale brightness mask: black areas (dark pixels)
/// become transparent and mark where edits should occur. White areas are preserved.

use crate::get_id;
use crate::float_image::FloatImage;
use crate::input::{Input, InputSettings};
use crate::node_settings::NodeSettings;
use crate::operations::{OperationResponse, OperationError, OutputResponse, default_image, convert_input};
use crate::output::Output;
use crate::value::{Value, ValueType};
use image::RgbaImage;
use serde::{Deserialize, Serialize};
use std::io::Cursor;
use std::sync::Arc;
use std::time::Instant;

use super::shared;

/// OpenAI image edits API endpoint.
const OPENAI_EDITS_URL: &str = "https://api.openai.com/v1/images/edits";

/// Operation that edits an image using a text prompt via the OpenAI DALL-E 2 API.
///
/// Inputs: image, mask, prompt, size.
/// Outputs: edited image, width, height.
///
/// The mask is a brightness mask: black = edit here, white = keep.
/// Internally converted to a PNG with alpha transparency for the API.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpAiEdit {}

impl OpAiEdit {
    /// Returns node metadata.
    pub fn settings() -> NodeSettings {
        NodeSettings {
            name: "ai edit".to_string(),
            description: "Edits an image using a text prompt via OpenAI DALL-E 2. Black areas in the mask are edited.".to_string(),
        }
    }

    /// Creates the input definitions.
    pub fn create_inputs() -> Vec<Input> {
        vec![
            Input::new("image".to_string(), Value::Image { data: default_image(), change_id: get_id() }, None, None),
            Input::new("mask".to_string(), Value::Image { data: default_image(), change_id: get_id() }, None, None),
            Input::new("prompt".to_string(), Value::Text(String::new()), Some(InputSettings::MultiLineText), None),
            Input::new("size".to_string(), Value::Text("1024x1024".to_string()), Some(InputSettings::Dropdown {
                options: vec!["1024x1024".to_string(), "512x512".to_string(), "256x256".to_string()],
            }), None),
        ]
    }

    /// Creates the output definitions.
    pub fn create_outputs() -> Vec<Output> {
        vec![
            Output::new("image".to_string(), Value::Image { data: default_image(), change_id: get_id() }, None),
            Output::new("width".to_string(), Value::Integer(0), None),
            Output::new("height".to_string(), Value::Integer(0), None),
        ]
    }

    /// Convert a mask FloatImage to PNG bytes with proper alpha transparency.
    ///
    /// The mask is treated as a brightness mask: the average brightness of each pixel
    /// determines the alpha value. Black (0.0) → alpha 0 (transparent, will be edited).
    /// White (1.0) → alpha 255 (opaque, preserved).
    fn mask_to_png_bytes(mask: &FloatImage) -> Result<Vec<u8>, String> {
        let w = mask.width();
        let h = mask.height();
        let ch = mask.channels() as usize;
        let raw = mask.as_raw();
        let mut out = RgbaImage::new(w, h);

        for (i, pixel) in out.pixels_mut().enumerate() {
            let offset = i * ch;
            let src = &raw[offset..offset + ch];

            // Compute brightness as the average of color channels (excluding alpha).
            let brightness = match ch {
                1 => src[0],
                2 => src[0], // grayscale+alpha: use luminance, ignore source alpha
                3 => (src[0] + src[1] + src[2]) / 3.0,
                4 => (src[0] + src[1] + src[2]) / 3.0, // use RGB brightness, ignore source alpha
                _ => 1.0,
            };

            // Brightness → alpha: white (1.0) = opaque (keep), black (0.0) = transparent (edit).
            let alpha = (brightness.clamp(0.0, 1.0) * 255.0) as u8;
            // RGB channels are white; only the alpha channel matters for the mask.
            *pixel = image::Rgba([255, 255, 255, alpha]);
        }

        let mut buf = Cursor::new(Vec::new());
        image::DynamicImage::ImageRgba8(out)
            .write_to(&mut buf, image::ImageFormat::Png)
            .map_err(|e| format!("Failed to encode mask as PNG: {}", e))?;
        Ok(buf.into_inner())
    }

    /// Builds the multipart form for the OpenAI image edits endpoint.
    ///
    /// DALL-E 2 requires `response_format: "b64_json"` to get base64 instead of URLs.
    /// The mask PNG has transparent (alpha=0) areas where edits should occur.
    pub fn build_multipart_form(
        image_png_bytes: Vec<u8>,
        mask_png_bytes: Option<Vec<u8>>,
        prompt: &str,
        size: &str,
    ) -> reqwest::multipart::Form {
        let image_part = reqwest::multipart::Part::bytes(image_png_bytes)
            .file_name("image.png")
            .mime_str("image/png")
            .unwrap();

        let mut form = reqwest::multipart::Form::new()
            .part("image", image_part)
            .text("prompt", prompt.to_string())
            .text("model", "dall-e-2".to_string())
            .text("size", size.to_string())
            .text("response_format", "b64_json".to_string());

        // Attach the mask if provided.
        if let Some(mask_bytes) = mask_png_bytes {
            let mask_part = reqwest::multipart::Part::bytes(mask_bytes)
                .file_name("mask.png")
                .mime_str("image/png")
                .unwrap();
            form = form.part("mask", mask_part);
        }

        form
    }

    /// Executes the operation: sends image + mask + prompt to OpenAI edits endpoint.
    pub async fn run(inputs: &mut [Input]) -> Result<OperationResponse, OperationError> {
        let start_time = Instant::now();
        let mut input_errors: Vec<(usize, String)> = vec![];

        // Convert inputs: 0=image, 1=mask, 2=prompt, 3=size.
        let image_converted = convert_input(inputs, 0, ValueType::Image, &mut input_errors);
        let mask_converted = convert_input(inputs, 1, ValueType::Image, &mut input_errors);
        let prompt_converted = convert_input(inputs, 2, ValueType::Text, &mut input_errors);
        let size_converted = convert_input(inputs, 3, ValueType::Text, &mut input_errors);

        if !input_errors.is_empty() {
            return Err(OperationError { input_errors, node_error: None });
        }

        let Value::Image { data: image_data, .. } = image_converted.unwrap() else { unreachable!() };
        let Value::Image { data: mask_data, .. } = mask_converted.unwrap() else { unreachable!() };
        let Value::Text(prompt) = prompt_converted.unwrap() else { unreachable!() };
        let Value::Text(size) = size_converted.unwrap() else { unreachable!() };

        // Validate prompt is not empty.
        if prompt.trim().is_empty() {
            return Err(OperationError {
                input_errors: vec![(2, "Prompt cannot be empty.".to_string())],
                node_error: None,
            });
        }

        // Resolve API key from environment.
        let api_key = match shared::resolve_api_key("OPENAI_API_KEY") {
            Ok(key) => key,
            Err(msg) => return Err(OperationError { input_errors: vec![], node_error: Some(msg) }),
        };

        // Convert input image to PNG bytes for the multipart form.
        let image_png_bytes = match shared::float_image_to_png_bytes(&image_data) {
            Ok(bytes) => bytes,
            Err(msg) => return Err(OperationError { input_errors: vec![], node_error: Some(msg) }),
        };

        // Validate mask dimensions match the input image.
        let mask_is_default = mask_data.width() == 1 && mask_data.height() == 1;
        if !mask_is_default
            && (mask_data.width() != image_data.width() || mask_data.height() != image_data.height())
        {
            return Err(OperationError {
                input_errors: vec![(1, format!(
                    "Mask size ({}x{}) must match image size ({}x{}).",
                    mask_data.width(), mask_data.height(),
                    image_data.width(), image_data.height()
                ))],
                node_error: None,
            });
        }

        // Convert the mask to PNG with alpha transparency if a real mask was provided.
        // Skip the default 1x1 placeholder image.
        let mask_png_bytes = if !mask_is_default {
            match Self::mask_to_png_bytes(&mask_data) {
                Ok(bytes) => Some(bytes),
                Err(msg) => return Err(OperationError { input_errors: vec![], node_error: Some(msg) }),
            }
        } else {
            None
        };

        // Check cost limit before making the API call.
        if let Err(msg) = shared::check_cost_limit() {
            return Err(OperationError { input_errors: vec![], node_error: Some(msg) });
        }

        // Build and send the request.
        let form = Self::build_multipart_form(image_png_bytes, mask_png_bytes, &prompt, &size);
        let json = match shared::make_ai_multipart_request(OPENAI_EDITS_URL, &api_key, form).await {
            Ok(json) => json,
            Err(msg) => return Err(OperationError { input_errors: vec![], node_error: Some(msg) }),
        };

        // Parse the response.
        let (image, width, height, _) = match shared::parse_openai_image_response(&json) {
            Ok(result) => result,
            Err(msg) => return Err(OperationError { input_errors: vec![], node_error: Some(msg) }),
        };

        // Estimate and record cost.
        let cost = shared::estimate_cost_from_response(&json, "dall-e-2", &size, "standard");
        shared::add_session_cost(cost);

        Ok(OperationResponse {
            ai_cost_usd: Some(cost),
            time: Instant::now().duration_since(start_time),
            responses: vec![
                OutputResponse { value: Value::Image { data: Arc::new(image), change_id: get_id() } },
                OutputResponse { value: Value::Integer(width) },
                OutputResponse { value: Value::Integer(height) },
            ],
        })
    }
}

#[cfg(test)]
#[path = "edit_tests.rs"]
mod tests;
