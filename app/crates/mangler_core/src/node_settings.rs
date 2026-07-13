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
    /// A one-sentence summary of what the node does. Shown as a hover
    /// tooltip in the node menu / search popup and as a subheading at the
    /// top of the node settings panel.
    ///
    /// Not serialized — static text re-derived on load from the operation
    /// definition (see `Graph::load`), so saves stay small and old files
    /// never show stale text.
    #[serde(skip)]
    pub description: String,
    /// A longer, multi-paragraph explanation of the node: what problem it
    /// solves, how its inputs interact, typical use cases, caveats. Shown
    /// in a collapsible section in the node settings panel.
    ///
    /// Not serialized — re-derived on load like `description`.
    #[serde(skip)]
    pub help: String,
}