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
    /// Skipped during serialization — outputs are always recomputed when the graph runs.
    #[serde(skip)]
    pub value: Value,
    /// The initial/reset value for this output.
    /// Skipped during serialization — reconstructed from the operation definition.
    #[serde(skip)]
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
        input.accepts_any_type || self.value.value_type().valid_conversions().contains(&input.value.value_type())
    }
}

#[cfg(test)]
#[path = "output_tests.rs"]
mod tests;

/// Identifies a specific output inside a subgraph that should feed data
/// back to the parent node's corresponding output.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OutputLink {
    /// The node ID within the subgraph that owns the source output.
    pub node_id: String,
    /// The zero-based index of the source output on that subgraph node.
    pub output_index: usize,
}
