use std::collections::HashMap;

use eframe::{
    egui::{self, Pos2, RichText, ViewportBuilder, ViewportId},
};

use crate::{graph::graph_node::GraphNode, themes::theme::Theme};

use super::{
    text_viewer::TextViewer,
    image_viewer::ImageViewer,
    color_viewer::ColorViewer,
    viewer_3d::Viewer3d,
    material_channels::{
        MaterialChannel, MaterialChannelAssignments, MaterialAssignment,
        list_image_outputs, resolve_material,
    },
};

#[derive(PartialEq, Clone, Copy)]
enum ViewTab {
    Texture2D,
    Material3D,
}

pub struct ViewPanel {
    image_viewer: ImageViewer,
    viewer_3d: Viewer3d,
    active_tab: ViewTab,
    material_assignments: MaterialChannelAssignments,
    pub close_window: bool,
}

impl ViewPanel {
    pub fn new() -> ViewPanel {
        ViewPanel {
            image_viewer: ImageViewer::new(),
            viewer_3d: Viewer3d::new(),
            active_tab: ViewTab::Texture2D,
            material_assignments: MaterialChannelAssignments::new(),
            close_window: false,
        }
    }

    pub fn show(
        &mut self,
        ctx: &egui::Context,
        graph_node: &GraphNode,
        output_index: usize,
        theme: &Theme,
        separate_window: bool,
        cursor_position: Pos2,
        graph_nodes: &HashMap<String, GraphNode>,
    ) -> ViewPanelResponse {
        if separate_window {
            self.show_separate(ctx, graph_node, output_index, theme, graph_nodes)
        } else {
            self.show_embedded(ctx, graph_node, output_index, theme, cursor_position, graph_nodes)
        }
    }

    fn show_separate(
        &mut self,
        ctx: &egui::Context,
        graph_node: &GraphNode,
        output_index: usize,
        theme: &Theme,
        graph_nodes: &HashMap<String, GraphNode>,
    ) -> ViewPanelResponse {
        let view_panel_response = ViewPanelResponse::new();
        self.close_window = false;

        let title = if let Some(output) = graph_node.outputs.get(output_index) {
            format!("{} - {}", graph_node.settings.name, output.name)
        } else {
            graph_node.settings.name.clone()
        };

        ctx.show_viewport_immediate(
            ViewportId::from_hash_of("view_panel"),
            ViewportBuilder::default()
                .with_title(title)
                .with_inner_size([600.0, 400.0]),
            |ctx, _class| {
                egui::CentralPanel::default().show(ctx, |ui| {
                    let cursor_position = ctx.input(|i| {
                        i.pointer.hover_pos().unwrap_or(Pos2::ZERO)
                    });

                    self.show_content(ui, graph_node, output_index, cursor_position, theme, graph_nodes);
                });

                if ctx.input(|i| i.viewport().close_requested()) {
                    self.close_window = true;
                }
            },
        );

        view_panel_response
    }

    fn show_embedded(
        &mut self,
        ctx: &egui::Context,
        graph_node: &GraphNode,
        output_index: usize,
        theme: &Theme,
        cursor_position: Pos2,
        graph_nodes: &HashMap<String, GraphNode>,
    ) -> ViewPanelResponse {
        let mut view_panel_response = ViewPanelResponse::new();
        self.close_window = false;

        egui::Window::new(graph_node.settings.name.clone()).title_bar(false).constrain(false).show(ctx, |ui| {
            if let Some(output) = graph_node.outputs.get(output_index) {
                let height = 22.0;
                ui.style_mut().spacing.interact_size.y = height;

                ui.horizontal(|ui| {
                    ui.add_space(12.0);
                    let title = format!("{} {}", graph_node.settings.name, output.name);
                    ui.heading(RichText::new(title));
                    ui.add_space(22.0);
                    if ui.button("X").clicked() {
                        self.close_window = true;
                    }
                });

                ui.add_space(12.0);
            }

            self.show_content(ui, graph_node, output_index, cursor_position, theme, graph_nodes);

            if ui.ui_contains_pointer() {
                view_panel_response.is_mouse_over = true;
            }
        });

        view_panel_response
    }

    fn show_content(
        &mut self,
        ui: &mut egui::Ui,
        graph_node: &GraphNode,
        output_index: usize,
        cursor_position: Pos2,
        theme: &Theme,
        graph_nodes: &HashMap<String, GraphNode>,
    ) {
        if let Some(output) = graph_node.outputs.get(output_index) {
            // Tab switcher for image outputs
            if matches!(&output.value, mangler_core::value::Value::Image { .. }) {
                ui.horizontal(|ui| {
                    ui.selectable_value(&mut self.active_tab, ViewTab::Texture2D, "2D");
                    ui.selectable_value(&mut self.active_tab, ViewTab::Material3D, "3D");
                });
                ui.add_space(4.0);
            }

            // For image outputs, dispatch based on active tab
            if let mangler_core::value::Value::Image { data, change_id } = &output.value {
                match self.active_tab {
                    ViewTab::Texture2D => {
                        self.image_viewer.show(ui, graph_node.id.clone(), output_index, change_id.clone(), data, cursor_position, theme);
                    }
                    ViewTab::Material3D => {
                        // Run auto-detection on first switch to 3D
                        self.material_assignments.auto_detect(&graph_node.id, graph_nodes);

                        // Show material assignment UI
                        self.show_material_ui(ui, graph_nodes);

                        ui.add_space(4.0);

                        // Resolve assignments to actual image data
                        let mut material = resolve_material(&self.material_assignments, graph_nodes);

                        // If no albedo is assigned, use the currently viewed output
                        if material.albedo.is_none() {
                            material.albedo = Some((data.as_ref().clone(), change_id.clone()));
                        }

                        self.viewer_3d.show_material(ui, &material, theme);
                    }
                }
                return;
            }

            // Non-image types always use their dedicated viewer
            match &output.value {
                mangler_core::value::Value::Bool(value) => TextViewer::show(ui, value.to_string()),
                mangler_core::value::Value::Integer(value) => TextViewer::show(ui, value.to_string()),
                mangler_core::value::Value::Decimal(value) => TextViewer::show(ui, format!("{:?}", value)),
                mangler_core::value::Value::Text(value) => TextViewer::show(ui, value.to_string()),
                mangler_core::value::Value::Color(value) => ColorViewer::show(ui, *value),
                mangler_core::value::Value::Path(path) => TextViewer::show(ui, path.to_str().unwrap_or("none").to_string()),
                mangler_core::value::Value::FilterType(value) => TextViewer::show(ui, format!("{:?}", value)),
                mangler_core::value::Value::ColorFormat(value) => TextViewer::show(ui, format!("{:?}", value)),
                mangler_core::value::Value::ImageType(value) => TextViewer::show(ui, format!("{:?}", value)),
                mangler_core::value::Value::Trigger => TextViewer::show(ui, "trigger".to_string()),
                mangler_core::value::Value::NoiseWorleyDistanceFunction(value) => TextViewer::show(ui, format!("{:?}", value)),
                mangler_core::value::Value::ColorSpace(value) => TextViewer::show(ui, format!("{:?}", value)),
                mangler_core::value::Value::BlendMode(value) => TextViewer::show(ui, format!("{:?}", value)),
                mangler_core::value::Value::TextHAlign(value) => TextViewer::show(ui, format!("{:?}", value)),
                mangler_core::value::Value::TextVAlign(value) => TextViewer::show(ui, format!("{:?}", value)),
                mangler_core::value::Value::Image { .. } => unreachable!(),
            }
        }
    }

    /// Render the material channel assignment UI (collapsible).
    fn show_material_ui(
        &mut self,
        ui: &mut egui::Ui,
        graph_nodes: &HashMap<String, GraphNode>,
    ) {
        let available_outputs = list_image_outputs(graph_nodes);

        egui::CollapsingHeader::new("Material Channels")
            .default_open(false)
            .show(ui, |ui| {
                for channel in MaterialChannel::ALL {
                    ui.horizontal(|ui| {
                        ui.label(format!("{:>10}", channel.label()));

                        // Clone current assignment to avoid borrow conflict with the closure
                        let current = self.material_assignments.get(channel).cloned();
                        let current_label = current.as_ref()
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
                                if ui.selectable_label(is_none, "None").clicked() {
                                    self.material_assignments.clear(channel);
                                }

                                for (node_id, output_index, label) in &available_outputs {
                                    let is_selected = current.as_ref().map_or(false, |a| {
                                        &a.node_id == node_id && a.output_index == *output_index
                                    });
                                    if ui.selectable_label(is_selected, label).clicked() {
                                        self.material_assignments.set(
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
}

pub struct ViewPanelResponse {
    pub is_mouse_over: bool,
}

impl ViewPanelResponse {
    pub fn new() -> ViewPanelResponse {
        ViewPanelResponse { is_mouse_over: false }
    }
}
