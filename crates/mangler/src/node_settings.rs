//! Display metadata for a node in the graph editor.

use serde::{Deserialize, Serialize};

/// Display metadata for a node, used by the UI to render the node header.
///
/// Each operation defines its own settings via the `settings()` method.
/// Subgraph nodes derive their settings from the loaded child graph's name.
#[derive(Clone, Debug, Serialize, Deserialize, Default)]
pub struct NodeSettings {
    /// The node's display name shown in the graph editor header.
    pub name: String,
    /// A brief description of what the node does, shown as a tooltip.
    pub description: String,
}