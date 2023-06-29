use eframe::{
    egui::{self, Pos2, Rect, Layout, RichText},
    epaint::Rounding, WindowInfo,
};
use epaint::Vec2;

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

    pub fn show(&mut self, ctx: &egui::Context, graph_node: &GraphNode, output_index: usize, theme: &Theme, cursor_position: Pos2) -> ViewPanelResponse {
        let mut view_panel_response = ViewPanelResponse::new();
        self.close_window = false;

        

        egui::Window::new(graph_node.settings.name.clone()).title_bar(false).constrain(false).show(ctx, |ui| { 

            if let Some(output) = graph_node.outputs.get(output_index) {
                let height = 22.0;

                // title bar
                ui.style_mut().spacing.interact_size.y = height;

                // // size of title bar bg
                // let rect = Rect::from_min_size(
                //     Pos2::new(ui.cursor().left() - ui.style().spacing.window_margin.left,
                //     ui.cursor().top() - ui.style().spacing.window_margin.top),
                //     Vec2::new(
                //         ui.min_rect().right() + ui.style().spacing.window_margin.left + ui.style().spacing.window_margin.right,
                //         //300.0 + ui.style().spacing.window_margin.left + ui.style().spacing.window_margin.right,
                //         height + ui.style().spacing.window_margin.top + ui.style().spacing.window_margin.bottom
                //     )
                // );

                // paint bg
                //ui.painter().rect_filled(rect, Rounding::none(), theme.get().node_header_bg);

                // name and close button
                ui.horizontal(|ui| {
                    //ui.painter().rect_filled(ui.max_rect(), Rounding::none(), theme.get().node_header_bg);

                    ui.add_space(12.0);
                    let title = format!("{} {}", graph_node.settings.name, output.name);
                    ui.heading(RichText::new(title));
                    
                    // ui.with_layout(Layout::right_to_left(egui::Align::Center), |ui| {
                        
                    // });

                    ui.add_space(22.0);

                    if ui.button("X").clicked() {
                        self.close_window = true;
                    }
                });

                ui.add_space(12.0);

                match &output.value {
                    mangler::value::Value::Bool(value) => TextViewer::show(ui, value.to_string()),
                    mangler::value::Value::Integer(value) => TextViewer::show(ui, value.to_string()),
                    mangler::value::Value::Decimal(value) => TextViewer::show(ui, format!("{:?}", value)),
                    mangler::value::Value::String(value) => TextViewer::show(ui, value.to_string()),
                    mangler::value::Value::Color(value) => ColorViewer::show(ui, *value),
                    mangler::value::Value::DynamicImage { data, change_id } => self.image_viewer.show(ui, graph_node.id.clone(), output_index, change_id.clone(), data, cursor_position, theme),
                    mangler::value::Value::Path(path) => TextViewer::show(ui, path.to_str().unwrap_or("none").to_string()),
                    mangler::value::Value::FilterType(value) => TextViewer::show(ui, format!("{:?}", value)),
                    mangler::value::Value::ColorFormat(value) => TextViewer::show(ui, format!("{:?}", value)),
                    mangler::value::Value::ImageType(value) => TextViewer::show(ui, format!("{:?}", value)),
                    mangler::value::Value::Trigger => TextViewer::show(ui, "trigger".to_string()),
                }
            }

            if ui.ui_contains_pointer() {
                view_panel_response.is_mouse_over = true;
            }
        });

        view_panel_response
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