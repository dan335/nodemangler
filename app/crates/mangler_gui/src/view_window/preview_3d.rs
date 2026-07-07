//! 3D preview content: a standalone material viewer with mesh selection and
//! per-channel image assignments. Extracted from `ViewPanel::show_material_ui`
//! plus the material-resolution + render call, no longer gated on a currently
//! viewed image output.

use std::collections::HashMap;

use eframe::egui;

use crate::{graph::graph_node::GraphNode, themes::theme::Theme};

use super::{
    gl_renderer::{MeshKind, MeshResolution},
    material_channels::{
        list_image_outputs, resolve_material, MaterialAssignment, MaterialChannel,
        MaterialChannelAssignments,
    },
    viewer_3d::Viewer3d,
    viewer_settings::{ToneMap, Viewer3dSettings},
};

/// Owns the GL-backed 3D viewer and its material channel assignments for one
/// panel leaf. Each leaf keeps its own arcball camera, channel bindings and
/// light/camera settings.
pub struct Preview3dPanel {
    pub viewer: Viewer3d,
    pub assignments: MaterialChannelAssignments,
    /// Per-leaf light + camera settings (in-memory only; see `Viewer3dSettings`).
    pub settings: Viewer3dSettings,
}

impl Preview3dPanel {
    pub fn new() -> Self {
        Self {
            viewer: Viewer3d::new(),
            assignments: MaterialChannelAssignments::new(),
            settings: Viewer3dSettings::default(),
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

    // Geometry (mesh tessellation + height displacement), then light and camera
    // controls — all styled like the Material Channels section.
    show_geometry_ui(panel, ui);
    show_lighting_ui(panel, ui);
    show_camera_ui(panel, ui, theme);

    ui.add_space(4.0);

    // Resolve assignments to actual image data and render.
    let material = resolve_material(&panel.assignments, graph_nodes);
    panel
        .viewer
        .show_material(ui, &material, &panel.settings, theme);
}

/// "Geometry" section: mesh tessellation level and height-displacement amount.
/// Same CollapsingHeader + 2-column Grid style as the other sections.
fn show_geometry_ui(panel: &mut Preview3dPanel, ui: &mut egui::Ui) {
    egui::CollapsingHeader::new("Geometry")
        .default_open(false)
        .show(ui, |ui| {
            egui::Grid::new("viewer_3d_geometry_grid")
                .num_columns(2)
                .spacing([8.0, 4.0])
                .show(ui, |ui| {
                    // Mesh tessellation level (drives the renderer's mesh cache).
                    ui.label("Resolution");
                    egui::ComboBox::from_id_salt("viewer_3d_mesh_resolution")
                        .selected_text(panel.viewer.mesh_resolution.label())
                        .show_ui(ui, |ui| {
                            ui.with_layout(
                                egui::Layout::top_down(egui::Align::Min).with_cross_justify(true),
                                |ui| {
                                    for res in MeshResolution::ALL {
                                        ui.selectable_value(
                                            &mut panel.viewer.mesh_resolution,
                                            res,
                                            res.label(),
                                        );
                                    }
                                },
                            );
                        });
                    ui.end_row();

                    // Height displacement amount (0 = flat, no displacement).
                    ui.label("Height Scale");
                    ui.add(egui::Slider::new(
                        &mut panel.settings.height_scale,
                        0.0..=1.0,
                    ));
                    ui.end_row();

                    // UV tiling: repeat the material textures across the mesh.
                    // Values >1 wrap through the sphere/cylinder UV seam (expected).
                    ui.label("UV Tiling");
                    ui.add(
                        egui::DragValue::new(&mut panel.settings.uv_tiling)
                            .range(1.0..=16.0)
                            .speed(0.05),
                    );
                    ui.end_row();

                    // Wireframe overlay on top of the shaded fill.
                    ui.label("Wireframe");
                    ui.checkbox(&mut panel.settings.wireframe, "");
                    ui.end_row();
                });
        });
}

/// "Lighting" section: azimuth/elevation angles (shown in degrees, stored in
/// radians), light color and intensity. Mirrors the Material Channels grid
/// (CollapsingHeader + 2-column egui::Grid) for a consistent look.
fn show_lighting_ui(panel: &mut Preview3dPanel, ui: &mut egui::Ui) {
    egui::CollapsingHeader::new("Lighting")
        .default_open(false)
        .show(ui, |ui| {
            egui::Grid::new("viewer_3d_lighting_grid")
                .num_columns(2)
                .spacing([8.0, 4.0])
                .show(ui, |ui| {
                    let settings = &mut panel.settings;

                    // Azimuth: UI in degrees (−180..180), stored in radians.
                    ui.label("Azimuth");
                    let mut azimuth_deg = settings.light_azimuth.to_degrees();
                    if ui
                        .add(egui::Slider::new(&mut azimuth_deg, -180.0..=180.0).suffix("°"))
                        .changed()
                    {
                        settings.light_azimuth = azimuth_deg.to_radians();
                    }
                    ui.end_row();

                    // Elevation: UI in degrees (0..90), stored in radians.
                    ui.label("Elevation");
                    let mut elevation_deg = settings.light_elevation.to_degrees();
                    if ui
                        .add(egui::Slider::new(&mut elevation_deg, 0.0..=90.0).suffix("°"))
                        .changed()
                    {
                        settings.light_elevation = elevation_deg.to_radians();
                    }
                    ui.end_row();

                    // Light color (linear RGB triplet).
                    ui.label("Color");
                    ui.color_edit_button_rgb(&mut settings.light_color);
                    ui.end_row();

                    // Intensity multiplier.
                    ui.label("Intensity");
                    ui.add(egui::Slider::new(&mut settings.light_intensity, 0.0..=10.0));
                    ui.end_row();

                    // Environment (IBL) contribution: 0 = no ambient, 1 = the
                    // procedural sky's authored radiance.
                    ui.label("Env Intensity");
                    ui.add(egui::Slider::new(&mut settings.env_intensity, 0.0..=2.0));
                    ui.end_row();

                    // Draw the procedural sky (with sun disc) behind the mesh.
                    ui.label("Skybox");
                    ui.checkbox(&mut settings.show_skybox, "");
                    ui.end_row();

                    // Tone-mapping operator applied to the HDR render (and the
                    // skybox) before the gamma step.
                    ui.label("Tone Map");
                    egui::ComboBox::from_id_salt("viewer_3d_tone_map")
                        .selected_text(settings.tone_map.label())
                        .show_ui(ui, |ui| {
                            ui.with_layout(
                                egui::Layout::top_down(egui::Align::Min).with_cross_justify(true),
                                |ui| {
                                    for tm in ToneMap::ALL {
                                        ui.selectable_value(
                                            &mut settings.tone_map,
                                            tm,
                                            tm.label(),
                                        );
                                    }
                                },
                            );
                        });
                    ui.end_row();
                });
        });
}

/// "Camera" section: FOV slider plus a faint hint about the pan/reset controls.
fn show_camera_ui(panel: &mut Preview3dPanel, ui: &mut egui::Ui, theme: &Theme) {
    egui::CollapsingHeader::new("Camera")
        .default_open(false)
        .show(ui, |ui| {
            egui::Grid::new("viewer_3d_camera_grid")
                .num_columns(2)
                .spacing([8.0, 4.0])
                .show(ui, |ui| {
                    // Field of view, in degrees, driving the perspective each frame.
                    ui.label("FOV");
                    ui.add(
                        egui::Slider::new(&mut panel.settings.fov_y_degrees, 20.0..=90.0)
                            .suffix("°"),
                    );
                    ui.end_row();
                });

            // Faint interaction hint (theme-derived color, no hardcoded values).
            ui.label(
                egui::RichText::new("Middle-drag to pan · F to reset")
                    .color(theme.get().text_faint)
                    .small(),
            );
        });
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
