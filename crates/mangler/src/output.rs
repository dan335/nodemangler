//! Node output definitions and connection validation.
//!
//! Each node has zero or more outputs that carry computed results to downstream
//! nodes. Outputs support fan-out: a single output can feed multiple inputs on
//! different nodes simultaneously.

use crate::{value::Value, get_id, Input};
use serde::{Deserialize, Serialize};

/// A single output slot on a node.
///
/// Outputs hold the result of a node's computation and track which downstream
/// inputs they are connected to. They also support being "exposed" so that a
/// subgraph can surface them as outputs on the parent node.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Output {
    /// Unique identifier for this output.
    pub id: String,
    /// Display name shown in the graph editor.
    pub name: String,
    /// The current computed value produced by the node.
    pub value: Value,
    /// The initial/reset value for this output.
    pub default_value: Value,
    /// Fan-out connections: list of (downstream_node_id, input_index) pairs.
    pub connection: Option<Vec<(String, usize)>>,
    /// Whether this output is exposed to the parent graph (for subgraph composition).
    pub is_exposed: bool,
    /// Link to a subgraph's internal output so that data flows from the child
    /// graph's output node back to the parent node's output. Not serialized.
    #[serde(skip)]
    pub link: Option<OutputLink>,
}

/// Outputs are compared by identity (ID) only, ignoring values and connections.
impl PartialEq for Output {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id
    }
}

impl Output {
    /// Create a new output with the given name, default value, and optional subgraph link.
    /// A unique ID is generated automatically.
    pub fn new(name: String, default_value: Value, link: Option<OutputLink>) -> Output {
        Output {
            name,
            value: default_value.clone(),
            default_value,
            connection: None,
            is_exposed: false,
            link,
            id: get_id(),
        }
    }

    /// Check whether this output can be connected to the given input based on
    /// type compatibility. Returns `true` if the input's value type is in this
    /// output's list of valid conversions.
    pub fn is_valid_connection(&self, input: &Input) -> bool {
        self.value.value_type().valid_conversions().contains(&input.value.value_type())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::color::Color;
    use crate::input::Input;

    #[test]
    fn test_new_defaults() {
        let output = Output::new("result".to_string(), Value::Integer(0), None);
        assert_eq!(output.name, "result");
        assert!(!output.id.is_empty());
        assert!(output.connection.is_none());
        assert!(!output.is_exposed);
        assert!(output.link.is_none());
        match (&output.value, &output.default_value) {
            (Value::Integer(v), Value::Integer(d)) => {
                assert_eq!(*v, 0);
                assert_eq!(*d, 0);
            }
            _ => panic!("Expected Integer"),
        }
    }

    #[test]
    fn test_new_with_link() {
        let link = OutputLink { node_id: "n1".to_string(), output_index: 2 };
        let output = Output::new("out".to_string(), Value::Bool(true), Some(link));
        assert!(output.link.is_some());
        assert_eq!(output.link.as_ref().unwrap().output_index, 2);
    }

    #[test]
    fn test_partial_eq_same_id() {
        let a = Output::new("a".to_string(), Value::Decimal(1.0), None);
        let mut b = a.clone();
        assert_eq!(a, b);
        b.name = "different".to_string();
        assert_eq!(a, b);
    }

    #[test]
    fn test_partial_eq_different_id() {
        let a = Output::new("a".to_string(), Value::Decimal(1.0), None);
        let b = Output::new("a".to_string(), Value::Decimal(1.0), None);
        assert_ne!(a, b);
    }

    // === is_valid_connection ===

    #[test]
    fn test_valid_connection_same_type() {
        let output = Output::new("out".to_string(), Value::Decimal(0.0), None);
        let input = Input::new("in".to_string(), Value::Decimal(0.0), None, None);
        assert!(output.is_valid_connection(&input));
    }

    #[test]
    fn test_valid_connection_integer_output_to_bool_input() {
        let output = Output::new("out".to_string(), Value::Integer(1), None);
        let input = Input::new("in".to_string(), Value::Bool(false), None, None);
        // Integer valid_conversions contains Bool
        assert!(output.is_valid_connection(&input));
    }

    #[test]
    fn test_valid_connection_bool_output_to_decimal_input() {
        let output = Output::new("out".to_string(), Value::Bool(true), None);
        let input = Input::new("in".to_string(), Value::Decimal(0.0), None, None);
        // Bool valid_conversions contains Decimal
        assert!(output.is_valid_connection(&input));
    }

    #[test]
    fn test_valid_connection_color_output_to_integer_input() {
        // Color can now convert to Integer (luminance)
        let output = Output::new("out".to_string(), Value::Color(Color::default()), None);
        let input = Input::new("in".to_string(), Value::Integer(0), None, None);
        assert!(output.is_valid_connection(&input));
    }

    #[test]
    fn test_invalid_connection_string_output_to_bool_input() {
        let output = Output::new("out".to_string(), Value::String("hi".to_string()), None);
        let input = Input::new("in".to_string(), Value::Bool(false), None, None);
        // String valid_conversions: [String, Trigger] — Bool not in list
        assert!(!output.is_valid_connection(&input));
    }

    #[test]
    fn test_valid_connection_decimal_output_to_string_input() {
        let output = Output::new("out".to_string(), Value::Decimal(1.0), None);
        let input = Input::new("in".to_string(), Value::String("".to_string()), None, None);
        // Decimal valid_conversions: [Bool, Integer, Decimal, String, Trigger] — String is in list
        assert!(output.is_valid_connection(&input));
    }

    #[test]
    fn test_valid_connection_trigger() {
        // Everything can connect to Trigger
        let output = Output::new("out".to_string(), Value::Bool(true), None);
        let input = Input::new("in".to_string(), Value::Trigger, None, None);
        assert!(output.is_valid_connection(&input));
    }
}

/// Identifies a specific output inside a subgraph that should feed data
/// back to the parent node's corresponding output.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OutputLink {
    /// The node ID within the subgraph that owns the source output.
    pub node_id: String,
    /// The zero-based index of the source output on that subgraph node.
    pub output_index: usize,
}
