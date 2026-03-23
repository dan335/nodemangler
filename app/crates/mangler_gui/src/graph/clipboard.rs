use eframe::egui::Pos2;
use mangler_core::value::Value;
use mangler_core::AddNodeType;
use serde::{Serialize, Deserialize};

/// Prefix used to identify NodeMangler clipboard data in the system clipboard.
const CLIPBOARD_MARKER: &str = "NODEMANGLER:";

/// Serde helper to serialize/deserialize egui's `Pos2` as `[f32; 2]`.
mod pos2_serde {
    use eframe::egui::Pos2;
    use serde::{self, Deserialize, Deserializer, Serialize, Serializer};

    pub fn serialize<S: Serializer>(pos: &Pos2, s: S) -> Result<S::Ok, S::Error> {
        [pos.x, pos.y].serialize(s)
    }

    pub fn deserialize<'de, D: Deserializer<'de>>(d: D) -> Result<Pos2, D::Error> {
        let [x, y] = <[f32; 2]>::deserialize(d)?;
        Ok(Pos2::new(x, y))
    }
}

/// A snapshot of a single node captured during a copy operation.
#[derive(Clone, Serialize, Deserialize)]
pub struct ClipboardNode {
    /// Original node ID (used to remap connections when pasting).
    pub original_id: String,
    /// The operation/subgraph type needed to recreate this node.
    pub node_type: AddNodeType,
    /// Position in graph space at copy time.
    #[serde(with = "pos2_serde")]
    pub position: Pos2,
    /// Input values at copy time (index, value). Images are excluded to avoid memory bloat.
    pub input_values: Vec<(usize, Value)>,
    /// Whether the node was enabled when copied.
    pub is_enabled: bool,
    /// User-defined custom name, if any.
    pub custom_name: Option<String>,
}

/// A connection between two copied nodes (both endpoints are in the clipboard).
#[derive(Clone, Serialize, Deserialize)]
pub struct ClipboardConnection {
    /// Original source (upstream) node ID.
    pub output_node_id: String,
    /// Output index on the source node.
    pub output_index: usize,
    /// Original destination (downstream) node ID.
    pub input_node_id: String,
    /// Input index on the destination node.
    pub input_index: usize,
}

/// The full clipboard contents from a copy operation.
#[derive(Clone, Serialize, Deserialize)]
pub struct Clipboard {
    /// The copied nodes.
    pub nodes: Vec<ClipboardNode>,
    /// Internal connections between the copied nodes.
    pub connections: Vec<ClipboardConnection>,
}

impl Clipboard {
    /// Build a clipboard from the given selected node IDs and the graph node map.
    ///
    /// Captures each selected node's type, position, input values (excluding images),
    /// and enabled state. Also captures connections where both endpoints are selected.
    pub fn from_selection(
        selected_ids: &std::collections::HashSet<String>,
        graph_nodes: &std::collections::HashMap<String, crate::graph::graph_node::GraphNode>,
    ) -> Option<Clipboard> {
        if selected_ids.is_empty() {
            return None;
        }

        let mut nodes = Vec::new();
        let mut connections = Vec::new();

        for id in selected_ids {
            let Some(node) = graph_nodes.get(id) else {
                continue;
            };
            let Some(node_type) = &node.node_type else {
                // Node was loaded from a save file and doesn't have a type recorded.
                // We can't recreate it, so skip.
                continue;
            };

            // Capture input values, skipping images to avoid memory bloat.
            let input_values: Vec<(usize, Value)> = node
                .inputs
                .iter()
                .enumerate()
                .filter(|(_, input)| !matches!(input.value, Value::Image { .. }))
                .map(|(i, input)| (i, input.value.clone()))
                .collect();

            nodes.push(ClipboardNode {
                original_id: id.clone(),
                node_type: node_type.clone(),
                position: node.position,
                input_values,
                is_enabled: node.is_enabled,
                custom_name: node.custom_name.clone(),
            });

            // Capture internal connections (where the output node is also selected).
            for (input_index, input) in node.inputs.iter().enumerate() {
                if let Some((output_node_id, output_index)) = &input.connection {
                    if selected_ids.contains(output_node_id) {
                        connections.push(ClipboardConnection {
                            output_node_id: output_node_id.clone(),
                            output_index: *output_index,
                            input_node_id: id.clone(),
                            input_index,
                        });
                    }
                }
            }
        }

        if nodes.is_empty() {
            return None;
        }

        Some(Clipboard { nodes, connections })
    }

    /// Compute the centroid of all copied nodes.
    pub fn centroid(&self) -> Pos2 {
        let n = self.nodes.len() as f32;
        let sum_x: f32 = self.nodes.iter().map(|n| n.position.x).sum();
        let sum_y: f32 = self.nodes.iter().map(|n| n.position.y).sum();
        Pos2::new(sum_x / n, sum_y / n)
    }

    /// Serialize the clipboard to a string for the system clipboard.
    /// Prepends a marker so we can quickly identify our data on paste.
    pub fn to_clipboard_string(&self) -> String {
        let json = serde_json::to_string(self).unwrap_or_default();
        format!("{}{}", CLIPBOARD_MARKER, json)
    }

    /// Try to deserialize a clipboard from system clipboard text.
    /// Returns `None` if the text doesn't start with our marker or JSON is invalid.
    pub fn from_clipboard_string(text: &str) -> Option<Clipboard> {
        let json = text.strip_prefix(CLIPBOARD_MARKER)?;
        serde_json::from_str(json).ok()
    }
}

#[cfg(test)]
#[path = "clipboard_tests.rs"]
mod tests;
