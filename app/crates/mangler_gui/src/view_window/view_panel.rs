use eframe::{
    egui::{self, Pos2, RichText, ViewportBuilder, ViewportId},
};

use crate::{graph::graph_node::GraphNode, themes::theme::Theme};

use super::{text_viewer::TextViewer, image_viewer::ImageViewer, color_viewer::ColorViewer};

pub struct ViewPanel {
    image_viewer: ImageViewer,
    pub close_window: bool,
}

impl ViewPanel {
    pub fn new() -> ViewPanel {
        ViewPanel {
            image_viewer: ImageViewer::new(),
            close_window: false,
        }
    }

    pub fn show(&mut self, ctx: &egui::Context, graph_node: &GraphNode, output_index: usize, theme: &Theme, separate_window: bool, cursor_position: Pos2) -> ViewPanelResponse {
        if separate_window {
            self.show_separate(ctx, graph_node, output_index, theme)
        } else {
            self.show_embedded(ctx, graph_node, output_index, theme, cursor_position)
        }
    }

    fn show_separate(&mut self, ctx: &egui::Context, graph_node: &GraphNode, output_index: usize, theme: &Theme) -> ViewPanelResponse {
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

                    self.show_content(ui, graph_node, output_index, cursor_position, theme);
                });

                if ctx.input(|i| i.viewport().close_requested()) {
                    self.close_window = true;
                }
            },
        );

        view_panel_response
    }

    fn show_embedded(&mut self, ctx: &egui::Context, graph_node: &GraphNode, output_index: usize, theme: &Theme, cursor_position: Pos2) -> ViewPanelResponse {
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

            self.show_content(ui, graph_node, output_index, cursor_position, theme);

            if ui.ui_contains_pointer() {
                view_panel_response.is_mouse_over = true;
            }
        });

        view_panel_response
    }

    fn show_content(&mut self, ui: &mut egui::Ui, graph_node: &GraphNode, output_index: usize, cursor_position: Pos2, theme: &Theme) {
        if let Some(output) = graph_node.outputs.get(output_index) {
            match &output.value {
                mangler_core::value::Value::Bool(value) => TextViewer::show(ui, value.to_string()),
                mangler_core::value::Value::Integer(value) => TextViewer::show(ui, value.to_string()),
                mangler_core::value::Value::Decimal(value) => TextViewer::show(ui, format!("{:?}", value)),
                mangler_core::value::Value::Text(value) => TextViewer::show(ui, value.to_string()),
                mangler_core::value::Value::Color(value) => ColorViewer::show(ui, *value),
                mangler_core::value::Value::Image { data, change_id } => self.image_viewer.show(ui, graph_node.id.clone(), output_index, change_id.clone(), data, cursor_position, theme),
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
            }
        }
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
