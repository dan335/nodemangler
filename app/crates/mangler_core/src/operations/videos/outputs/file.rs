//! Video-to-file output operation.
//!
//! This node carries the configuration for a video render (output path,
//! format, fps, duration). Clicking Render in the GUI sends a
//! `ChangeGraphMessage::StartRender { output_node_id }` addressed to this
//! node; the engine then spawns a separate task that drives the actual
//! encoding.
//!
//! The `run()` executed on the live engine is a trivial passthrough: it
//! copies the `image` input to the `last_frame` output so node thumbnails
//! keep updating interactively. The render task reads the same inputs
//! from a detached snapshot; the live graph continues normally.

use crate::get_id;
use crate::input::{FileDialogType, Input, InputSettings};
use crate::node_settings::NodeSettings;
use crate::operations::{default_image, OperationError, OperationResponse, OutputResponse};
use crate::output::Output;
use crate::value::{Value, VideoCodec, VideoContainer};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Instant;

/// Operation that writes a video file.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpVideoOutputFile {}

impl OpVideoOutputFile {
    pub fn settings() -> NodeSettings {
        NodeSettings {
            name: "video to file".to_string(),
            description: "Renders the graph frame-by-frame into a video file. Use the Render button in the inspector to start.".to_string(),
        }
    }

    pub fn create_inputs() -> Vec<Input> {
        vec![
            Input::new(
                "image".to_string(),
                Value::Image { data: default_image(), change_id: get_id() },
                None,
                None,
            ),
            Input::new(
                "path".to_string(),
                Value::Path(PathBuf::new()),
                Some(InputSettings::Path {
                    extension_filter: vec![
                        "mp4".to_string(),
                        "mov".to_string(),
                        "mkv".to_string(),
                        "webm".to_string(),
                    ],
                    set_directory: None,
                    set_file_name: None,
                    set_title: Some("video output".to_string()),
                    file_dialog_type: FileDialogType::SaveFile,
                }),
                None,
            ),
            Input::new(
                "container".to_string(),
                Value::VideoContainer(VideoContainer::Mp4),
                None,
                None,
            ),
            Input::new(
                "codec".to_string(),
                Value::VideoCodec(VideoCodec::H264),
                None,
                None,
            ),
            Input::new(
                "fps".to_string(),
                Value::Decimal(30.0),
                Some(InputSettings::DragValue {
                    clamp: Some((1.0, 240.0)),
                    speed: Some(1.0),
                }),
                None,
            ),
            Input::new(
                "duration".to_string(),
                Value::Decimal(10.0),
                Some(InputSettings::DragValue {
                    clamp: Some((0.1, 86400.0)),
                    speed: Some(0.1),
                }),
                None,
            ),
        ]
    }

    pub fn create_outputs() -> Vec<Output> {
        vec![
            // last_frame is slot 0 so the node preview uses the decoded
            // image as its thumbnail (program.rs keys the node thumbnail
            // off output_index == 0).
            Output::new(
                "last_frame".to_string(),
                Value::Image { data: default_image(), change_id: get_id() },
                None,
            ),
            Output::new("rendered_path".to_string(), Value::Path(PathBuf::new()), None),
        ]
    }

    /// Interactive passthrough: copy the incoming `image` to `last_frame`
    /// so the node's thumbnail updates while the user scrubs. Never touches
    /// the encoder — that's the render task's job.
    pub async fn run(inputs: &mut [Input]) -> Result<OperationResponse, OperationError> {
        let start_time = Instant::now();

        let (data, change_id) = match inputs.get(0).map(|i| &i.value) {
            Some(Value::Image { data, change_id }) => (Arc::clone(data), change_id.clone()),
            _ => (default_image(), get_id()),
        };

        // Read `path` to echo on `rendered_path` until the first render finishes.
        // The engine overwrites `rendered_path` itself on RenderFinished via
        // its normal message flow — but having a reasonable default here
        // keeps the output slot non-empty in the meantime.
        let path = match inputs.get(1).map(|i| &i.value) {
            Some(Value::Path(p)) => p.clone(),
            _ => PathBuf::new(),
        };

        Ok(OperationResponse {
            time: Instant::now().duration_since(start_time),
            responses: vec![
                OutputResponse { value: Value::Image { data, change_id } },
                OutputResponse { value: Value::Path(path) },
            ],
        })
    }
}
