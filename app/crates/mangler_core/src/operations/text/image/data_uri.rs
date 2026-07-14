//! Base64 data-URI encoding of an image.
//!
//! Downscales the image so its longest side fits `max size`, encodes it as a
//! PNG, and wraps the Base64 bytes in a `data:image/png;base64,…` URI for
//! embedding directly in HTML, CSS, or Markdown.

use crate::get_id;
use crate::input::{Input, InputSettings};
use crate::node_settings::NodeSettings;
use crate::operations::{OperationResponse, OperationError, OutputResponse, default_image, convert_input};
use crate::output::Output;
use crate::value::{Value, ValueType};
use serde::{Deserialize, Serialize};
use std::time::Instant;

use image::{DynamicImage, ImageFormat};
use std::io::Cursor;
use crate::operations::text::encoding::base64_encode;

/// Operation that encodes an image as a Base64 PNG data URI.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpTextImageDataUri {}

impl OpTextImageDataUri {
    /// Returns the node metadata (name, description, help).
    pub fn settings() -> NodeSettings {
        NodeSettings {
            name: "data uri".to_string(),
            description: "Encodes a (downscaled) PNG as a Base64 data URI.".to_string(),
            help: "Encodes a (downscaled) PNG as a Base64 `data:image/png;base64,…` URI for embedding in HTML/CSS/Markdown without a separate file. The image is first shrunk so its longest side is at most `max size` (preserving aspect) to keep the string a reasonable length.\n\nData URIs grow ~33% larger than the raw bytes, so favour a small `max size` for icons and inline previews.".to_string(),
        }
    }

    /// Creates the input ports: the image and the maximum longest-side size.
    pub fn create_inputs() -> Vec<Input> {
        vec![
            Input::new("image".to_string(), Value::Image { data: default_image(), change_id: get_id() }, None, None)
                .with_description("Image to encode."),
            Input::new("max size".to_string(), Value::Integer(512), Some(InputSettings::DragValue { clamp: Some((16.0, 4096.0)), speed: None }), None)
                .with_description("Longest side is downscaled to at most this many pixels (16..4096)."),
        ]
    }

    /// Creates the output port: the data URI text.
    pub fn create_outputs() -> Vec<Output> {
        vec![
            Output::new("uri".to_string(), Value::Text(String::new()), None)
                .with_description("`data:image/png;base64,…` URI for the encoded image."),
        ]
    }

    /// Executes the data-URI encoding.
    pub async fn run(inputs: &mut [Input]) -> Result<OperationResponse, OperationError> {
        let start_time = Instant::now();
        let mut input_errors: Vec<(usize, String)> = vec![];

        let image_converted = convert_input(inputs, 0, ValueType::Image, &mut input_errors);
        let max_size_converted = convert_input(inputs, 1, ValueType::Integer, &mut input_errors);

        if !input_errors.is_empty() { return Err(OperationError { input_errors, node_error: None }); }

        let Value::Image { data, change_id: _ } = image_converted.unwrap() else { unreachable!() };
        let Value::Integer(max_size) = max_size_converted.unwrap() else { unreachable!() };

        let (w, h) = data.dimensions();
        let maxd = max_size.clamp(16, 4096) as u32;
        let img = if w.max(h) > maxd { data.resize_fit_premultiplied(maxd, maxd) } else { (*data).clone() };

        let dynimg = DynamicImage::ImageRgba8(img.to_rgba8());
        let mut buf: Vec<u8> = Vec::new();
        if dynimg.write_to(&mut Cursor::new(&mut buf), ImageFormat::Png).is_err() {
            return Err(OperationError { input_errors: vec![], node_error: Some("Failed to encode image as PNG.".to_string()) });
        }
        let uri = format!("data:image/png;base64,{}", base64_encode(&buf));

        Ok(OperationResponse {
            time: Instant::now().duration_since(start_time),
            responses: vec![OutputResponse { value: Value::Text(uri) }],
        })
    }
}

#[cfg(test)]
#[path = "data_uri_tests.rs"]
mod tests;
