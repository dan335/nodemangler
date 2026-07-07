//! 3D preview content: a standalone material viewer with mesh selection and
//! per-channel image assignments. Extracted from `ViewPanel::show_material_ui`
//! plus the material-resolution + render call, no longer gated on a currently
//! viewed image output.
//!
//! The controls above the viewport are a single horizontal toolbar of
//! `ui.menu_button` dropdowns ("Mesh", "Material", "Light", "Camera") rather
//! than a mesh combo + stacked `CollapsingHeader`s, so the viewport keeps
//! nearly all of the panel's vertical space (Substance-Designer-style compact
//! UI). See the individual `*_menu` functions for the no-nested-`ComboBox`
//! constraint that shapes how single-pick lists are laid out inside them.

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
    // Compact toolbar: one row of menu-button dropdowns instead of a mesh
    // combo + four stacked CollapsingHeaders, so the viewport below gets
    // nearly all of the panel's vertical space.
    ui.horizontal(|ui| {
        mesh_menu(panel, ui);
        material_menu(panel, ui, graph_nodes);
        light_menu(panel, ui);
        camera_menu(panel, ui, theme);
    });

    ui.add_space(2.0);

    // Resolve assignments to actual image data and render.
    let material = resolve_material(&panel.assignments, graph_nodes);
    panel
        .viewer
        .show_material(ui, &material, &panel.settings, theme);
}

/// Small faint heading used to separate groups of controls within a single
/// menu body (e.g. "Mesh Kind" vs "Resolution" inside the Mesh menu, both of
/// which live in the same dropdown — see `mesh_menu`'s doc comment for why).
fn section_label(ui: &mut egui::Ui, text: &str) {
    ui.label(egui::RichText::new(text).small().weak());
}

/// "Mesh" toolbar menu: mesh kind, tessellation resolution, wireframe
/// overlay, height-displacement scale and UV tiling.
///
/// `MeshKind` and `MeshResolution` are single-pick lists that would normally
/// be `ComboBox`es, but a `ComboBox` nested inside a menu popup is known to
/// misbehave in egui 0.34 (it can close the parent menu or get clipped by
/// it). Instead the menu popup itself *is* the dropdown: each option is laid
/// out directly as a `selectable_value` row in the menu body, with a small
/// section label + separator between groups sharing the one menu.
fn mesh_menu(panel: &mut Preview3dPanel, ui: &mut egui::Ui) {
    ui.menu_button("Mesh", |ui| {
        ui.set_min_width(220.0);

        // Mesh tessellation shape (drives the renderer's mesh cache).
        section_label(ui, "Mesh Kind");
        for kind in MeshKind::ALL {
            ui.selectable_value(&mut panel.viewer.mesh_kind, kind, kind.label());
        }

        ui.separator();

        // Mesh tessellation level (drives the renderer's mesh cache).
        section_label(ui, "Resolution");
        for res in MeshResolution::ALL {
            ui.selectable_value(&mut panel.viewer.mesh_resolution, res, res.label());
        }

        ui.separator();

        egui::Grid::new("viewer_3d_mesh_grid")
            .num_columns(2)
            .spacing([8.0, 4.0])
            .show(ui, |ui| {
                // Wireframe overlay on top of the shaded fill.
                ui.label("Wireframe");
                ui.checkbox(&mut panel.settings.wireframe, "");
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
            });
    });
}

/// "Material" toolbar menu: one entry per PBR channel binding.
///
/// Each channel needs its own "None" + list-of-image-outputs picker, which
/// (like the mesh kind/resolution pickers above) cannot be a `ComboBox`
/// nested inside the "Material" menu popup. Sub-menus (menu-in-menu) *are*
/// the egui-0.34-supported nesting form, so each channel gets its own
/// `ui.menu_button` inside the Material menu body — calling `menu_button`
/// from inside another menu's contents renders it as a proper cascading
/// sub-menu instead of a fresh top-level dropdown. The candidate list is
/// wrapped in a scroll area since it can get long.
fn material_menu(
    panel: &mut Preview3dPanel,
    ui: &mut egui::Ui,
    graph_nodes: &HashMap<String, GraphNode>,
) {
    let available_outputs = list_image_outputs(graph_nodes);

    ui.menu_button("Material", |ui| {
        ui.set_min_width(220.0);

        for channel in MaterialChannel::ALL {
            // Clone current assignment to avoid borrow conflict with the closure.
            let current = panel.assignments.get(channel).cloned();
            let current_label = current
                .as_ref()
                .and_then(|a| {
                    available_outputs
                        .iter()
                        .find(|(nid, oi, _)| nid == &a.node_id && *oi == a.output_index)
                })
                .map(|(_, _, label)| label.as_str())
                .unwrap_or("None");
            let is_none = current.is_none();

            ui.menu_button(format!("{}: {}", channel.label(), current_label), |ui| {
                ui.set_min_width(220.0);
                egui::ScrollArea::vertical()
                    .max_height(300.0)
                    .show(ui, |ui| {
                        if ui.selectable_label(is_none, "None").clicked() {
                            panel.assignments.clear(channel);
                        }

                        for (node_id, output_index, label) in &available_outputs {
                            let is_selected = current.as_ref().is_some_and(|a| {
                                &a.node_id == node_id && a.output_index == *output_index
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
                    });
            });
        }
    });
}

/// "Light" toolbar menu: sun azimuth/elevation/color/intensity, environment
/// (IBL) intensity, skybox toggle and tone-mapping operator.
///
/// Azimuth/elevation/color/intensity/env/skybox are slider, color-button and
/// checkbox widgets, which are fine inside a menu popup. `ToneMap` is a
/// single-pick list though, so — same reasoning as `mesh_menu` — it is laid
/// out as `selectable_value` rows directly in the menu body rather than a
/// nested `ComboBox`.
fn light_menu(panel: &mut Preview3dPanel, ui: &mut egui::Ui) {
    ui.menu_button("Light", |ui| {
        ui.set_min_width(220.0);

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
            });

        ui.separator();

        // Tone-mapping operator applied to the HDR render (and the skybox)
        // before the gamma step.
        section_label(ui, "Tone Map");
        for tm in ToneMap::ALL {
            ui.selectable_value(&mut panel.settings.tone_map, tm, tm.label());
        }
    });
}

/// "Camera" toolbar menu: field-of-view slider plus the faint pan/reset hint.
/// No single-pick list here, so the no-nested-`ComboBox` constraint that
/// shapes the other menus doesn't apply.
fn camera_menu(panel: &mut Preview3dPanel, ui: &mut egui::Ui, theme: &Theme) {
    ui.menu_button("Camera", |ui| {
        ui.set_min_width(220.0);

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
