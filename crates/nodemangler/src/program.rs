use eframe::egui;
use epaint::{ColorImage, Pos2, Rect, Rounding, Vec2};
use mangler::{
    get_id, graph::Graph, operation::Operation, AddConnectionMessage, AddNodeMessage,
    AddedConnectionMessage, AddedNodeMessage, GraphMessage, LoadedNodeMessage, NewGraphError,
    NodeInputChangedMessage, NodeOutputChangedMessage, NodePosition, RemoveConnectionMessage,
    RemoveNodeMessage, RemovedConnectionMessage, RemovedNodeMessage, SetNodeInputMessage,
};
use std::path::PathBuf;
use tokio::{
    sync::mpsc,
    task::JoinHandle,
    time::{Duration, Instant},
};

use crate::{
    graph::{
        graph_editor::{GraphEditor, GraphEditorResponse},
        graph_node::GraphNode,
    },
    menu::menu_panel::MenuPanel,
    settings::{graph_settings_panel, node_settings_panel},
    view::view_panel::ViewPanel,
    view_to_graph_space_pos2, APP_MENU_HEIGHT,
};

pub struct Program {
    pub id: String,
    pub name: String,
    pub save_path: Option<PathBuf>,
    //pub needs_to_save: bool,
    pub thread_handle: JoinHandle<()>,
    tx_add_node: mpsc::Sender<AddNodeMessage>,
    tx_remove_node: mpsc::Sender<RemoveNodeMessage>,
    tx_add_connection: mpsc::Sender<AddConnectionMessage>,
    tx_remove_connection: mpsc::Sender<RemoveConnectionMessage>,
    tx_set_input: mpsc::Sender<SetNodeInputMessage>,
    rx_input_changed: mpsc::Receiver<NodeInputChangedMessage>,
    rx_output_changed: mpsc::Receiver<NodeOutputChangedMessage>,
    rx_added_node: mpsc::Receiver<AddedNodeMessage>,
    rx_removed_node: mpsc::Receiver<RemovedNodeMessage>,
    rx_loaded_node: mpsc::Receiver<LoadedNodeMessage>,
    rx_added_connection: mpsc::Receiver<AddedConnectionMessage>,
    rx_removed_connection: mpsc::Receiver<RemovedConnectionMessage>,
    tx_graph_setting: mpsc::Sender<GraphMessage>,
    tx_node_position: mpsc::Sender<NodePosition>,
    graph_editor: GraphEditor,
    view_panel: ViewPanel,
    menu_panel: MenuPanel,
    editing_node_id: Option<String>,
    viewing_node_id: Option<String>,
    dragging_menu_button: Option<Operation>,
}

impl Program {
    pub fn new(id: Option<String>, save_file: Option<PathBuf>) -> Result<Self, NewGraphError> {
        let (tx_add_node, mut rx_add_node) = mpsc::channel::<AddNodeMessage>(32);
        let (tx_remove_node, mut rx_remove_node) = mpsc::channel::<RemoveNodeMessage>(32);
        let (tx_add_connection, mut rx_add_connection) = mpsc::channel::<AddConnectionMessage>(32);
        let (tx_remove_connection, mut rx_remove_connection) =
            mpsc::channel::<RemoveConnectionMessage>(32);
        let (tx_set_input, mut rx_set_input) = mpsc::channel::<SetNodeInputMessage>(32);
        let (tx_input_changed, rx_input_changed) = mpsc::channel::<NodeInputChangedMessage>(32);
        let (tx_output_changed, rx_output_changed) = mpsc::channel::<NodeOutputChangedMessage>(32);
        let (tx_added_node, rx_added_node) = mpsc::channel::<AddedNodeMessage>(32);
        let (tx_removed_node, rx_removed_node) = mpsc::channel::<RemovedNodeMessage>(32);
        let (tx_loaded_node, rx_loaded_node) = mpsc::channel::<LoadedNodeMessage>(32);
        let (tx_added_connection, rx_added_connection) =
            mpsc::channel::<AddedConnectionMessage>(32);
        let (tx_removed_connection, rx_removed_connection) =
            mpsc::channel::<RemovedConnectionMessage>(32);
        let (tx_graph_setting, mut rx_graph_setting) = mpsc::channel::<GraphMessage>(32);
        let (tx_node_position, mut rx_node_position) = mpsc::channel::<NodePosition>(32);

        let graph_result = match save_file {
            Some(path) => Graph::load(
                path,
                tx_output_changed,
                tx_input_changed,
                tx_added_node,
                tx_removed_node,
                tx_loaded_node,
                tx_added_connection,
                tx_removed_connection,
            ),
            None => {
                let graph_id = match id {
                    Some(graph_id) => graph_id,
                    None => get_id(),
                };

                Graph::new(
                    graph_id,
                    tx_output_changed,
                    tx_input_changed,
                    tx_added_node,
                    tx_removed_node,
                    tx_loaded_node,
                    tx_added_connection,
                    tx_removed_connection,
                )
            }
        };

        match graph_result {
            Ok(mut graph) => {
                let id = graph.id.clone();
                let name = graph.name.clone();
                let save_path = graph.save_path.clone();

                let thread_handle = tokio::spawn(async move {
                    loop {
                        let mut sleep_time = Instant::now() + Duration::from_millis(16);

                        while let Ok(add_node_message) = rx_add_node.try_recv() {
                            graph
                                .add_node(
                                    add_node_message.node_id,
                                    add_node_message.operation,
                                    add_node_message.position,
                                )
                                .await;
                        }

                        while let Ok(remove_node_message) = rx_remove_node.try_recv() {
                            graph.remove_node(remove_node_message.node_id).await;
                        }

                        while let Ok(add_connection_message) = rx_add_connection.try_recv() {
                            graph
                                .add_connection(
                                    add_connection_message.input_node_id,
                                    add_connection_message.input_connection_index,
                                    add_connection_message.output_node_id,
                                    add_connection_message.output_connection_index,
                                )
                                .await;
                        }

                        while let Ok(remove_connection_message) = rx_remove_connection.try_recv() {
                            graph
                                .remove_connection(
                                    remove_connection_message.node_id,
                                    remove_connection_message.input_index,
                                )
                                .await;
                        }

                        while let Ok(node_input_message) = rx_set_input.try_recv() {
                            graph.set_input(
                                node_input_message.node_id,
                                node_input_message.input_index,
                                node_input_message.value,
                            );
                        }

                        while let Ok(graph_message) = rx_graph_setting.try_recv() {
                            match graph_message {
                                GraphMessage::SavePath(save_path) => {
                                    graph.set_save_path(save_path);
                                }
                                GraphMessage::GraphName(name) => {
                                    graph.name = name;
                                    graph.save_to_file();
                                }
                            }
                        }

                        while let Ok(node_position_message) = rx_node_position.try_recv() {
                            graph.set_node_position(
                                node_position_message.node_id,
                                node_position_message.position,
                            );
                        }

                        graph.run().await;

                        sleep_time = sleep_time.max(Instant::now() + Duration::from_millis(2));
                        tokio::time::sleep_until(sleep_time).await;
                    }
                });

                Ok(Program {
                    tx_add_node,
                    tx_remove_node,
                    tx_add_connection,
                    tx_remove_connection,
                    tx_set_input,
                    rx_input_changed,
                    rx_output_changed,
                    id,
                    name,
                    save_path,
                    thread_handle,
                    graph_editor: GraphEditor::new(),
                    view_panel: ViewPanel::new(),
                    menu_panel: MenuPanel::new(),
                    dragging_menu_button: None,
                    editing_node_id: None,
                    viewing_node_id: None,
                    rx_added_node,
                    rx_removed_node,
                    rx_added_connection,
                    rx_removed_connection,
                    tx_graph_setting,
                    tx_node_position,
                    rx_loaded_node,
                })
            }
            Err(error) => Err(NewGraphError(format!(
                "Error creating new graph.  Error: {:?}",
                error
            ))),
        }
    }

    pub fn close(self) {
        self.thread_handle.abort();
    }

    pub fn show(&mut self, ctx: &egui::Context, frame: &mut eframe::Frame, ui: &mut egui::Ui) {
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
                if let Some(input) = node.inputs.get_mut(node_input_changed_message.input_index) {
                    input.value = node_input_changed_message.value;
                    //self.needs_to_save = true;
                }
            }
        }

        while let Ok(added_node_message) = self.rx_added_node.try_recv() {
            self.graph_editor.add_node(
                added_node_message.node_id.clone(),
                added_node_message.operation,
                Pos2::new(added_node_message.position.x, added_node_message.position.y),
            );
            //self.needs_to_save = true;
        }

        while let Ok(removed_node_message) = self.rx_removed_node.try_recv() {
            if self.editing_node_id == Some(removed_node_message.node_id.clone()) {
                self.editing_node_id = None;
            }
            if self.viewing_node_id == Some(removed_node_message.node_id.clone()) {
                self.viewing_node_id = None;
            };
            self.graph_editor.remove_node(&removed_node_message.node_id);
            //self.needs_to_save = true;
        }

        while let Ok(loaded_node_message) = self.rx_loaded_node.try_recv() {
            let graph_node = GraphNode {
                id: loaded_node_message.node.id.clone(),
                position: Pos2::new(
                    loaded_node_message.node.position.x,
                    loaded_node_message.node.position.y,
                ),
                settings: loaded_node_message.node.settings,
                inputs: loaded_node_message.node.inputs,
                outputs: loaded_node_message.node.outputs,
                time: None,
                is_dragging: false,
                last_drag_position: None,
                thumbnail: None,
            };

            self.graph_editor
                .graph_nodes
                .insert(loaded_node_message.node.id, graph_node);
        }

        while let Ok(added_connection_message) = self.rx_added_connection.try_recv() {
            // set output connection
            if let Some(from) = self
                .graph_editor
                .graph_nodes
                .get_mut(&added_connection_message.output_node_id)
            {
                from.set_output_connection(
                    added_connection_message.output_connection_index,
                    added_connection_message.input_node_id.clone(),
                    added_connection_message.input_connection_index,
                );

                //from.is_dirty = true;
            }

            // set input connection
            if let Some(to) = self
                .graph_editor
                .graph_nodes
                .get_mut(&added_connection_message.input_node_id)
            {
                to.set_input_connection(
                    added_connection_message.input_connection_index,
                    added_connection_message.output_node_id,
                    added_connection_message.output_connection_index,
                );
            }

            //self.needs_to_save = true;
        }

        while let Ok(removed_connection_message) = self.rx_removed_connection.try_recv() {
            let mut output: Option<(String, usize)> = None;

            if let Some(node) = self
                .graph_editor
                .graph_nodes
                .get_mut(&removed_connection_message.node_id)
            {
                if let Some((output_node_id, output_index)) =
                    &node.inputs[removed_connection_message.input_index].connection
                {
                    output = Some((output_node_id.clone(), *output_index));
                }

                node.clear_input_connection(removed_connection_message.input_index);
                //node.inputs[input_index].connection = None;
            }

            if let Some((output_node_id, output_index)) = output {
                if let Some(node) = self.graph_editor.graph_nodes.get_mut(&output_node_id) {
                    if let Some(c) = node.outputs.get_mut(output_index) {
                        let d = c.connection.as_mut().unwrap();
                        d.remove(output_index);
                    }
                }
            }

            //self.needs_to_save = true;
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
            Pos2::new(0.0, APP_MENU_HEIGHT),
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
            Pos2::new(200.0, APP_MENU_HEIGHT),
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
            Pos2::new(app_rect.width() - 300.0, APP_MENU_HEIGHT),
            Pos2::new(app_rect.width(), app_rect.height() / 2.0),
        );

        ui.allocate_ui_at_rect(settings_panel_rect, |ui| {
            puffin::profile_scope!("settings panel");

            let left_top = ui.max_rect().left_top();
            let right_bottom = ui.max_rect().right_bottom();
            let padding = 10.0;

            // create rect for content
            let ui_rect = egui::Rect::from_two_pos(
                egui::Pos2::new(left_top.x + padding, left_top.y + padding),
                egui::Pos2::new(right_bottom.x - padding, right_bottom.y - padding),
            );

            ui.allocate_ui_at_rect(ui_rect, |ui| {
                let mut show_graph_settings = true;

                // show node settings
                if let Some(editing_node_id) = &self.editing_node_id {
                    if let Some(node) = self.graph_editor.graph_nodes.get_mut(editing_node_id) {
                        node_settings_panel::show(ui, node, self.tx_set_input.clone());
                        show_graph_settings = false;
                    }
                }

                if show_graph_settings {
                    let graph_settings_response =
                        graph_settings_panel::show(ui, &mut self.name, &self.save_path);

                    // name changed
                    if let Some(new_name) = graph_settings_response.new_name {
                        self.name = new_name.clone();

                        let graph_message = GraphMessage::GraphName(new_name);

                        match self.tx_graph_setting.try_send(graph_message) {
                            Ok(_) => {}
                            Err(err) => {
                                println!("Error sending graph_message: {:?}", err);
                            }
                        }
                    }

                    // save path changed
                    if let Some(save_path) = graph_settings_response.new_save_path {
                        self.save_path = Some(save_path.clone());

                        let graph_message = GraphMessage::SavePath(save_path);

                        match self.tx_graph_setting.try_send(graph_message) {
                            Ok(_) => {}
                            Err(err) => {
                                println!("Error sending graph_message: {:?}", err);
                            }
                        }
                    }
                }
            });
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

            if let Some(new_node_position) = graph_editor_response.new_node_position {
                let node_position_message = NodePosition {
                    node_id: new_node_position.0,
                    position: glam::f32::vec2(new_node_position.1.x, new_node_position.1.y),
                };

                match self.tx_node_position.try_send(node_position_message) {
                    Ok(_) => {}
                    Err(error) => {
                        println!("Error sending node position message. {:?}", error);
                    }
                }
            }

            if graph_editor_response.request_redraw {
                ctx.request_repaint();
            }

            if let Some(editing_node_id) = graph_editor_response.editing_node_id {
                self.edit_node(editing_node_id);
            }

            if let Some(viewing_node_id) = graph_editor_response.viewing_node_id {
                self.view_node(viewing_node_id);
            }

            if graph_editor_response.clear_editing_node {
                self.editing_node_id = None;
            }

            if graph_editor_response.clear_viewing_node {
                self.viewing_node_id = None;
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
                self.remove_connection(node_id.clone(), *input_index);
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
                if let Some(operation) = &self.dragging_menu_button {
                    if bottom_panel_rect.contains(cursor_position) {
                        //let node_position_view_space = Pos2::new(cursor_position.x - bottom_panel_rect.min.x, cursor_position.y - bottom_panel_rect.min.y);
                        //println!("{:?}", cursor_position);
                        self.add_node(
                            operation.clone(),
                            view_to_graph_space_pos2(self.graph_editor.zoom, cursor_position)
                                - self.graph_editor.position.to_vec2(),
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
        let txt = "left click: edit      right click: view      ctrl + left click: delete".to_string();
        ui.painter().text(
            pos,
            egui::Align2::LEFT_BOTTOM,
            txt,
            egui::FontId::proportional(12.0),
            egui::Color32::from_gray(150),
        );
    }

    pub fn add_node(&mut self, operation: Operation, position_graph_space: Pos2) {
        let node_id = get_id();

        let add_node_message = AddNodeMessage {
            node_id,
            operation,
            position: glam::f32::Vec2::new(position_graph_space.x, position_graph_space.y),
        };

        match self.tx_add_node.try_send(add_node_message) {
            Ok(_) => {}
            Err(err) => {
                println!("Error sending AddNodeMessage: {:?}", err);
            }
        }
    }

    // pub fn save_to_file(&mut self) {
    //     if let Some(path) = &self.save_path {
    //         for node in self.graph_editor.graph_nodes.iter() {
    //             //println!("{:?}", toml::ser::to_string(&node));
    //             println!("{:?}", serde_json::to_string(&node));
    //         }
    //     }
    // }

    pub fn remove_node(&mut self, node_id: String) {
        let remove_node_message = RemoveNodeMessage {
            node_id,
        };

        match self.tx_remove_node.try_send(remove_node_message) {
            Ok(_) => {}
            Err(err) => {
                println!("Error sending RemoveNodeMessage: {:?}", err);
            }
        }
    }

    pub fn view_node(&mut self, node_id: String) {
        self.viewing_node_id = Some(node_id);
        //self.needs_to_save = true;
    }

    pub fn edit_node(&mut self, node_id: String) {
        self.editing_node_id = Some(node_id);
        //self.needs_to_save = true;
    }

    pub fn add_connection(
        &mut self,
        input_node_id: String,
        input_connection_index: usize,
        output_node_id: String,
        output_connection_index: usize,
    ) {
        let add_connection_message = AddConnectionMessage {
            input_node_id,
            input_connection_index,
            output_node_id,
            output_connection_index,
        };

        match self.tx_add_connection.try_send(add_connection_message) {
            Ok(_) => {}
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
            Ok(_) => {}
            Err(err) => {
                println!("Error sending RemoveConnectionMessage: {:?}", err);
            }
        }
    }
}
