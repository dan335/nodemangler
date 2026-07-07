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
    Emissive,
}

impl MaterialChannel {
    pub const ALL: [MaterialChannel; 7] = [
        MaterialChannel::Albedo,
        MaterialChannel::Normal,
        MaterialChannel::Roughness,
        MaterialChannel::Metallic,
        MaterialChannel::Height,
        MaterialChannel::AmbientOcclusion,
        MaterialChannel::Emissive,
    ];

    pub fn label(&self) -> &'static str {
        match self {
            MaterialChannel::Albedo => "Albedo",
            MaterialChannel::Normal => "Normal",
            MaterialChannel::Roughness => "Roughness",
            MaterialChannel::Metallic => "Metallic",
            MaterialChannel::Height => "Height",
            MaterialChannel::AmbientOcclusion => "AO",
            MaterialChannel::Emissive => "Emissive",
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
}

impl MaterialChannelAssignments {
    pub fn new() -> Self {
        Self {
            assignments: HashMap::new(),
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

}

/// Resolved material data ready for the 3D renderer.
pub struct MaterialData {
    pub albedo: Option<(FloatImage, String)>,
    pub normal: Option<(FloatImage, String)>,
    pub roughness: Option<(FloatImage, String)>,
    pub metallic: Option<(FloatImage, String)>,
    pub height: Option<(FloatImage, String)>,
    pub ao: Option<(FloatImage, String)>,
    pub emissive: Option<(FloatImage, String)>,
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
            emissive: None,
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
                    MaterialChannel::Emissive => data.emissive = Some(image_data),
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

/// Maps a `material` export node's input index (see
/// `mangler_core::operations::images::outputs::material::OpImageOutputMaterial::create_inputs`)
/// to the 3D-preview `MaterialChannel` it feeds, for the right-click
/// "bind material node to 3D view" action.
///
/// The material node's input order is a frozen contract (documented on
/// `create_inputs`): 0..=7 are the eight PBR maps, 8..=31 are engine
/// preset/file/folder/format/custom-slot inputs that don't correspond to a
/// GUI material channel. Index 1 (opacity) also has no home here — the GUI's
/// `MaterialChannel` enum has no Opacity variant — so it maps to `None` like
/// the non-map inputs.
pub fn material_input_channel(input_index: usize) -> Option<MaterialChannel> {
    match input_index {
        0 => Some(MaterialChannel::Albedo),
        1 => None, // opacity — no GUI channel for it
        2 => Some(MaterialChannel::Normal),
        3 => Some(MaterialChannel::Roughness),
        4 => Some(MaterialChannel::Metallic),
        5 => Some(MaterialChannel::AmbientOcclusion),
        6 => Some(MaterialChannel::Height),
        7 => Some(MaterialChannel::Emissive),
        _ => None, // preset/file name/folder/format/custom slots
    }
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
    // Sort by label, then by (node_id, output_index) so the list ordering is
    // fully deterministic — without the tiebreaker, items sharing a label would
    // fall back to HashMap iteration order and could swap positions.
    result.sort_by(|a, b| a.2.cmp(&b.2).then(a.0.cmp(&b.0)).then(a.1.cmp(&b.1)));
    result
}

#[cfg(test)]
#[path = "material_channels_tests.rs"]
mod tests;
