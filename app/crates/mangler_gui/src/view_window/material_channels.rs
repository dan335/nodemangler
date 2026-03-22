use std::collections::HashMap;
use mangler_core::float_image::FloatImage;

use crate::graph::graph_node::GraphNode;

/// A PBR material channel type.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum MaterialChannel {
    Albedo,
    Normal,
    Roughness,
    Metallic,
    Height,
    AmbientOcclusion,
}

impl MaterialChannel {
    pub const ALL: [MaterialChannel; 6] = [
        MaterialChannel::Albedo,
        MaterialChannel::Normal,
        MaterialChannel::Roughness,
        MaterialChannel::Metallic,
        MaterialChannel::Height,
        MaterialChannel::AmbientOcclusion,
    ];

    pub fn label(&self) -> &'static str {
        match self {
            MaterialChannel::Albedo => "Albedo",
            MaterialChannel::Normal => "Normal",
            MaterialChannel::Roughness => "Roughness",
            MaterialChannel::Metallic => "Metallic",
            MaterialChannel::Height => "Height",
            MaterialChannel::AmbientOcclusion => "AO",
        }
    }

    /// Keywords that indicate this channel when found in an output name.
    fn keywords(&self) -> &[&str] {
        match self {
            MaterialChannel::Albedo => &["albedo", "base_color", "basecolor", "diffuse", "color"],
            MaterialChannel::Normal => &["normal"],
            MaterialChannel::Roughness => &["roughness", "rough"],
            MaterialChannel::Metallic => &["metallic", "metal"],
            MaterialChannel::Height => &["height", "displacement", "bump"],
            MaterialChannel::AmbientOcclusion => &["ao", "ambient_occlusion", "occlusion"],
        }
    }
}

/// Points to a specific output on a specific node.
#[derive(Clone, Debug, PartialEq)]
pub struct MaterialAssignment {
    pub node_id: String,
    pub output_index: usize,
}

/// Stores assignments from PBR channels to graph node outputs.
pub struct MaterialChannelAssignments {
    pub assignments: HashMap<MaterialChannel, MaterialAssignment>,
    /// Whether auto-detection has been run for the current viewed node.
    auto_detected_for: Option<String>,
}

impl MaterialChannelAssignments {
    pub fn new() -> Self {
        Self {
            assignments: HashMap::new(),
            auto_detected_for: None,
        }
    }

    pub fn get(&self, channel: MaterialChannel) -> Option<&MaterialAssignment> {
        self.assignments.get(&channel)
    }

    pub fn set(&mut self, channel: MaterialChannel, assignment: MaterialAssignment) {
        self.assignments.insert(channel, assignment);
    }

    pub fn clear(&mut self, channel: MaterialChannel) {
        self.assignments.remove(&channel);
    }

    #[allow(dead_code)]
    pub fn clear_all(&mut self) {
        self.assignments.clear();
        self.auto_detected_for = None;
    }

    /// Auto-detect channel assignments by scanning all graph node output names.
    /// Only runs once per viewed node (tracked by node_id).
    pub fn auto_detect(
        &mut self,
        viewed_node_id: &str,
        graph_nodes: &HashMap<String, GraphNode>,
    ) {
        if self.auto_detected_for.as_deref() == Some(viewed_node_id) {
            return;
        }
        self.auto_detected_for = Some(viewed_node_id.to_string());
        self.assignments.clear();

        for channel in MaterialChannel::ALL {
            if let Some(assignment) = find_best_match(channel, graph_nodes) {
                self.assignments.insert(channel, assignment);
            }
        }
    }
}

/// Resolved material data ready for the 3D renderer.
pub struct MaterialData {
    pub albedo: Option<(FloatImage, String)>,
    pub normal: Option<(FloatImage, String)>,
    pub roughness: Option<(FloatImage, String)>,
    pub metallic: Option<(FloatImage, String)>,
    pub height: Option<(FloatImage, String)>,
    pub ao: Option<(FloatImage, String)>,
}

impl MaterialData {
    pub fn empty() -> Self {
        Self {
            albedo: None,
            normal: None,
            roughness: None,
            metallic: None,
            height: None,
            ao: None,
        }
    }
}

/// Resolve all channel assignments to actual image data from graph nodes.
pub fn resolve_material(
    assignments: &MaterialChannelAssignments,
    graph_nodes: &HashMap<String, GraphNode>,
) -> MaterialData {
    let mut data = MaterialData::empty();

    for channel in MaterialChannel::ALL {
        if let Some(assignment) = assignments.get(channel) {
            if let Some(image_data) = resolve_image(assignment, graph_nodes) {
                match channel {
                    MaterialChannel::Albedo => data.albedo = Some(image_data),
                    MaterialChannel::Normal => data.normal = Some(image_data),
                    MaterialChannel::Roughness => data.roughness = Some(image_data),
                    MaterialChannel::Metallic => data.metallic = Some(image_data),
                    MaterialChannel::Height => data.height = Some(image_data),
                    MaterialChannel::AmbientOcclusion => data.ao = Some(image_data),
                }
            }
        }
    }

    data
}

/// Try to extract an image from a node output assignment.
fn resolve_image(
    assignment: &MaterialAssignment,
    graph_nodes: &HashMap<String, GraphNode>,
) -> Option<(FloatImage, String)> {
    let node = graph_nodes.get(&assignment.node_id)?;
    let output = node.outputs.get(assignment.output_index)?;
    if let mangler_core::value::Value::Image { data, change_id } = &output.value {
        Some((data.as_ref().clone(), change_id.clone()))
    } else {
        None
    }
}

/// Find the best matching output for a channel by scanning output names.
fn find_best_match(
    channel: MaterialChannel,
    graph_nodes: &HashMap<String, GraphNode>,
) -> Option<MaterialAssignment> {
    let keywords = channel.keywords();

    for (node_id, node) in graph_nodes {
        for (output_index, output) in node.outputs.iter().enumerate() {
            // Only consider image outputs
            if !matches!(&output.value, mangler_core::value::Value::Image { .. }) {
                continue;
            }

            let name_lower = output.name.to_lowercase();
            let node_name_lower = node.settings.name.to_lowercase();

            for keyword in keywords {
                if name_lower.contains(keyword) || node_name_lower.contains(keyword) {
                    return Some(MaterialAssignment {
                        node_id: node_id.clone(),
                        output_index,
                    });
                }
            }
        }
    }

    None
}

/// Collect all image-type outputs across all graph nodes.
/// Returns (node_id, output_index, display_label) for each.
pub fn list_image_outputs(graph_nodes: &HashMap<String, GraphNode>) -> Vec<(String, usize, String)> {
    let mut result = Vec::new();
    for (node_id, node) in graph_nodes {
        for (output_index, output) in node.outputs.iter().enumerate() {
            if matches!(&output.value, mangler_core::value::Value::Image { .. }) {
                let label = format!("{} - {}", node.settings.name, output.name);
                result.push((node_id.clone(), output_index, label));
            }
        }
    }
    result.sort_by(|a, b| a.2.cmp(&b.2));
    result
}

#[cfg(test)]
#[path = "material_channels_tests.rs"]
mod tests;
