//! Camera RAW input operation.
//!
//! Develops a camera RAW file (Canon CR3/CR2, Nikon NEF, Sony ARW, Adobe DNG,
//! …) into an image, exposing the controls that have to happen *inside* the
//! demosaic pipeline while the sensor data still exists.
//!
//! Everything that can equally well be done downstream is deliberately left to
//! the existing adjustment nodes — there is no temperature slider here because
//! `white balance` already implements Kelvin + tint with Bradford adaptation,
//! and no denoise or lens correction because those nodes exist too.
//!
//! The plain `from file` node also opens RAW files, using the camera's as-shot
//! settings; use this node when you want to change them.

use crate::get_id;
use crate::input::{Input, InputSettings};
use crate::node_settings::NodeSettings;
use crate::operations::{convert_input, default_image, OperationError, OperationResponse, OutputResponse};
use crate::output::Output;
use crate::value::{Value, ValueType};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Instant;

use super::raw_decode::{decode_raw, RawOptions, RawWhiteBalance};

/// Input index of the `path` input (a positional contract with `run`).
pub const PATH: usize = 0;
/// Input index of the `white balance` input.
pub const WHITE_BALANCE: usize = 1;
/// Input index of the `output` encoding input.
pub const OUTPUT_ENCODING: usize = 2;
/// Input index of the `demosaic` input.
pub const DEMOSAIC: usize = 3;
/// Input index of the `exposure` input.
pub const EXPOSURE: usize = 4;
/// Input index of the `max size` input.
pub const MAX_SIZE: usize = 5;

/// Default longest-edge cap, in pixels.
///
/// A 26-megapixel raw is ~312 MB as 3-channel f32, and every downstream node
/// that produces an image allocates another buffer, so a full-resolution
/// default would make a modest edit chain consume several gigabytes. 4096 is
/// large enough to edit against and cheap enough to stay interactive; set 0 for
/// the final full-resolution render.
pub const DEFAULT_MAX_SIZE: i32 = 4096;

/// Operation that develops a camera RAW file into an image.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpImageInputRaw {}

impl OpImageInputRaw {
    /// Returns the node metadata (name, description and help text).
    pub fn settings() -> NodeSettings {
        NodeSettings {
            name: "from raw".to_string(),
            description: "Develops a camera raw file.".to_string(),
            help: "Decodes a camera raw file — Canon CR3/CR2, Nikon NEF, Sony ARW, Fujifilm RAF, Adobe DNG and many others — into an image, using rawler's development pipeline: rescale, demosaic, crop, white balance, camera-to-sRGB colour matrix, and sRGB gamma. The file's EXIF orientation is applied, so portrait shots come out upright.\n\nThe plain 'from file' node opens the same formats using the camera's as-shot settings; use this node when you want to change how the raw is developed.\n\n'white balance' picks the multipliers applied to the sensor data before demosaic — 'as shot' matches the camera's own JPEG, 'camera neutral' removes the creative decision so you can set temperature with a downstream white balance node, and 'none' leaves the raw sensor ratios (green-dominant; for diagnostics).\n\n'output' chooses sRGB-encoded (matching every other image in the graph) or scene-linear, which is the correct source for tone map, bloom and physically-correct blending. Highlights above 1.0 survive in both, so recovery nodes have headroom to work with.\n\n'demosaic' off returns the single-channel sensor mosaic at full resolution, for sensor-pattern and debugging work.\n\n'max size' caps the longest edge. Raw files are enormous — a 26-megapixel frame is about 312 MB per copy, and every node downstream allocates another — so leave this at 4096 while composing and set it to 0 for the final render.\n\nA four-colour sensor's fourth layer is dropped rather than passed through as alpha. Errors if the file cannot be read or the camera is not supported.".to_string(),
        }
    }

    /// Creates the input definitions.
    pub fn create_inputs() -> Vec<Input> {
        vec![
            Input::new("path".to_string(), Value::Path(PathBuf::new()), Some(InputSettings::Path {
                // Raw-only: this node cannot open a PNG, so offering one would lie.
                extension_filter: ValueType::raw_file_extensions(),
                set_directory: None,
                set_file_name: None,
                set_title: Some("raw photo".to_string()),
                file_dialog_type: crate::input::FileDialogType::PickFile,
            }), None)
                .with_description("Path to a camera raw file to develop."),
            Input::new("white balance".to_string(), Value::Text("as shot".to_string()), Some(InputSettings::Dropdown {
                options: vec!["as shot".to_string(), "camera neutral".to_string(), "none".to_string()],
            }), None)
                .with_description("Which white-balance multipliers to develop with. Applied to the sensor data before demosaic, so this cannot be reproduced downstream."),
            Input::new("output".to_string(), Value::Text("srgb".to_string()), Some(InputSettings::Dropdown {
                options: vec!["srgb".to_string(), "linear".to_string()],
            }), None)
                .with_description("Encode the result as sRGB (matching the rest of the graph) or leave it scene-linear for tone mapping and physically-correct blending."),
            Input::new("demosaic".to_string(), Value::Bool(true), None, None)
                .with_description("Reconstruct full colour from the sensor mosaic. Off returns the raw single-channel CFA pattern."),
            Input::new("exposure".to_string(), Value::Decimal(0.0), Some(InputSettings::Slider { range: (-5.0, 5.0), step_by: Some(0.01), clamp_to_range: false }), None)
                .with_description("Exposure adjustment in stops, applied in linear light before encoding, so it can recover highlights that encoding would otherwise flatten."),
            Input::new("max size".to_string(), Value::Integer(DEFAULT_MAX_SIZE), Some(InputSettings::DragValue { clamp: Some((0.0, 16384.0)), speed: None }), None)
                .with_description("Cap the longest edge, in pixels. 0 loads at full resolution — expect hundreds of megabytes per node."),
        ]
    }

    /// Creates the output definitions.
    pub fn create_outputs() -> Vec<Output> {
        vec![
            Output::new("output".to_string(), Value::Image { data: default_image(), change_id: get_id() }, None)
                .with_description("The developed image."),
            Output::new("width".to_string(), Value::Integer(1), None)
                .with_description("Width of the developed image in pixels."),
            Output::new("height".to_string(), Value::Integer(1), None)
                .with_description("Height of the developed image in pixels."),
        ]
    }

    /// Executes the operation: develops the raw file at the given path.
    pub async fn run(inputs: &mut [Input]) -> Result<OperationResponse, OperationError> {
        let start_time = Instant::now();
        let mut input_errors: Vec<(usize, String)> = vec![];

        // convert inputs
        let path_converted = convert_input(inputs, PATH, ValueType::Path, &mut input_errors);
        let white_balance_converted = convert_input(inputs, WHITE_BALANCE, ValueType::Text, &mut input_errors);
        let output_converted = convert_input(inputs, OUTPUT_ENCODING, ValueType::Text, &mut input_errors);
        let demosaic_converted = convert_input(inputs, DEMOSAIC, ValueType::Bool, &mut input_errors);
        let exposure_converted = convert_input(inputs, EXPOSURE, ValueType::Decimal, &mut input_errors);
        let max_size_converted = convert_input(inputs, MAX_SIZE, ValueType::Integer, &mut input_errors);

        // return if error
        if !input_errors.is_empty() { return Err(OperationError { input_errors, node_error: None }); }

        // get values
        let Value::Path(path) = path_converted.unwrap() else { unreachable!() };
        let Value::Text(white_balance) = white_balance_converted.unwrap() else { unreachable!() };
        let Value::Text(output_encoding) = output_converted.unwrap() else { unreachable!() };
        let Value::Bool(demosaic) = demosaic_converted.unwrap() else { unreachable!() };
        let Value::Decimal(exposure_stops) = exposure_converted.unwrap() else { unreachable!() };
        let Value::Integer(max_size) = max_size_converted.unwrap() else { unreachable!() };

        // run node
        let options = RawOptions {
            white_balance: RawWhiteBalance::from_label(&white_balance),
            demosaic,
            // Any unrecognised label falls back to the sRGB default.
            linear_output: output_encoding == "linear",
            max_dimension: (max_size > 0).then_some(max_size as u32),
            apply_orientation: true,
            exposure_stops,
        };

        match decode_raw(&path, &options) {
            Ok(float_img) => {
                let width = float_img.width();
                let height = float_img.height();
                Ok(OperationResponse {
                    time: Instant::now().duration_since(start_time),
                    responses: vec![
                        OutputResponse { value: Value::Image { data: Arc::new(float_img), change_id: get_id() } },
                        OutputResponse { value: Value::Integer(width as i32) },
                        OutputResponse { value: Value::Integer(height as i32) },
                    ],
                })
            }
            Err(e) => Err(OperationError { input_errors, node_error: Some(format!("Error developing raw file: {}", e)) }),
        }
    }
}

#[cfg(test)]
#[path = "raw_tests.rs"]
mod tests;
