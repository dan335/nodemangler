//! Image-from-clipboard input operation.
//!
//! Reads image data from the system clipboard using the `arboard` crate and
//! outputs the image along with its dimensions. The clipboard RGBA bytes are
//! converted to a 4-channel `FloatImage`.
//!
//! ## Why this node stores its pixels
//! Every other image source in this directory is a *reference* the graph file
//! can keep — a path, a folder, a URL — so reopening the graph re-reads it. The
//! clipboard is not: it belongs to the OS session, and by the next time the
//! graph is opened it holds something else, or nothing. So the captured pixels
//! are embedded in the graph file (base64 PNG on
//! [`Input::embedded_image`](crate::input::Input::embedded_image)) and the node
//! restores them on load instead of re-reading the clipboard.
//!
//! That restore is the reason the capture control is a **momentary button**
//! rather than the `Value::Trigger` this node used to take. A trigger's
//! fingerprint is constant, so `run` cannot tell a fresh press apart from "the
//! graph just loaded and everything is dirty" — and re-sampling the clipboard
//! on load is precisely the behaviour that lost the image in the first place,
//! or worse, would silently swap it for whatever the user had copied since. A
//! `Bool` + [`InputSettings::Button`] arrives as a one-shot `true` that this
//! run consumes, the same pattern the output nodes' save button uses. Both
//! render identically in the settings panel.

use crate::float_image::FloatImage;
use crate::get_id;
use crate::input::{Input, InputSettings};
use crate::node_settings::NodeSettings;
use crate::operations::images::inputs::{decode_png_base64, encode_png_base64};
use crate::operations::{OperationResponse, OperationError, OutputResponse, default_image};
use crate::output::Output;
use crate::value::Value;
use arboard::Clipboard;
use image::{ImageBuffer, RgbaImage};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use std::time::Instant;

/// Index of the capture button among this node's inputs. The stored pixels hang
/// off the same input.
const CAPTURE: usize = 0;

/// Operation that grabs an image from the system clipboard.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpImageInputClipboard {}

impl OpImageInputClipboard {
    /// Returns the node metadata (name and description) for this operation.
    pub fn settings() -> NodeSettings {
        NodeSettings {
            name: "from clipboard".to_string(),
            description: "Grabs an image from the clipboard.".to_string(),
            help: "Reads the current system clipboard contents as raw RGBA bytes via arboard and wraps them in a 4-channel FloatImage. The node captures once when you add it, and again each time you press the button — so copying a new image elsewhere and clicking refreshes the output.\n\nThe captured pixels are **saved inside the graph file** as a base64 PNG, and restored when the graph is reopened. Every other image source stores only a reference (a path, a folder, a URL) and re-reads it, but the clipboard belongs to the OS session and is gone by the next one, so there is nothing else to re-read from. Reopening a graph therefore shows the picture you captured, never whatever happens to be on the clipboard that day.\n\nTwo consequences worth knowing. The graph file grows by roughly the PNG size of the image — a full-screen capture can add several megabytes, so prefer `from file` for anything large or reused across graphs. And the round-trip is 8 bits per channel, which is exact for clipboard content (it arrives as RGBA8) but would clip a higher-range image.\n\nThe node reports an error when the clipboard is empty or does not hold an image (for example when it only contains text).".to_string(),
        }
    }

    /// Creates the input definitions: the momentary capture button.
    pub fn create_inputs() -> Vec<Input> {
        vec![
            Input::new(
                "copy from clipboard".to_string(),
                Value::Bool(false),
                Some(InputSettings::Button),
                None,
            )
            .with_description(
                "Press to capture the current clipboard image. The captured pixels are stored in \
                 the graph file, so reopening the graph shows them again.",
            ),
        ]
    }

    /// Creates the output definitions: the grabbed image, its width, and its height.
    pub fn create_outputs() -> Vec<Output> {
        vec![
            Output::new("output".to_string(), Value::Image { data:default_image(), change_id:get_id() }, None)
                .with_description("The image decoded from the clipboard."),
            Output::new("width".to_string(), Value::Integer(1), None)
                .with_description("Width of the clipboard image in pixels."),
            Output::new("height".to_string(), Value::Integer(1), None)
                .with_description("Height of the clipboard image in pixels."),
        ]
    }

    /// Executes the operation: captures the clipboard, or restores the pixels a
    /// previous capture stored in the graph file.
    pub async fn run(inputs: &mut [Input]) -> Result<OperationResponse, OperationError> {
        let start_time = Instant::now();

        let pressed = matches!(inputs.get(CAPTURE).map(|i| &i.value), Some(Value::Bool(true)));
        // Consume the pulse so the next run is a restore, not a re-capture.
        // Skipped when driven upstream — the value is not ours to reset.
        if let Some(input) = inputs.get_mut(CAPTURE) {
            if input.connection.is_none() {
                input.value = Value::Bool(false);
            }
        }
        let stored = inputs.get(CAPTURE).and_then(|i| i.embedded_image.clone());

        let image = match stored.as_deref() {
            // Reopened from a saved graph: the pixels travel in the file.
            Some(encoded) if !pressed => decode_png_base64(encoded).map_err(node_error)?,
            // An explicit press, or a node that has never captured — the latter
            // so that dropping the node in still grabs what you just copied.
            _ => {
                let Some(image) = read_clipboard_image() else {
                    return Err(node_error(
                        "Error grabbing clipboard or clipboard is empty.".to_string(),
                    ));
                };
                let encoded = encode_png_base64(&image).map_err(node_error)?;
                if let Some(input) = inputs.get_mut(CAPTURE) {
                    input.embedded_image = Some(encoded);
                }
                image
            }
        };

        let (width, height) = image.dimensions();
        Ok(OperationResponse {
            time: Instant::now().duration_since(start_time),
            responses: vec![
                OutputResponse {
                    value: Value::Image { data: Arc::new(image), change_id: get_id() },
                },
                OutputResponse { value: Value::Integer(width as i32) },
                OutputResponse { value: Value::Integer(height as i32) },
            ],
        })
    }
}

/// The system clipboard's current image, when it holds one.
fn read_clipboard_image() -> Option<FloatImage> {
    let mut clipboard = Clipboard::new().ok()?;
    let image_bytes = clipboard.get_image().ok()?;
    // Convert raw clipboard bytes into an RgbaImage buffer.
    let rgba: RgbaImage = ImageBuffer::from_raw(
        image_bytes.width.try_into().ok()?,
        image_bytes.height.try_into().ok()?,
        image_bytes.bytes.into_owned(),
    )?;
    Some(FloatImage::from_dynamic(&image::DynamicImage::ImageRgba8(rgba)))
}

fn node_error(message: String) -> OperationError {
    OperationError { input_errors: vec![], node_error: Some(message) }
}

#[cfg(test)]
#[path = "clipboard_tests.rs"]
mod tests;
