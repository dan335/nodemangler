//! 3D preview content: a standalone material viewer with mesh selection and
//! per-channel image assignments. Extracted from `ViewPanel::show_material_ui`
//! plus the material-resolution + render call, no longer gated on a currently
//! viewed image output.

use std::collections::HashMap;

use eframe::egui;

use crate::{graph::graph_node::GraphNode, themes::theme::Theme};

use super::{
    gl_renderer::MeshKind,
    material_channels::{
        list_image_outputs, resolve_material, MaterialAssignment, MaterialChannel,
        MaterialChannelAssignments,
    },
    viewer_3d::Viewer3d,
};

/// Owns the GL-backed 3D viewer and its material channel assignments for one
/// panel leaf. Each leaf keeps its own arcball camera and channel bindings.
pub struct Preview3dPanel {
    pub viewer: Viewer3d,
    pub assignments: MaterialChannelAssignments,
}

impl Preview3dPanel {
    pub fn new() -> Self {
        Self {
            viewer: Viewer3d::new(),
            assignments: MaterialChannelAssignments::new(),
        }
    }
}

/// Render the 3D material preview into `ui`. A free function (rather than a
/// method) so the caller can destructure `Program` fields to avoid borrow
/// conflicts.
pub fn show(
    panel: &mut Preview3dPanel,
    ui: &mut egui::Ui,
    graph_nodes: &HashMap<String, GraphNode>,
    theme: &Theme,
) {
    show_material_ui(panel, ui, graph_nodes);

    ui.add_space(4.0);

    // Resolve assignments to actual image data and render.
    let material = resolve_material(&panel.assignments, graph_nodes);
    panel.viewer.show_material(ui, &material, theme);
}

/// Render the mesh-kind combo + collapsible material channel assignment UI.
fn show_material_ui(
    panel: &mut Preview3dPanel,
    ui: &mut egui::Ui,
    graph_nodes: &HashMap<String, GraphNode>,
) {
    let available_outputs = list_image_outputs(graph_nodes);

    ui.horizontal(|ui| {
        ui.label("Mesh");
        egui::ComboBox::from_id_salt("viewer_3d_mesh_kind")
            .selected_text(panel.viewer.mesh_kind.label())
            .show_ui(ui, |ui| {
                ui.with_layout(
                    egui::Layout::top_down(egui::Align::Min).with_cross_justify(true),
                    |ui| {
                        for kind in MeshKind::ALL {
                            ui.selectable_value(&mut panel.viewer.mesh_kind, kind, kind.label());
                        }
                    },
                );
            });
    });

    egui::CollapsingHeader::new("Material Channels")
        .default_open(false)
        .show(ui, |ui| {
            egui::Grid::new("material_channels_grid")
                .num_columns(2)
                .spacing([8.0, 4.0])
                .show(ui, |ui| {
                    for channel in MaterialChannel::ALL {
                        ui.label(channel.label());

                        // Clone current assignment to avoid borrow conflict with the closure
                        let current = panel.assignments.get(channel).cloned();
                        let current_label = current
                            .as_ref()
                            .and_then(|a| {
                                available_outputs.iter().find(|(nid, oi, _)| {
                                    nid == &a.node_id && *oi == a.output_index
                                })
                            })
                            .map(|(_, _, label)| label.as_str())
                            .unwrap_or("None");

                        let is_none = current.is_none();

                        egui::ComboBox::from_id_salt(format!("mat_{:?}", channel))
                            .selected_text(current_label)
                            .width(200.0)
                            .show_ui(ui, |ui| {
                                ui.with_layout(
                                    egui::Layout::top_down(egui::Align::Min)
                                        .with_cross_justify(true),
                                    |ui| {
                                        if ui.selectable_label(is_none, "None").clicked() {
                                            panel.assignments.clear(channel);
                                        }

                                        for (node_id, output_index, label) in &available_outputs {
                                            let is_selected =
                                                current.as_ref().is_some_and(|a| {
                                                    &a.node_id == node_id
                                                        && a.output_index == *output_index
                                                });
                                            if ui.selectable_label(is_selected, label).clicked() {
                                                panel.assignments.set(
                                                    channel,
                                                    MaterialAssignment {
                                                        node_id: node_id.clone(),
                                                        output_index: *output_index,
                                                    },
                                                );
                                            }
                                        }
                                    },
                                );
                            });

                        ui.end_row();
                    }
                });
        });
}
