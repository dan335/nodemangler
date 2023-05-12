#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")] // hide console window on Windows in release

use eframe::egui::{self};
use eframe::epaint::Rounding;
use mangler::nodes::add::Add;
use mangler::nodes::node::Node;
use mangler::nodes::operation::ConnectionSettings;
use mangler::{graph::Graph, nodes::node_settings::NodeSettings};
use mangler::{nodes::*, get_id};
use egui::Vec2;

mod graph;
mod menu;
mod menu_button;
mod title_bar;
use graph::graph_editor::GraphEditor;
use egui::{Pos2, Rect, Sense};
use menu::Menu;
use crate::graph::graph_editor::GraphEditorResponse;



pub const DEFAULT_WINDOW_WIDTH: f32 = 1280.0;
pub const DEFAULT_WINDOW_HEIGHT: f32 = 800.0;


fn main() -> Result<(), eframe::Error> {
    //let mut graph = Graph::new();

    // let id = add::Add::new(&mut graph);

    // if let Some(node) = graph.nodes.get_mut(&id) {
    //     node.set_intput_value(0, Value::Decimal { value: 5.0 });
    // }

    // graph.run();

    // if let Some(v) = graph.nodes.get(&id) {
    //     println!("Hello, world! {:?}", v.print_output());
    // }

    let options = eframe::NativeOptions {
        initial_window_size: Some(egui::vec2(DEFAULT_WINDOW_WIDTH, DEFAULT_WINDOW_HEIGHT)),
        ..Default::default()
    };

    eframe::run_native(
        "Node Mangler",
        options,
        Box::new(|_cc| Box::<MyApp>::default()),
    )
}

struct MyApp {
    pub graph: Graph,
    graph_editor: GraphEditor,
    menu: Menu,
    dragging_menu_button: Option<NodeSettings>,
}

impl Default for MyApp {
    fn default() -> Self {
        Self {
            graph: Graph::new(),
            graph_editor: GraphEditor::new(),
            menu: Menu::new(),
            dragging_menu_button: None,
        }
    }
}

impl eframe::App for MyApp {
    fn update(&mut self, ctx: &egui::Context, frame: &mut eframe::Frame) {

        egui::CentralPanel::default().show(ctx, |ui| {

            let app_rect = ctx.screen_rect();
            let cursor_position = ui.ctx().input(|i| i.pointer.hover_pos()).unwrap_or(Pos2::ZERO);
            let cursor_primary_down: bool = ui.ctx().input(|i| i.pointer.primary_down());
            let cursor_inside = app_rect.contains(cursor_position);

            //let mouse_response = ui.allocate_rect(app_rect, Sense::drag());

            // sidebar menu
            let left_rect = Rect::from_two_pos(Pos2::new(0.0, 0.0), Pos2::new(200.0, app_rect.width()));
            ui.allocate_ui_at_rect(left_rect, |ui| {

                // sidebar bg
                ui.painter().add(egui::Shape::rect_filled(
                    ui.max_rect(),
                    Rounding::none(),
                    egui::Color32::from_gray(40),
                ));

                // show menu
                let menu_result = self.menu.show(ui, cursor_position);

                // dragging from menu
                if menu_result.dragging_menu_button.is_some() {
                    self.dragging_menu_button = menu_result.dragging_menu_button.clone();
                }
                
            });

            // top panel
            let top_rect = Rect::from_two_pos(Pos2::new(200.0, 0.0), Pos2::new(app_rect.width(), app_rect.height() / 2.0));
            ui.allocate_ui_at_rect(top_rect, |ui| {
                ui.painter().add(egui::Shape::rect_filled(
                    ui.max_rect(),
                    Rounding::none(),
                    egui::Color32::from_gray(30),
                ));
                ui.vertical_centered(|ui| {
                    ui.heading("Top Panel");
                });
            });

            // bottom graph panel
            let bottom_rect = Rect::from_two_pos(Pos2::new(200.0, app_rect.height() / 2.0), Pos2::new(app_rect.width(), app_rect.height()));
            ui.allocate_ui_at_rect(bottom_rect, |ui| {
                let graph_editor_response: GraphEditorResponse = self.graph_editor.show(ui, cursor_position, &self.graph.nodes, cursor_primary_down);

                if let Some(new_connection) = graph_editor_response.new_connection {
                    self.connect_nodes(new_connection);
                }
            });

            // dragging from menu
            // mouse leaves app
            // stop dragging
            if !cursor_inside {
                self.dragging_menu_button = None;
            }

            // release mouse button after dragging menu button
            ui.input(|i| {
                if i.pointer.primary_released() {
                    if let Some(dragging_settings) = self.dragging_menu_button.clone() {
                        if bottom_rect.contains(cursor_position) {

                            let node_settings = add::SETTINGS.clone();
                            let input_sttings = &add::INPUT_SETTINGS. clone();
                            let output_settings = &add::OUTPUT_SETTINGS.clone();

                            self.add_node(node_settings, input_sttings, &output_settings, cursor_position);
                        }
                    }

                    self.dragging_menu_button = None;
                }
            });

            // dragging node from menu
            // draw shape behind mouse being dragged
            if let Some(dragging_settings) = self.dragging_menu_button.clone() {
                let drag_rect = Rect::from_center_size(cursor_position, Vec2::new(80.0, 80.0));
                ui.painter().add(egui::Shape::rect_filled(drag_rect, Rounding::none(), egui::Color32::from_gray(100)));
            }

            // show cpu usage in bototm right corner
            if let Some(cpu_usage) = frame.info().cpu_usage {
                let pos = Pos2::new(app_rect.right() - 10.0, app_rect.bottom() - 10.0);
                let txt = format!("{:.3} ms", cpu_usage * 1000.0);
                ui.painter().text(pos, egui::Align2::RIGHT_BOTTOM, txt, egui::FontId::monospace(8.0), egui::Color32::from_gray(150));
            }
        });
    }
}


impl MyApp {
    pub fn connect_nodes(&mut self, new_connection: NewConnection) {
        if self.graph.nodes.get_mut(&new_connection.input_node_id).is_some() && self.graph.nodes.get_mut(&new_connection.output_node_id).is_some() {

            if let Some(from) = self.graph.nodes.get_mut(&new_connection.input_node_id) {
                from.set_output_connection(new_connection.output_connection_index, new_connection.input_node_id);
            }

            if let Some(to) = self.graph.nodes.get_mut(&new_connection.output_node_id) {
                to.set_input_connection(new_connection.input_connection_index, new_connection.output_node_id);
            }
        }
    }

    pub fn add_node(&mut self, node_settings: NodeSettings, input_settings: &Vec<ConnectionSettings>, output_settings: &Vec<ConnectionSettings>, position: Pos2) -> String {
        let id = get_id();
        let add = Add::default();
        let node = Node::new(id.clone(), input_settings, output_settings, Box::new(add));
        self.graph.add_node(id.clone(), node);
        self.graph_editor.add_node(id.clone(), node_settings, position);
        id
    }
}


#[derive(Debug)]
pub struct NewConnection {
    input_node_id: String,
    input_connection_index: usize,
    output_node_id: String,
    output_connection_index: usize,
}

impl NewConnection {
    pub fn new(input_node_id: String, input_connection_index: usize, output_node_id: String, output_connection_index: usize,) -> NewConnection {
        NewConnection { input_node_id, input_connection_index, output_node_id, output_connection_index }
    }
}
