//! Image-from-folder input operation.
//!
//! Scans a folder for image files and outputs a single one, selected by an
//! `index` input, plus that file's stem, the clamped index actually used, and
//! the total number of image files found. Unlike [`super::file`], which loads
//! one fixed path, this node is meant to be stepped through by an
//! engine-side batch driver (not built here) that increments `index` and
//! re-runs the graph once per file — the settings-panel "run batch" button
//! mentioned in the help text below. The listing/ordering logic is exposed as
//! standalone helpers ([`resolve_folder`], [`list_image_files`]) so that
//! future batch driver can reuse the exact same file set and order this node
//! uses, instead of re-deriving it and risking drift.

use crate::get_id;
use crate::input::{Input, InputSettings};
use crate::node_settings::NodeSettings;
use crate::operations::{OperationResponse, OperationError, OutputResponse, default_image, convert_input};
use crate::output::Output;
use crate::value::{Value, ValueType};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use std::time::Instant;

use super::file::load_image_from_path;

/// Input index of the `folder` path input (a positional contract with `run`).
pub const FOLDER: usize = 0;
/// Input index of the `index` input (a positional contract with `run`).
pub const INDEX: usize = 1;

/// Operation that loads one image at a time from a folder of images, selected
/// by a numeric index.
///
/// Files are listed non-recursively and sorted case-insensitively by name so
/// the order is stable across runs and platforms. The index is clamped into
/// the valid range rather than erroring, so a batch driver can safely walk
/// past either end without special-casing the last step.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpImageInputFromFolder {}

impl OpImageInputFromFolder {
    /// Returns the node metadata (name and description) for this operation.
    pub fn settings() -> NodeSettings {
        NodeSettings {
            name: "from folder".to_string(),
            description: "Loads one image at a time from a folder of images.".to_string(),
            help: "Scans `folder` (non-recursively) for image files and outputs the one at `index`, along with its file name (without extension), the clamped index that was actually used, and the total number of image files found. Files are sorted case-insensitively by name for a deterministic order across runs and platforms.\n\n`index` is clamped into the available range: negative indices load the first file and indices past the end load the last file, so it's safe to sweep past either boundary. This node pairs with the \"run batch\" button in its settings panel, which re-runs the graph once per file by driving this index from 0 to count-1, force-saving any output nodes each time — output nodes name their files after each source image automatically, or wire this node's `file name` output into an output node's `file name` input for full control. If files are added to or removed from the folder mid-batch, the changing file list can cause some items to repeat or be skipped between runs.\n\nErrors if the folder is unset, cannot be read, or contains no recognized image files.".to_string(),
        }
    }

    /// Creates the input definitions: the source folder and the selected index.
    pub fn create_inputs() -> Vec<Input> {
        vec![
            Input::new("folder".to_string(), Value::Path(PathBuf::new()), Some(InputSettings::Path {
                extension_filter: vec![],
                set_directory: None,
                set_file_name: None,
                set_title: Some("image folder".to_string()),
                file_dialog_type: crate::input::FileDialogType::PickFolder,
            }), None)
                .with_description("Folder containing the images to step through. Relative paths resolve against the graph's own folder."),
            Input::new("index".to_string(), Value::Integer(0), Some(InputSettings::DragValue { clamp: Some((0.0, 100000.0)), speed: None }), None)
                .with_description("Which image to load (0-based), clamped to the number of files found. The batch run steps this automatically."),
        ]
    }

    /// Creates the output definitions: the selected image, its file stem, the
    /// clamped index used, and the total file count.
    pub fn create_outputs() -> Vec<Output> {
        vec![
            Output::new("output".to_string(), Value::Image { data: default_image(), change_id: get_id() }, None)
                .with_description("The image selected by index."),
            Output::new("file name".to_string(), Value::Text(String::new()), None)
                .with_description("The selected file's name without its extension."),
            Output::new("index".to_string(), Value::Integer(0), None)
                .with_description("The index actually used, after clamping to the available file count."),
            Output::new("count".to_string(), Value::Integer(0), None)
                .with_description("Number of image files found in the folder."),
        ]
    }

    /// Executes the operation: lists the folder's image files and loads the
    /// one at the (clamped) requested index.
    ///
    /// Errors — each attributed to the input responsible — if the folder is
    /// unset, unreadable, or empty of recognized image files; also errors
    /// (as a node-level error, like [`super::file::OpImageInputFile::run`])
    /// if the selected file fails to decode.
    pub async fn run(inputs: &mut [Input]) -> Result<OperationResponse, OperationError> {
        let start_time = Instant::now();
        let mut input_errors: Vec<(usize, String)> = vec![];

        // convert inputs
        let folder_converted = convert_input(inputs, FOLDER, ValueType::Path, &mut input_errors);
        let index_converted = convert_input(inputs, INDEX, ValueType::Integer, &mut input_errors);

        // return if error
        if !input_errors.is_empty() { return Err(OperationError { input_errors, node_error: None }); }

        // get values
        let Value::Path(folder) = folder_converted.unwrap() else { unreachable!() };
        let Value::Integer(index) = index_converted.unwrap() else { unreachable!() };

        // Resolve the folder against the graph's directory (None outside the
        // engine, e.g. a direct unit-test call — the folder must be absolute).
        let graph_dir = crate::run_context::current().and_then(|c| c.graph_dir);
        let Some(resolved_dir) = resolve_folder(&folder, graph_dir.as_deref()) else {
            let msg = "no folder set".to_string();
            return Err(OperationError { input_errors: vec![(FOLDER, msg.clone())], node_error: None });
        };

        // List and sort the folder's image files.
        let files = match list_image_files(&resolved_dir) {
            Ok(files) => files,
            Err(e) => {
                let msg = format!("could not read folder: {e}");
                return Err(OperationError { input_errors: vec![(FOLDER, msg.clone())], node_error: None });
            }
        };
        if files.is_empty() {
            let msg = format!("no image files found in {}", resolved_dir.display());
            return Err(OperationError { input_errors: vec![(FOLDER, msg.clone())], node_error: None });
        }

        // Clamp the requested index into the valid range: negative indices
        // load the first file, indices past the end load the last file, so a
        // batch driver can sweep past either boundary without special-casing.
        let count = files.len();
        let clamped_index = index.clamp(0, count as i32 - 1) as usize;
        let path = &files[clamped_index];

        // Decoding is shared with the "from file" node and the GUI's library
        // image preview.
        match load_image_from_path(path) {
            Ok(float_img) => {
                let stem = path.file_stem().map(|s| s.to_string_lossy().to_string()).unwrap_or_default();
                Ok(OperationResponse {
                    time: Instant::now().duration_since(start_time),
                    responses: vec![
                        OutputResponse { value: Value::Image { data: std::sync::Arc::new(float_img), change_id: get_id() } },
                        OutputResponse { value: Value::Text(stem) },
                        OutputResponse { value: Value::Integer(clamped_index as i32) },
                        OutputResponse { value: Value::Integer(count as i32) },
                    ],
                })
            }
            Err(e) => Err(OperationError { input_errors: vec![], node_error: Some(format!("Error opening image: {}", e)) }),
        }
    }
}

/// Resolve the `folder` input against the graph's directory.
///
/// - An empty path means no folder has been chosen: returns `None`.
/// - An absolute path is returned as-is.
/// - A relative path is joined onto `graph_dir`; if there is no graph
///   directory (graph never saved, or a direct unit-test call outside the
///   engine), a relative path cannot be resolved and this also returns
///   `None`.
///
/// Mirrors the folder-resolution half of
/// [`resolve_output_dir_and_stem`](crate::operations::images::outputs::resolve_output_dir_and_stem),
/// which the `to file`/`material` output nodes use for the same purpose.
pub fn resolve_folder(folder: &Path, graph_dir: Option<&Path>) -> Option<PathBuf> {
    if folder.as_os_str().is_empty() {
        None
    } else if folder.is_absolute() {
        Some(folder.to_path_buf())
    } else {
        graph_dir.map(|dir| dir.join(folder))
    }
}

/// Non-recursively list the image files in `dir`.
///
/// Filters entries by extension (case-insensitive) against
/// `ValueType::file_extensions(&ValueType::Image)` — the same extension set
/// the "from file" node's path picker uses — and sorts the result
/// case-insensitively by file name, with the exact (case-sensitive) name as a
/// tie-break so the order is fully deterministic across platforms whose
/// directory-listing order otherwise varies. Subdirectories and
/// non-image files are excluded. Returns `Err` if `dir` cannot be read (e.g.
/// it doesn't exist).
pub fn list_image_files(dir: &Path) -> Result<Vec<PathBuf>, String> {
    let extensions = ValueType::file_extensions(&ValueType::Image);

    let entries = std::fs::read_dir(dir).map_err(|e| e.to_string())?;

    let mut files: Vec<PathBuf> = vec![];
    for entry in entries {
        let entry = entry.map_err(|e| e.to_string())?;
        let path = entry.path();
        if !path.is_file() {
            continue;
        }
        let matches_extension = path
            .extension()
            .and_then(|e| e.to_str())
            .map(|e| extensions.iter().any(|allowed| allowed.eq_ignore_ascii_case(e)))
            .unwrap_or(false);
        if matches_extension {
            files.push(path);
        }
    }

    // Case-insensitive sort by file name, falling back to the exact name so
    // ties (e.g. "a.png" vs "A.PNG") still resolve deterministically.
    files.sort_by(|a, b| {
        let name_a = a.file_name().map(|n| n.to_string_lossy().to_lowercase()).unwrap_or_default();
        let name_b = b.file_name().map(|n| n.to_string_lossy().to_lowercase()).unwrap_or_default();
        name_a.cmp(&name_b).then_with(|| a.file_name().cmp(&b.file_name()))
    });

    Ok(files)
}

#[cfg(test)]
#[path = "from_folder_tests.rs"]
mod tests;
