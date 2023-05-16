#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")] // hide console window on Windows in release

use eframe::egui::{self};
use eframe::epaint::Rounding;
use egui::Vec2;
use mangler::get_id;
use mangler::nodes::node::Node;
use mangler::nodes::operation::{ConnectionSettings, Operation};
use mangler::{graph::Graph, nodes::node_settings::NodeSettings};
use std::path::Path;

mod graph;
mod menu;
mod node_settings;
mod title_bar;
mod view;
use crate::graph::graph_editor::GraphEditorResponse;
use egui::{Pos2, Rect};
use graph::graph_editor::GraphEditor;
use menu::menu_panel::MenuPanel;
use node_settings::node_settings_panel::NodeSettingsPanel;
use view::view_panel::ViewPanel;

pub const DEFAULT_WINDOW_WIDTH: f32 = 1280.0;
pub const DEFAULT_WINDOW_HEIGHT: f32 = 800.0;
//const ICON: &[u8; 2869] = include_bytes!("..\\assets\\mangler_icon.png");

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
    //let mut icon_data: Option<IconData> = None;

    let icon_path = Path::new(env!("CARGO_MANIFEST_DIR")).join("assets/mangler_icon.png");
    //icon_data = Some(load_icon(icon_path.to_str().unwrap()));

    let options = eframe::NativeOptions {
        initial_window_size: Some(egui::vec2(DEFAULT_WINDOW_WIDTH, DEFAULT_WINDOW_HEIGHT)),
        icon_data: Some(load_icon(icon_path.to_str().unwrap())),
        maximized: true,
        ..Default::default()
    };

    eframe::run_native("Mangler", options, Box::new(|_cc| Box::<MyApp>::default()))
}

// do this without image crate?
fn load_icon(path: &str) -> eframe::IconData {
    let (icon_rgba, icon_width, icon_height) = {
        let image = image::open(path)
            .expect("Failed to open icon path")
            .into_rgba8();
        let (width, height) = image.dimensions();
        let rgba = image.into_raw();
        (rgba, width, height)
    };

    eframe::IconData {
        rgba: icon_rgba,
        width: icon_width,
        height: icon_height,
    }
}

struct MyApp {
    pub graph: Graph,
    graph_editor: GraphEditor,
    node_settings_panel: NodeSettingsPanel,
    view_panel: ViewPanel,
    menu_panel: MenuPanel,
    dragging_menu_button: Option<(
        NodeSettings,
        Vec<ConnectionSettings>,
        Vec<ConnectionSettings>,
        Operation,
    )>,
    editing_node_id: Option<String>,
    viewing_node_id: Option<String>,
}

impl Default for MyApp {
    fn default() -> Self {
        Self {
            graph: Graph::default(),
            graph_editor: GraphEditor::new(),
            node_settings_panel: NodeSettingsPanel::new(),
            view_panel: ViewPanel::new(),
            menu_panel: MenuPanel::new(),
            dragging_menu_button: None,
            editing_node_id: None,
            viewing_node_id: None,
        }
    }
}

impl eframe::App for MyApp {
    fn update(&mut self, ctx: &egui::Context, frame: &mut eframe::Frame) {
        egui::CentralPanel::default().show(ctx, |ui| {
            let app_rect = ctx.screen_rect();
            let cursor_position = ui
                .ctx()
                .input(|i| i.pointer.hover_pos())
                .unwrap_or(Pos2::ZERO);
            let cursor_primary_down: bool = ui.ctx().input(|i| i.pointer.primary_down());
            let cursor_inside = app_rect.contains(cursor_position);

            //let mouse_response = ui.allocate_rect(app_rect, Sense::drag());

            // -------------------------
            // menu panel
            let menu_panel_rect = Rect::from_two_pos(
                Pos2::new(0.0, 0.0),
                Pos2::new(200.0, app_rect.height() / 2.0),
            );
            ui.allocate_ui_at_rect(menu_panel_rect, |ui| {
                let menu_result = self.menu_panel.show(ui);

                // dragging from menu
                if menu_result.dragging_menu_button.is_some() {
                    self.dragging_menu_button = menu_result.dragging_menu_button;
                }
            });

            // -------------------------
            // top panel
            let top_panel_rect = Rect::from_two_pos(
                Pos2::new(200.0, 0.0),
                Pos2::new(app_rect.width() - 300.0, app_rect.height() / 2.0),
            );
            ui.allocate_ui_at_rect(top_panel_rect, |ui| {
                self.view_panel.show(ui);
            });

            // -------------------------
            // settings panel - top right
            let settings_panel_rect = Rect::from_two_pos(
                Pos2::new(app_rect.width() - 300.0, 0.0),
                Pos2::new(app_rect.width(), app_rect.height() / 2.0),
            );
            ui.allocate_ui_at_rect(settings_panel_rect, |ui| {
                if let Some(node_id) = &self.editing_node_id {
                    if let Some(node) = self.graph.nodes.get_mut(node_id) {
                        let settings_response = self.node_settings_panel.show(
                            ui,
                            Some(&mut node.settings),
                            Some(&mut node.inputs),
                            Some(&node.outputs),
                        );

                        if !settings_response.input_indexes_that_changed.is_empty() {
                            node.is_dirty = true;
                            self.graph.is_dirty = true;
                        }
                    } else {
                        self.node_settings_panel.show(ui, None, None, None);
                    }
                } else {
                    self.node_settings_panel.show(ui, None, None, None);
                }
            });

            // -------------------------
            // bottom graph panel
            let bottom_panel_rect = Rect::from_two_pos(
                Pos2::new(0.0, app_rect.height() / 2.0),
                Pos2::new(app_rect.width(), app_rect.height()),
            );
            ui.allocate_ui_at_rect(bottom_panel_rect, |ui| {
                let graph_editor_response: GraphEditorResponse = self.graph_editor.show(
                    ui,
                    cursor_position,
                    &self.graph.nodes,
                    cursor_primary_down,
                    &self.editing_node_id,
                    &self.viewing_node_id,
                );

                if graph_editor_response.request_redraw {
                    ctx.request_repaint();
                }

                if let Some(editing_node_id) = graph_editor_response.editing_node_id {
                    self.edit_node(editing_node_id);
                }

                if let Some(viewing_node_id) = graph_editor_response.viewing_node_id {
                    self.view_node(viewing_node_id);
                }

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
                    if let Some(dragging_settings) = &self.dragging_menu_button {
                        if bottom_panel_rect.contains(cursor_position) {
                            let node_settings = dragging_settings.0.clone();
                            let input_sttings = &dragging_settings.1.clone();
                            let output_settings = &dragging_settings.2.clone();

                            self.add_node(
                                node_settings,
                                input_sttings,
                                output_settings,
                                dragging_settings.3.clone(),
                                cursor_position,
                            );
                        }
                    }

                    self.dragging_menu_button = None;
                }
            });

            // dragging node from menu
            // draw shape behind mouse being dragged
            if let Some(_dragging_settings) = &self.dragging_menu_button {
                let drag_rect = Rect::from_center_size(cursor_position, Vec2::new(80.0, 80.0));
                ui.painter().add(egui::Shape::rect_filled(
                    drag_rect,
                    Rounding::none(),
                    egui::Color32::from_gray(100),
                ));
            }

            // show cpu usage in bototm right corner
            if let Some(cpu_usage) = frame.info().cpu_usage {
                let pos = Pos2::new(app_rect.right() - 10.0, app_rect.bottom() - 10.0);
                let txt = format!("{:.3} ms", cpu_usage * 1000.0);
                ui.painter().text(
                    pos,
                    egui::Align2::RIGHT_BOTTOM,
                    txt,
                    egui::FontId::monospace(8.0),
                    egui::Color32::from_gray(150),
                );
            }
        });

        if self.graph.is_dirty {
            let changed_nodes = self.graph.run();

            for node_id in changed_nodes.iter() {
                if let Some(graph_node) = self.graph_editor.graph_nodes.get(node_id) {
                    graph_node.thumbnail_is_dirty = true;
                }
            }
        }
    }
}

impl MyApp {
    pub fn connect_nodes(&mut self, new_connection: NewConnection) {
        // if graph contains both nodes
        if self
            .graph
            .nodes
            .get_mut(&new_connection.input_node_id)
            .is_some()
            && self
                .graph
                .nodes
                .get_mut(&new_connection.output_node_id)
                .is_some()
        {
            // set output connection
            if let Some(from) = self.graph.nodes.get_mut(&new_connection.output_node_id) {
                from.set_output_connection(
                    new_connection.output_connection_index,
                    new_connection.input_node_id.clone(),
                    new_connection.input_connection_index,
                );

                from.is_dirty = true;
            }

            // set input connection
            if let Some(to) = self.graph.nodes.get_mut(&new_connection.input_node_id) {
                to.set_input_connection(
                    new_connection.input_connection_index,
                    new_connection.output_node_id,
                    new_connection.output_connection_index,
                );
            }

            // mark graph as dirty
            self.graph.is_dirty = true;
        }
    }

    pub fn add_node(
        &mut self,
        node_settings: NodeSettings,
        input_settings: &[ConnectionSettings],
        output_settings: &[ConnectionSettings],
        operation: Operation,
        position: Pos2,
    ) -> String {
        let id = get_id();
        self.graph_editor
            .add_node(id.clone(), node_settings.clone(), position);
        let node = Node::new(
            id.clone(),
            node_settings,
            input_settings,
            output_settings,
            operation,
        );
        self.graph.add_node(id.clone(), node);
        id
    }

    pub fn view_node(&mut self, node_id: String) {
        self.viewing_node_id = Some(node_id);
    }

    pub fn edit_node(&mut self, node_id: String) {
        self.editing_node_id = Some(node_id);
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
    pub fn new(
        input_node_id: String,
        input_connection_index: usize,
        output_node_id: String,
        output_connection_index: usize,
    ) -> NewConnection {
        NewConnection {
            input_node_id,
            input_connection_index,
            output_node_id,
            output_connection_index,
        }
    }
}
