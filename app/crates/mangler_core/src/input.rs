//! Node input definitions and connection validation.
//!
//! Each node has zero or more inputs that receive data, either from the user
//! (via the UI) or from an upstream node's output through a connection. Inputs
//! carry their current value, default value, optional UI settings, connection
//! state, and error information.

use std::path::PathBuf;
use crate::{value::Value, get_id, Output};
use serde::{Deserialize, Serialize};

/// A single input slot on a node.
///
/// Inputs receive values either from user interaction or from a connected
/// upstream output. They also support being "exposed" so that a subgraph
/// can surface them as inputs on the parent node.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Input {
    /// Unique identifier for this input.
    pub id: String,
    /// Display name shown in the graph editor.
    pub name: String,
    /// Human-readable description of what this input controls, shown as a
    /// tooltip when the user hovers over the input's name in the node settings
    /// panel. Empty string means "no tooltip". `#[serde(default)]` so older
    /// saved graphs (which lack the field) deserialize with an empty description.
    #[serde(default)]
    pub description: String,
    /// The current value held by this input.
    /// Images are replaced with a 1x1 placeholder during serialization to avoid
    /// storing full pixel data. Non-image values (numbers, paths, colors, etc.)
    /// are preserved so user settings survive save/load.
    #[serde(with = "crate::value::value_skip_images")]
    pub value: Value,
    /// The initial/reset value for this input.
    /// Skipped during serialization — reconstructed from the operation definition.
    #[serde(skip)]
    pub default_value: Value,
    /// Optional UI widget configuration (drag value, slider, file picker, etc.).
    pub settings: Option<InputSettings>,
    /// If connected, the (node_id, output_index) of the upstream source.
    pub connection: Option<(String, usize)>,
    /// Whether this input is in an error state (e.g. validation failure).
    /// Skipped during serialization — transient execution state.
    #[serde(skip)]
    pub is_error: bool,
    /// Human-readable error message when `is_error` is true.
    /// Skipped during serialization — transient execution state.
    #[serde(skip)]
    pub error_message: Option<String>,
    /// Whether this input is exposed to the parent graph (for subgraph composition).
    pub is_exposed: bool,
    /// Whether this input accepts any value type (bypasses type compatibility checks).
    /// Used by pass-through nodes like select that forward values without conversion.
    #[serde(default)]
    pub accepts_any_type: bool,
    // Link to a subgraph's internal input so that data flows from
    // the parent node's input into the child graph's input node.
    #[serde(skip)]
    pub link: Option<InputLink>,
}

/// Inputs are compared by identity (ID) only, ignoring values and connections.
impl PartialEq for Input {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id
    }
}

impl Input {
    /// Create a new input with the given name, default value, optional UI settings,
    /// and optional subgraph link. A unique ID is generated automatically.
    pub fn new(name: String, default_value: Value, settings: Option<InputSettings>, link: Option<InputLink>) -> Input {
        Input {
            id: get_id(),
            name,
            description: String::new(),
            value: default_value.clone(),
            default_value,
            settings,
            connection: None,
            is_error: false,
            error_message: None,
            is_exposed: false,
            accepts_any_type: false,
            link,
        }
    }

    /// Builder: attach a human-readable description used as the tooltip when
    /// hovering the input's name in the node settings panel.
    pub fn with_description(mut self, description: impl Into<String>) -> Input {
        self.description = description.into();
        self
    }

    /// Check whether an output can be connected to this input based on type
    /// compatibility. Returns `true` if the output's value type is in this
    /// input's list of valid conversions.
    pub fn is_valid_connection(&self, output: &Output) -> bool {
        self.accepts_any_type || self.value.value_type().valid_conversions().contains(&output.value.value_type())
    }
}

#[cfg(test)]
#[path = "input_tests.rs"]
mod tests;

/// Identifies a specific input inside a subgraph that should receive data
/// from the parent node's corresponding input.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InputLink {
    /// The node ID within the subgraph that owns the target input.
    pub node_id: String,
    /// The unique ID of the target input on that subgraph node.
    pub input_id: String,
}

/// Additional UI widget settings for an input that do not belong in the value itself.
///
/// These control how the input is rendered and interacted with in the graph editor.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum InputSettings {
    /// A file/folder path picker dialog.
    Path {
        /// Allowed file extensions (e.g. `["png", "jpg"]`).
        extension_filter: Vec<String>,
        /// Starting directory for the file dialog.
        set_directory: Option<PathBuf>,
        /// Default file name pre-filled in the dialog.
        set_file_name: Option<String>,
        /// Title bar text for the file dialog window.
        set_title: Option<String>,
        /// Whether to pick a file, pick a folder, or save a file.
        file_dialog_type: FileDialogType,
    },
    /// A numeric drag widget with optional clamping and drag speed.
    DragValue {
        /// Optional (min, max) clamp range.
        clamp: Option<(f32, f32)>,
        /// Drag sensitivity (pixels per unit change).
        speed: Option<f32>,
    },
    /// A horizontal slider widget with a fixed range.
    Slider {
        /// (min, max) range for the slider.
        range: (f32, f32),
        /// Optional step increment between discrete values.
        step_by: Option<f32>,
        /// Whether to prevent the value from exceeding the range.
        clamp_to_range: bool,
    },
    /// A single-line text field.
    SingleLineText,
    /// A multi-line text area.
    MultiLineText,
    /// A dropdown list of predefined text options.
    Dropdown {
        /// The allowed values shown in the dropdown.
        options: Vec<String>,
    },
}

/// The type of file dialog to present when an input uses `InputSettings::Path`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum FileDialogType {
    /// Open a single file.
    PickFile,
    /// Open a folder.
    PickFolder,
    /// Save to a file path.
    SaveFile,
}