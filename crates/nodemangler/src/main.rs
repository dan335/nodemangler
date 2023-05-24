#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")] // hide console window on Windows in release

use eframe::egui::{self};
use eframe::epaint::{ColorImage, Rounding};
use egui::Vec2;
use mangler::nodes::operation::{ConnectionSettings, Operation};
use mangler::{
    get_id, AddConnectionMessage, AddNodeMessage, NodeInputChangedMessage,
    NodeOutputChangedMessage, RemoveConnectionMessage, RemoveNodeMessage, SetNodeInputMessage,
};
use mangler::{graph::Graph, nodes::node_settings::NodeSettings};
use std::path::Path;
use tokio::time::{Duration, Instant};
mod graph;
mod menu;
mod node_settings;
mod title_bar;
mod view;
use crate::graph::graph_editor::GraphEditorResponse;
use egui::{Pos2, Rect};
use graph::graph_editor::{GraphEditor};
use menu::menu_panel::MenuPanel;
use node_settings::node_settings_panel::NodeSettingsPanel;
use tokio::sync::mpsc;
use view::view_panel::ViewPanel;

pub const PROFILE: bool = false;
pub const DEFAULT_WINDOW_WIDTH: f32 = 1280.0;
pub const DEFAULT_WINDOW_HEIGHT: f32 = 800.0;
//const ICON: &[u8; 2869] = include_bytes!("..\\assets\\mangler_icon.png");

#[tokio::main]
async fn main() -> Result<(), eframe::Error> {
    puffin::set_scopes_on(PROFILE);

    let icon_path = Path::new(env!("CARGO_MANIFEST_DIR")).join("assets/mangler_icon.png");

    let options = eframe::NativeOptions {
        initial_window_size: Some(egui::vec2(DEFAULT_WINDOW_WIDTH, DEFAULT_WINDOW_HEIGHT)),
        icon_data: Some(load_icon(icon_path.to_str().unwrap())),
        maximized: true,
        ..Default::default()
    };

    let (tx_add_node, mut rx_add_node) = mpsc::channel(32);
    let (tx_remove_node, mut rx_remove_node) = mpsc::channel(32);
    let (tx_add_connection, mut rx_add_connection) = mpsc::channel(32);
    let (tx_remove_connection, mut rx_remove_connection) = mpsc::channel(32);
    let (tx_set_input, mut rx_set_input) = mpsc::channel(32);
    let (tx_input_changed, rx_input_changed) = mpsc::channel(32);
    let (tx_output_changed, rx_output_changed) = mpsc::channel(32);

    let my_app = MyApp::new(
        tx_add_node,
        tx_remove_node,
        tx_add_connection,
        tx_remove_connection,
        tx_set_input,
        rx_input_changed,
        rx_output_changed,
    );

    eframe::run_native(
        "Mangler",
        options,
        Box::new(|_cc| {
            //let frame = cc.egui_ctx.clone();

            tokio::spawn(async move {
                let mut graph = Graph::new(tx_output_changed, tx_input_changed);

                loop {
                    let mut sleep_time = Instant::now() + Duration::from_millis(33);

                    while let Ok(add_node_message) = rx_add_node.try_recv() {
                        graph.add_node(
                            add_node_message.node_id,
                            add_node_message.node_settings,
                            add_node_message.input_settings,
                            add_node_message.output_settings,
                            add_node_message.operation,
                        );
                    }

                    while let Ok(remove_node_message) = rx_remove_node.try_recv() {
                        graph.remove_node(remove_node_message.node_id);
                    }

                    while let Ok(add_connection_message) = rx_add_connection.try_recv() {
                        graph.add_connection(
                            add_connection_message.input_node_id,
                            add_connection_message.input_connection_index,
                            add_connection_message.output_node_id,
                            add_connection_message.output_connection_index,
                        );
                    }

                    while let Ok(remove_connection_message) = rx_remove_connection.try_recv() {
                        graph.remove_connection(
                            remove_connection_message.node_id,
                            remove_connection_message.input_index,
                        );
                    }

                    while let Ok(node_input_message) = rx_set_input.try_recv() {
                        graph.set_input(
                            node_input_message.node_id,
                            node_input_message.input_index,
                            node_input_message.value,
                        );
                    }

                    graph.run().await;

                    sleep_time = sleep_time.max(Instant::now() + Duration::from_millis(5));
                    tokio::time::sleep_until(sleep_time).await;
                }
            });

            Box::<MyApp>::new(my_app)
        }),
    )
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
    tx_add_node: mpsc::Sender<AddNodeMessage>,
    tx_remove_node: mpsc::Sender<RemoveNodeMessage>,
    tx_add_connection: mpsc::Sender<AddConnectionMessage>,
    tx_remove_connection: mpsc::Sender<RemoveConnectionMessage>,
    tx_set_input: mpsc::Sender<SetNodeInputMessage>,
    rx_input_changed: mpsc::Receiver<NodeInputChangedMessage>,
    rx_output_changed: mpsc::Receiver<NodeOutputChangedMessage>,
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

impl eframe::App for MyApp {
    fn update(&mut self, ctx: &egui::Context, frame: &mut eframe::Frame) {
        if PROFILE {
            puffin::profile_function!();
            puffin::GlobalProfiler::lock().new_frame(); // call once per frame!

            puffin_egui::profiler_window(ctx);
        }

        // show app background
        egui::CentralPanel::default().show(ctx, |ui| {
            puffin::profile_scope!("central panel show");

            // messages for when node output changes
            while let Ok(node_output_message) = self.rx_output_changed.try_recv() {
                puffin::profile_scope!("ui receive output messages");
                if let Some(node) = self
                    .graph_editor
                    .graph_nodes
                    .get_mut(&node_output_message.node_id)
                {
                    if let Some(output) = node.outputs.get_mut(node_output_message.output_index) {
                        output.value = node_output_message.value;

                        if node_output_message.output_index == 0 {
                            node.thumbnail = match node_output_message.thumbnail {
                                Some(thumbnail) => {
                                    let pixels = thumbnail.as_flat_samples();
                                    let size =
                                        [thumbnail.width() as usize, thumbnail.height() as usize];
                                    let color_image =
                                        ColorImage::from_rgba_unmultiplied(size, pixels.as_slice());
                                    Some(ui.ctx().load_texture(
                                        node.id.clone(),
                                        color_image,
                                        Default::default(),
                                    ))
                                }
                                None => None,
                            };
                        }

                        // thumbnail
                        //node.is_dirty = true;
                        node.time = Some(node_output_message.time);
                    }
                }
            }

            // messages for when node input changes
            while let Ok(node_input_changed_message) = self.rx_input_changed.try_recv() {
                puffin::profile_scope!("ui receive input messages");
                if let Some(node) = self
                    .graph_editor
                    .graph_nodes
                    .get_mut(&node_input_changed_message.node_id)
                {
                    if let Some(input) = node.inputs.get_mut(node_input_changed_message.input_index)
                    {
                        input.set_value(node_input_changed_message.value);
                    }
                }
            }

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
                puffin::profile_scope!("menu panel");
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
                puffin::profile_scope!("top panel");

                if let Some(viewing_node_id) = &self.viewing_node_id {
                    if let Some(graph_node) = self.graph_editor.graph_nodes.get(viewing_node_id) {
                        self.view_panel.show(ui, Some(graph_node));
                    } else {
                        self.view_panel.show(ui, None);
                    }
                } else {
                    self.view_panel.show(ui, None);
                }
            });

            // -------------------------
            // settings panel - top right
            let settings_panel_rect = Rect::from_two_pos(
                Pos2::new(app_rect.width() - 300.0, 0.0),
                Pos2::new(app_rect.width(), app_rect.height() / 2.0),
            );

            ui.allocate_ui_at_rect(settings_panel_rect, |ui| {
                puffin::profile_scope!("settings panel");
                if let Some(node_id) = &self.editing_node_id {
                    if let Some(graph_node) = self.graph_editor.graph_nodes.get_mut(node_id) {
                        self.node_settings_panel.show(
                            ui,
                            Some(graph_node),
                            self.tx_set_input.clone(),
                        );
                    } else {
                        self.node_settings_panel
                            .show(ui, None, self.tx_set_input.clone());
                    }
                } else {
                    self.node_settings_panel
                        .show(ui, None, self.tx_set_input.clone());
                }
            });

            // -------------------------
            // bottom graph panel
            let bottom_panel_rect = Rect::from_two_pos(
                Pos2::new(0.0, app_rect.height() / 2.0),
                Pos2::new(app_rect.width(), app_rect.height()),
            );

            ui.allocate_ui_at_rect(bottom_panel_rect, |ui| {
                puffin::profile_scope!("graph panel");
                let graph_editor_response: GraphEditorResponse = self.graph_editor.show(
                    ui,
                    cursor_position,
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
                    self.add_connection(
                        new_connection.input_node_id,
                        new_connection.input_connection_index,
                        new_connection.output_node_id,
                        new_connection.output_connection_index,
                    );
                }

                for (node_id, input_index) in graph_editor_response.connections_to_delete.iter() {
                    self.remove_connection(node_id.clone(), input_index.clone());
                }

                for node_id in graph_editor_response.nodes_to_delete.iter() {
                    self.remove_node(node_id.clone());
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
                            let input_sttings = dragging_settings.1.clone();
                            let output_settings = dragging_settings.2.clone();

                            //let node_position_view_space = Pos2::new(cursor_position.x - bottom_panel_rect.min.x, cursor_position.y - bottom_panel_rect.min.y);
//println!("{:?}", cursor_position);
                            self.add_node(
                                node_settings,
                                input_sttings,
                                output_settings,
                                dragging_settings.3.clone(),
                                view_to_graph_space_pos2(self.graph_editor.zoom, cursor_position) - self.graph_editor.position.to_vec2(),
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

            // show help in bottom left
            let pos = Pos2::new(app_rect.left() + 10.0, app_rect.bottom() - 10.0);
            let txt =
                format!("left click: edit      right click: view      ctrl + left click: delete");
            ui.painter().text(
                pos,
                egui::Align2::LEFT_BOTTOM,
                txt,
                egui::FontId::proportional(12.0),
                egui::Color32::from_gray(150),
            );
        });
    }
}

impl MyApp {
    pub fn new(
        tx_add_node: mpsc::Sender<AddNodeMessage>,
        tx_remove_node: mpsc::Sender<RemoveNodeMessage>,
        tx_add_connection: mpsc::Sender<AddConnectionMessage>,
        tx_remove_connection: mpsc::Sender<RemoveConnectionMessage>,
        tx_set_input: mpsc::Sender<SetNodeInputMessage>,
        rx_input_changed: mpsc::Receiver<NodeInputChangedMessage>,
        rx_output_changed: mpsc::Receiver<NodeOutputChangedMessage>,
    ) -> Self {
        Self {
            tx_add_node,
            tx_remove_node,
            tx_add_connection,
            tx_remove_connection,
            tx_set_input,
            rx_input_changed,
            rx_output_changed,
            graph_editor: GraphEditor::new(),
            node_settings_panel: NodeSettingsPanel::new(),
            view_panel: ViewPanel::new(),
            menu_panel: MenuPanel::new(),
            dragging_menu_button: None,
            editing_node_id: None,
            viewing_node_id: None,
        }
    }

    pub fn add_node(
        &mut self,
        node_settings: NodeSettings,
        input_settings: Vec<ConnectionSettings>,
        output_settings: Vec<ConnectionSettings>,
        operation: Operation,
        position_graph_space: Pos2,
    ) {
        let node_id = get_id();

        let add_node_message = AddNodeMessage {
            node_id: node_id.clone(),
            node_settings: node_settings.clone(),
            input_settings: input_settings.clone(),
            output_settings: output_settings.clone(),
            operation: operation.clone(),
        };
println!("main add node {:?}", position_graph_space);
        match self.tx_add_node.try_send(add_node_message) {
            Ok(_) => {
                self.graph_editor.add_node(
                    node_id.clone(),
                    node_settings.clone(),
                    input_settings.clone(),
                    output_settings.clone(),
                    position_graph_space,
                );
            }
            Err(err) => {
                println!("Error sending AddNodeMessage: {:?}", err);
            }
        }
    }

    pub fn remove_node(&mut self, node_id: String) {
        let remove_node_message = RemoveNodeMessage {
            node_id: node_id.clone(),
        };

        match self.tx_remove_node.try_send(remove_node_message) {
            Ok(_) => {
                if self.editing_node_id == Some(node_id.clone()) {
                    self.editing_node_id = None;
                }
                if self.viewing_node_id == Some(node_id.clone()) {
                    self.viewing_node_id = None;
                };
                self.graph_editor.remove_node(&node_id);
            }
            Err(err) => {
                println!("Error sending RemoveNodeMessage: {:?}", err);
            }
        }
    }

    pub fn view_node(&mut self, node_id: String) {
        self.viewing_node_id = Some(node_id);
    }

    pub fn edit_node(&mut self, node_id: String) {
        self.editing_node_id = Some(node_id);
    }

    pub fn add_connection(
        &mut self,
        input_node_id: String,
        input_connection_index: usize,
        output_node_id: String,
        output_connection_index: usize,
    ) {
        let add_connection_message = AddConnectionMessage {
            input_node_id: input_node_id.clone(),
            input_connection_index,
            output_node_id: output_node_id.clone(),
            output_connection_index,
        };

        match self.tx_add_connection.try_send(add_connection_message) {
            Ok(_) => {
                // set output connection
                if let Some(from) = self.graph_editor.graph_nodes.get_mut(&output_node_id) {
                    from.set_output_connection(
                        output_connection_index,
                        input_node_id.clone(),
                        input_connection_index,
                    );

                    //from.is_dirty = true;
                }

                // set input connection
                if let Some(to) = self.graph_editor.graph_nodes.get_mut(&input_node_id) {
                    to.set_input_connection(
                        input_connection_index,
                        output_node_id,
                        output_connection_index,
                    );
                }
            }
            Err(err) => {
                println!("Error sending AddConnectionMessage: {:?}", err);
            }
        }
    }

    pub fn remove_connection(&mut self, node_id: String, input_index: usize) {
        let remove_connection_message = RemoveConnectionMessage {
            node_id,
            input_index,
        };

        match self
            .tx_remove_connection
            .try_send(remove_connection_message)
        {
            Ok(_) => todo!(),
            Err(err) => {
                println!("Error sending RemoveConnectionMessage: {:?}", err);
            }
        }
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


pub fn view_to_graph_space(zoom: f32, n: f32) -> f32 {
    n * zoom
}

pub fn view_to_graph_space_pos2(zoom: f32, n: Pos2) -> Pos2 {
    Pos2::new(view_to_graph_space(zoom, n.x), view_to_graph_space(zoom, n.y))
}

pub fn graph_to_view_space(zoom: f32, n: f32) -> f32 {
    n / zoom
}

pub fn graph_to_view_space_pos2(zoom: f32, n: Pos2) -> Pos2 {
    Pos2::new(graph_to_view_space(zoom, n.x), graph_to_view_space(zoom, n.y))
}