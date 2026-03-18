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
    /// The current value held by this input.
    pub value: Value,
    /// The initial/reset value for this input.
    pub default_value: Value,
    /// Optional UI widget configuration (drag value, slider, file picker, etc.).
    pub settings: Option<InputSettings>,
    /// If connected, the (node_id, output_index) of the upstream source.
    pub connection: Option<(String, usize)>,
    /// Whether this input is in an error state (e.g. validation failure).
    pub is_error: bool,
    /// Human-readable error message when `is_error` is true.
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

    /// Check whether an output can be connected to this input based on type
    /// compatibility. Returns `true` if the output's value type is in this
    /// input's list of valid conversions.
    pub fn is_valid_connection(&self, output: &Output) -> bool {
        self.accepts_any_type || self.value.value_type().valid_conversions().contains(&output.value.value_type())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::color::Color;
    use crate::output::Output;

    #[test]
    fn test_new_defaults() {
        let input = Input::new("test".to_string(), Value::Decimal(5.0), None, None);
        assert_eq!(input.name, "test");
        assert!(!input.id.is_empty());
        assert!(input.connection.is_none());
        assert!(!input.is_error);
        assert!(input.error_message.is_none());
        assert!(!input.is_exposed);
        assert!(input.link.is_none());
        match (&input.value, &input.default_value) {
            (Value::Decimal(v), Value::Decimal(d)) => {
                assert_eq!(*v, 5.0);
                assert_eq!(*d, 5.0);
            }
            _ => panic!("Expected Decimal"),
        }
    }

    #[test]
    fn test_new_with_settings() {
        let settings = InputSettings::DragValue { speed: Some(0.1), clamp: Some((0.0, 1.0)) };
        let input = Input::new("x".to_string(), Value::Integer(0), Some(settings), None);
        assert!(input.settings.is_some());
    }

    #[test]
    fn test_new_with_link() {
        let link = InputLink { node_id: "n1".to_string(), input_id: "i1".to_string() };
        let input = Input::new("x".to_string(), Value::Bool(false), None, Some(link));
        assert!(input.link.is_some());
        assert_eq!(input.link.as_ref().unwrap().node_id, "n1");
    }

    #[test]
    fn test_partial_eq_same_id() {
        let a = Input::new("a".to_string(), Value::Decimal(1.0), None, None);
        let mut b = a.clone();
        // Same id after clone
        assert_eq!(a, b);
        // Different name doesn't matter
        b.name = "different".to_string();
        assert_eq!(a, b);
    }

    #[test]
    fn test_partial_eq_different_id() {
        let a = Input::new("a".to_string(), Value::Decimal(1.0), None, None);
        let b = Input::new("a".to_string(), Value::Decimal(1.0), None, None);
        // Different get_id() calls produce different IDs
        assert_ne!(a, b);
    }

    // === is_valid_connection: compatible types ===

    #[test]
    fn test_valid_connection_same_type() {
        let input = Input::new("a".to_string(), Value::Decimal(0.0), None, None);
        let output = Output::new("out".to_string(), Value::Decimal(0.0), None);
        assert!(input.is_valid_connection(&output));
    }

    #[test]
    fn test_valid_connection_bool_to_integer() {
        // Bool input can accept Integer output (Integer → Bool conversion exists)
        let input = Input::new("a".to_string(), Value::Bool(false), None, None);
        let output = Output::new("out".to_string(), Value::Integer(1), None);
        assert!(input.is_valid_connection(&output));
    }

    #[test]
    fn test_valid_connection_integer_to_decimal() {
        let input = Input::new("a".to_string(), Value::Integer(0), None, None);
        let output = Output::new("out".to_string(), Value::Decimal(1.0), None);
        assert!(input.is_valid_connection(&output));
    }

    #[test]
    fn test_valid_connection_decimal_to_bool() {
        let input = Input::new("a".to_string(), Value::Decimal(0.0), None, None);
        let output = Output::new("out".to_string(), Value::Bool(true), None);
        assert!(input.is_valid_connection(&output));
    }

    #[test]
    fn test_valid_connection_bool_to_text() {
        let input = Input::new("a".to_string(), Value::Bool(false), None, None);
        let output = Output::new("out".to_string(), Value::Text("hi".to_string()), None);
        // Bool valid_conversions includes Text
        assert!(input.is_valid_connection(&output));
    }

    // === is_valid_connection: incompatible types ===

    #[test]
    fn test_valid_connection_color_to_integer() {
        // Color can now convert to Integer (luminance)
        let input = Input::new("a".to_string(), Value::Color(Color::default()), None, None);
        let output = Output::new("out".to_string(), Value::Integer(1), None);
        assert!(input.is_valid_connection(&output));
    }

    #[test]
    fn test_valid_connection_decimal_to_color() {
        // Decimal can now convert to Color (grayscale)
        let input = Input::new("a".to_string(), Value::Decimal(0.0), None, None);
        let output = Output::new("out".to_string(), Value::Color(Color::default()), None);
        assert!(input.is_valid_connection(&output));
    }

    #[test]
    fn test_invalid_connection_text_to_decimal() {
        // Text input expects Text output; Decimal can convert to Text but Text cannot receive Decimal
        let input = Input::new("a".to_string(), Value::Text("".to_string()), None, None);
        let output = Output::new("out".to_string(), Value::Decimal(1.0), None);
        // Text valid_conversions: [Text, Trigger] — Decimal is not in that list
        assert!(!input.is_valid_connection(&output));
    }

    #[test]
    fn test_valid_connection_path_to_text() {
        // Path input can accept Text (Text is in Path's valid_conversions).
        let input = Input::new("a".to_string(), Value::Path(PathBuf::new()), None, None);
        let output = Output::new("out".to_string(), Value::Text("test".to_string()), None);
        assert!(input.is_valid_connection(&output));
    }

    // === accepts_any_type ===

    #[test]
    fn test_accepts_any_type_default_false() {
        let input = Input::new("x".to_string(), Value::Decimal(0.0), None, None);
        assert!(!input.accepts_any_type);
    }

    #[test]
    fn test_accepts_any_type_allows_incompatible_connection() {
        // Normally a Text input can't accept a DynamicImage output
        let mut input = Input::new("x".to_string(), Value::Text(String::new()), None, None);
        let output = Output::new("out".to_string(), Value::DynamicImage {
            data: std::sync::Arc::new(image::DynamicImage::ImageRgba8(image::RgbaImage::new(1, 1))),
            change_id: crate::get_id(),
        }, None);
        assert!(!input.is_valid_connection(&output));

        // With accepts_any_type, it should be allowed
        input.accepts_any_type = true;
        assert!(input.is_valid_connection(&output));
    }
}

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