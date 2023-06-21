use eframe::{
    egui::{self, Pos2, Rect, Layout},
    epaint::{Rounding, Stroke},
};
use epaint::Vec2;

use crate::{graph::graph_node::GraphNode, themes::theme::Theme};

use super::{text_viewer::TextViewer, image_viewer::ImageViewer};

pub struct ViewPanel {
    image_viewer: ImageViewer,
    position: Pos2,
    pub close_window: bool,
}

impl ViewPanel {
    pub fn new() -> ViewPanel {
        ViewPanel {
            image_viewer: ImageViewer::new(),
            position: Pos2::ZERO,
            close_window: false,
        }
    }

    pub fn show(&mut self, ctx: &egui::Context, graph_node: &GraphNode, output_index: usize, theme: &Theme, cursor_position: Pos2) -> ViewPanelResponse {
        let mut view_panel_response = ViewPanelResponse::new();
        self.close_window = false;
        //let window_id = format!("{}_{}", graph_node.id, output_index);

        egui::Window::new("viewer").title_bar(false).constrain(false).show(ctx, |ui| {
            

            if let Some(output) = graph_node.outputs.get(output_index) {
                let height = 22.0;

                // title bar
                ui.style_mut().spacing.interact_size.y = height;

                // size of title bar bg
                let rect = Rect::from_min_size(
                    Pos2::new(ui.cursor().left() - ui.style().spacing.window_margin.left,
                    ui.cursor().top() - ui.style().spacing.window_margin.top),
                    Vec2::new(
                        ui.max_rect().right() + ui.style().spacing.window_margin.left + ui.style().spacing.window_margin.right,
                        height + ui.style().spacing.window_margin.top + ui.style().spacing.window_margin.bottom
                    )
                );

                // paint bg
                ui.painter().rect_filled(rect, Rounding::none(), theme.get().node_header_bg);

                // name and close button
                ui.horizontal(|ui| {
                    ui.add_space(12.0);
                    let title = format!("{} {}", graph_node.settings.name, output.name);
                    ui.heading(title);
                    

                    ui.with_layout(Layout::right_to_left(egui::Align::Center), |ui| {
                        ui.add_space(12.0);
                        if ui.button("X").clicked() {
                            self.close_window = true;
                        }
                    });
                });

                ui.add_space(12.0);

                match &output.value {
                    mangler::value::Value::Bool(value) => TextViewer::show(ui, value.to_string()),
                    mangler::value::Value::Integer(value) => TextViewer::show(ui, value.to_string()),
                    mangler::value::Value::Decimal(value) => TextViewer::show(ui, value.to_string()),
                    mangler::value::Value::String(value) => TextViewer::show(ui, value.to_string()),
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

    
    

    pub fn draw_background_grid(&self, ui: &mut egui::Ui, rect: Rect, graph_position: Pos2, theme: &Theme) {
        let stroke = Stroke::new(1.0, theme.get().grid_lines);
        let grid_size: f32 = 50.0;

        let mut x = rect.min.x + (graph_position.x % grid_size);
        let mut y = rect.min.y + (graph_position.y % grid_size);
        while x <= rect.max.x {
            let points: Vec<Pos2> = vec![
                Pos2::new(x, rect.min.y),
                Pos2::new(x, rect.max.y),
            ];
            ui.painter().add(egui::Shape::line(points.clone(), stroke));

            x += grid_size;
        }

        while y <= rect.max.y {
            let points: Vec<Pos2> = vec![
                Pos2::new(rect.min.x, y),
                Pos2::new(rect.max.x, y),
            ];
            ui.painter().add(egui::Shape::line(points.clone(), stroke));

            y += grid_size;
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