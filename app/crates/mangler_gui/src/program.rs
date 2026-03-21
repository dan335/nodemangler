use eframe::egui;
use epaint::{Color32, ColorImage, CornerRadius, Pos2, Rect};
use mangler_core::{
    get_id,
    node_type::NodeType,
    value::{Value, ValueType},
    AddNodeType, ChangeGraphMessage, ChangeNodeMessage, GraphChangedMessage, NewGraphError,
    NodeChangedMessage,
};
use std::path::PathBuf;
use std::time::Duration;
use tokio::sync::mpsc;

use crate::{
    graph::{
        graph_editor::{GraphEditor, GraphEditorResponse, TempConnection},
        graph_node::GraphNode,
        graph_node::ConnectionType,
        graph_node_thumbnail::GraphNodeThumbnail,
        node_search_popup::NodeSearchPopup,
    },
    graph_to_view_space,
    node_menu::{menu_item::MenuItemsResult, menu_panel::MenuPanel},
    settings::{graph_settings_panel, node_settings_panel},
    themes::theme::Theme,
    view_to_graph_space_pos2,
    view_window::view_panel::ViewPanel,
    ManglerError, APP_MENU_HEIGHT, NODE_MENU_WIDTH, NODE_SIZE, SETTINGS_PANEL_WIDTH,
};

pub struct Program {
    pub app: mangler_core::app::App,
    tx_change_graph: mpsc::Sender<ChangeGraphMessage>,
    tx_change_node: mpsc::Sender<ChangeNodeMessage>,
    rx_node_changed: mpsc::Receiver<NodeChangedMessage>,
    rx_graph_changed: mpsc::Receiver<GraphChangedMessage>,
    graph_editor: GraphEditor,
    view_panel: ViewPanel,
    menu_panel: MenuPanel,
    editing_node_id: Option<String>,
    viewing_node_id_index: Option<(String, usize)>, // id and output index
    dragging_menu_button: MenuItemsResult,
    pointer_position: Pos2,
    graph_run_time: Duration,
    node_search_popup: NodeSearchPopup,
}

impl Program {
    pub fn new(id: Option<String>, save_file: Option<PathBuf>) -> Result<Self, NewGraphError> {
        let (tx_change_graph, rx_change_graph) = mpsc::channel::<ChangeGraphMessage>(256);
        let (tx_change_node, rx_change_node) = mpsc::channel::<ChangeNodeMessage>(1024);
        let (tx_node_changed, rx_node_changed) = mpsc::channel::<NodeChangedMessage>(256);
        let (tx_graph_changed, rx_graph_changed) = mpsc::channel::<GraphChangedMessage>(256);

        let app_result = mangler_core::app::App::new(
            id,
            save_file,
            rx_change_graph,
            rx_change_node,
            tx_node_changed,
            tx_graph_changed,
        );

        match app_result {
            Ok(app) => Ok(Program {
                tx_change_graph,
                app,
                graph_editor: GraphEditor::new(),
                view_panel: ViewPanel::new(),
                menu_panel: MenuPanel::new(),
                dragging_menu_button: MenuItemsResult::default(),
                editing_node_id: None,
                viewing_node_id_index: None,
                rx_node_changed,
                tx_change_node,
                rx_graph_changed,
                pointer_position: Pos2::ZERO,
                graph_run_time: Duration::ZERO,
                node_search_popup: NodeSearchPopup::new(),
            }),
            Err(error) => Err(NewGraphError(format!(
                "Error creating program. {:?}",
                error
            ))),
        }
    }

    pub fn close(self) {
        self.app.thread_handle.abort();
    }

    pub fn show(
        &mut self,
        ctx: &egui::Context,
        ui: &mut egui::Ui,
        theme: &Theme,
        view_in_separate_window: bool,
    ) {
        puffin::profile_scope!("central panel show");

        while let Ok(graph_changed_message) = self.rx_graph_changed.try_recv() {
            match graph_changed_message {
                GraphChangedMessage::AddedNode {
                    node_id,
                    settings,
                    inputs,
                    outputs,
                    position,
                    is_subgraph,
                } => {
                    self.graph_editor.add_node(
                        node_id,
                        settings,
                        inputs,
                        outputs,
                        Pos2::new(position.x, position.y),
                        is_subgraph,
                    );

                    //self.needs_to_save = true;
                }
                GraphChangedMessage::LoadedNode { node } => {
                    let mut is_subgraph = false;

                    if let NodeType::Subgraph {
                        path: _,
                        graph: _,
                        rx_node_changed: _,
                    } = node.node_type
                    {
                        is_subgraph = true;
                    }

                    let mut graph_node = GraphNode::new(
                        node.id.clone(),
                        Pos2::new(node.position.x, node.position.y),
                        node.settings,
                        node.inputs,
                        node.outputs,
                        is_subgraph,
                    );
                    graph_node.is_enabled = node.is_enabled;

                    self.graph_editor.graph_nodes.insert(node.id, graph_node);
                }
                GraphChangedMessage::RemovedNode { node_id } => {
                    if self.editing_node_id.as_ref() == Some(&node_id) {
                        self.editing_node_id = None;
                    }
                    if self.viewing_node_id_index.as_ref().map(|(id, _)| id) == Some(&node_id) {
                        self.viewing_node_id_index = None;
                    }
                    self.graph_editor.remove_node(&node_id);
                    //self.needs_to_save = true;
                }
                GraphChangedMessage::AddedConnection {
                    input_node_id,
                    input_connection_index,
                    output_node_id,
                    output_connection_index,
                } => {
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

                    //self.needs_to_save = true;
                }
                GraphChangedMessage::RemovedConnection {
                    node_id,
                    input_index,
                } => {
                    let mut output: Option<(String, usize)> = None;

                    if let Some(node) = self.graph_editor.graph_nodes.get_mut(&node_id) {
                        if let Some((output_node_id, output_index)) =
                            &node.inputs[input_index].connection
                        {
                            output = Some((output_node_id.clone(), *output_index));
                        }

                        node.clear_input_connection(input_index);
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
            }
        }

        // Auto-layout nodes if they're all stacked at the same position
        // (e.g. graphs created from the CLI where all nodes default to origin).
        let moved_nodes = self.graph_editor.auto_layout_if_needed();
        for (node_id, new_pos) in moved_nodes {
            let message = ChangeNodeMessage::SetPosition {
                node_id,
                position: glam::f32::vec2(new_pos.x, new_pos.y),
            };
            let _ = self.tx_change_node.try_send(message);
        }

        while let Ok(node_changed_message) = self.rx_node_changed.try_recv() {
            match node_changed_message {
                NodeChangedMessage::InputChanged {
                    node_id,
                    input_index,
                    value,
                } => {
                    if let Some(node) = self.graph_editor.graph_nodes.get_mut(&node_id) {
                        if let Some(input) = node.inputs.get_mut(input_index) {
                            input.value = value;
                            //self.needs_to_save = true;
                        }
                    }
                }

                NodeChangedMessage::InputErrorChanged {
                    node_id,
                    input_index,
                    is_error,
                    message,
                } => {
                    if let Some(node) = self.graph_editor.graph_nodes.get_mut(&node_id) {
                        if let Some(input) = node.inputs.get_mut(input_index) {
                            input.is_error = is_error;
                            input.error_message = message;
                        }
                    }
                }

                NodeChangedMessage::OutputChanged {
                    node_id,
                    output_index,
                    value,
                    thumbnail,
                } => {
                    if let Some(node) = self.graph_editor.graph_nodes.get_mut(&node_id) {
                        if let Some(output) = node.outputs.get_mut(output_index) {
                            output.value = value.clone();
                            if output_index == 0 {
                                node.thumbnail = match thumbnail {
                                    Some(thumb) => match thumb {
                                        mangler_core::thumbnail::Thumbnail::Image(thumbnail) => {
                                            match value {
                                                Value::Color(_) => {
                                                    let pixels = thumbnail.as_flat_samples();

                                                    let size = [
                                                        thumbnail.width() as usize,
                                                        thumbnail.height() as usize,
                                                    ];

                                                    let color_image =
                                                        ColorImage::from_rgba_unmultiplied(
                                                            size,
                                                            pixels.as_slice(),
                                                        );

                                                    Some(GraphNodeThumbnail::Color {
                                                        texture_handle: ui.ctx().load_texture(
                                                            node.id.clone(),
                                                            color_image,
                                                            Default::default(),
                                                        ),
                                                    })
                                                }
                                                Value::DynamicImage { data, change_id: _ } => {
                                                    let pixels = thumbnail.as_flat_samples();

                                                    let size = [
                                                        thumbnail.width() as usize,
                                                        thumbnail.height() as usize,
                                                    ];

                                                    let color_image =
                                                        ColorImage::from_rgba_unmultiplied(
                                                            size,
                                                            pixels.as_slice(),
                                                        );

                                                    // color format
                                                    let bits = data.color().bits_per_pixel()
                                                        / data.color().channel_count() as u16;
                                                    let channels =
                                                        match data.color().channel_count() {
                                                            1 => "r".to_string(),
                                                            2 => "rg".to_string(),
                                                            3 => "rgb".to_string(),
                                                            4 => "rgba".to_string(),
                                                            _ => "".to_string(),
                                                        };

                                                    Some(GraphNodeThumbnail::Image {
                                                        texture_handle: ui.ctx().load_texture(
                                                            node.id.clone(),
                                                            color_image,
                                                            Default::default(),
                                                        ),
                                                        width: data.width(),
                                                        height: data.height(),
                                                        channels,
                                                        bits,
                                                    })
                                                }
                                                _ => None,
                                            }
                                        }
                                        mangler_core::thumbnail::Thumbnail::Text(v) => {
                                            Some(GraphNodeThumbnail::Text(v))
                                        }
                                    },
                                    None => Some(GraphNodeThumbnail::Text("None".to_string())),
                                };
                            }
                        }
                    }
                }

                NodeChangedMessage::ExposeInputChanged {
                    node_id,
                    input_index,
                    set_to,
                } => {
                    if let Some(node) = self.graph_editor.graph_nodes.get_mut(&node_id) {
                        if let Some(input) = node.inputs.get_mut(input_index) {
                            input.is_exposed = set_to;
                        }
                    }
                }
                NodeChangedMessage::ExposeOutputChanged {
                    node_id,
                    output_index,
                    set_to,
                } => {
                    if let Some(node) = self.graph_editor.graph_nodes.get_mut(&node_id) {
                        if let Some(output) = node.outputs.get_mut(output_index) {
                            output.is_exposed = set_to;
                        }
                    }
                }
                NodeChangedMessage::SubgraphLoaded {
                    node_id,
                    settings,
                    inputs,
                    outputs,
                } => {
                    if let Some(node) = self.graph_editor.graph_nodes.get_mut(&node_id) {
                        node.settings = settings;
                        node.inputs = inputs;
                        node.outputs = outputs;
                    }
                }
                NodeChangedMessage::Busy { node_id, is_busy } => {
                    if let Some(node) = self.graph_editor.graph_nodes.get_mut(&node_id) {
                        node.is_busy = is_busy;
                    }
                }
                NodeChangedMessage::InfoChanged { node_id, time } => {
                    if let Some(node) = self.graph_editor.graph_nodes.get_mut(&node_id) {
                        node.time = Some(time);
                    }
                }
                NodeChangedMessage::GraphRunCompleted { total_time } => {
                    self.graph_run_time = total_time;
                }
                NodeChangedMessage::Error {
                    node_id,
                    is_error,
                    message,
                } => {
                    if let Some(node) = self.graph_editor.graph_nodes.get_mut(&node_id) {
                        node.is_error = is_error;
                        node.error_message = message;
                    }
                }
            }
        }

        let app_rect = ctx.content_rect();

        if let Some(pos) = ctx.pointer_latest_pos() {
            self.pointer_position = pos;
        }

        // dropped files
        // can't figure out  how to get pointer position
        // so just put in middle of screen
        ctx.input(|i| {
            if !i.raw.dropped_files.is_empty() {
                for file in i.raw.dropped_files.iter() {
                    if let Some(path) = &file.path {
                        if let Some(extension) = path.extension() {
                            if let Ok(ext) = extension.to_os_string().into_string() {
                                for value_type in ValueType::types().iter() {
                                    if ValueType::file_extensions(value_type).contains(&ext.to_lowercase()) {
                                        match value_type {
                                            ValueType::DynamicImage => {
                                                let random_size = app_rect.width().min(app_rect.height()) * 0.3;
                                                let x = app_rect.center().x + fastrand::f32() * random_size - random_size * 0.5;
                                                let y = app_rect.center().y + fastrand::f32() * random_size - random_size * 0.5;
                                                let pos = view_to_graph_space_pos2(self.graph_editor.zoom, Pos2::new(x, y)) - self.graph_editor.position.to_vec2();
                                                if let Ok(node_id) = self.add_node(AddNodeType::Operation(mangler_core::operations::Operation::OpImageInputFile), pos) {

                                                    let message = ChangeNodeMessage::SetInput { node_id, input_index: 0, value: Value::Path(path.clone()) };

                                                    match self.tx_change_node.try_send(message) {
                                                        Ok(_) => {}
                                                        Err(err) => {
                                                            println!("Error sending graph_message: {:?}", err);
                                                        }
                                                    }
                                                }
                                            },
                                            _ => {},
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        });

        let cursor_primary_down: bool = ui.ctx().input(|i| i.pointer.primary_down());

        let cursor_inside = app_rect.contains(self.pointer_position);

        let menu_panel_rect = Rect::from_two_pos(
            Pos2::new(0.0, APP_MENU_HEIGHT),
            Pos2::new(NODE_MENU_WIDTH, app_rect.height()),
        );

        let node_graph_rect = Rect::from_two_pos(
            Pos2::new(NODE_MENU_WIDTH, APP_MENU_HEIGHT),
            Pos2::new(app_rect.width() - SETTINGS_PANEL_WIDTH, app_rect.height()),
        );

        let settings_panel_rect = Rect::from_two_pos(
            Pos2::new(app_rect.width() - SETTINGS_PANEL_WIDTH, APP_MENU_HEIGHT),
            Pos2::new(app_rect.width(), app_rect.height()),
        );

        // -------------------------
        // menu panel

        ui.scope_builder(egui::UiBuilder::new().max_rect(menu_panel_rect), |ui| {
            puffin::profile_scope!("menu panel");
            let r = self.menu_panel.show(ui, theme);

            if r.subgraph_being_created {
                self.dragging_menu_button.subgraph_being_created = true;
            }

            if r.operation_being_created.is_some() {
                self.dragging_menu_button.operation_being_created = r.operation_being_created;
            }
        });

        // -------------------------
        // settings panel - top right

        ui.scope_builder(egui::UiBuilder::new().max_rect(settings_panel_rect), |ui| {
            puffin::profile_scope!("settings panel");

            let left_top = ui.max_rect().left_top();
            let right_bottom = ui.max_rect().right_bottom();
            let padding = 10.0;

            // create rect for content
            let ui_rect = egui::Rect::from_two_pos(
                egui::Pos2::new(left_top.x + padding, left_top.y + padding),
                egui::Pos2::new(right_bottom.x - padding, right_bottom.y - padding),
            );

            ui.scope_builder(egui::UiBuilder::new().max_rect(ui_rect), |ui| {
                let mut show_graph_settings = true;

                // show node settings
                if let Some(editing_node_id) = &self.editing_node_id {
                    if let Some(node) = self.graph_editor.graph_nodes.get_mut(editing_node_id) {
                        let node_settings_response =
                            node_settings_panel::show(ui, node, &self.tx_change_node, theme);
                        show_graph_settings = false;

                        if node_settings_response.deselect_node {
                            self.editing_node_id = None;
                        }
                    }
                }

                if show_graph_settings {
                    let graph_settings_response =
                        graph_settings_panel::show(ui, &mut self.app.name, &self.app.save_path);

                    // name changed
                    if let Some(new_name) = graph_settings_response.new_name {
                        self.app.name = new_name.clone();

                        let message = ChangeGraphMessage::SetGraphName(new_name);

                        match self.tx_change_graph.try_send(message) {
                            Ok(_) => {}
                            Err(err) => {
                                println!("Error sending graph_message: {:?}", err);
                            }
                        }
                    }

                    // save path changed
                    if let Some(save_path) = graph_settings_response.new_save_path {
                        self.app.save_path = Some(save_path.clone());

                        let message = ChangeGraphMessage::SetSavePath(save_path);

                        match self.tx_change_graph.try_send(message) {
                            Ok(_) => {}
                            Err(err) => {
                                println!("Error sending graph_message: {:?}", err);
                            }
                        }
                    }
                }
            });
        });

        let mut is_mouse_over_viewer = false;

        if let Some((viewing_node_id, graph_node_output_index)) = &self.viewing_node_id_index {
            if let Some(graph_node) = self.graph_editor.graph_nodes.get(viewing_node_id) {
                let view_panel_response = self.view_panel.show(
                    ctx,
                    graph_node,
                    *graph_node_output_index,
                    theme,
                    view_in_separate_window,
                    self.pointer_position,
                );

                if !view_in_separate_window && view_panel_response.is_mouse_over {
                    is_mouse_over_viewer = true;
                }

                if self.view_panel.close_window {
                    self.viewing_node_id_index = None;
                }
            }
        }

        // -------------------------

        ui.scope_builder(egui::UiBuilder::new().max_rect(node_graph_rect), |ui| {
            puffin::profile_scope!("graph panel");
            let graph_editor_response: GraphEditorResponse = self.graph_editor.show(
                ui,
                self.pointer_position,
                cursor_primary_down,
                &self.editing_node_id,
                &self.viewing_node_id_index,
                theme,
                is_mouse_over_viewer,
            );

            if let Some(new_node_position) = graph_editor_response.new_node_position {
                let node_position_message = ChangeNodeMessage::SetPosition {
                    node_id: new_node_position.0,
                    position: glam::f32::vec2(new_node_position.1.x, new_node_position.1.y),
                };

                match self.tx_change_node.try_send(node_position_message) {
                    Ok(_) => {}
                    Err(error) => {
                        println!("Error sending node position message. {:?}", error);
                    }
                }
            }

            // if graph_editor_response.request_redraw {
            //     ctx.request_repaint();
            // }

            if let Some(editing_node_id) = graph_editor_response.editing_node_id {
                self.edit_node(editing_node_id);
            }

            if let Some((viewing_node_id, viewing_output_index)) =
                graph_editor_response.viewing_node_id_index
            {
                self.view_node(viewing_node_id, viewing_output_index);
            }

            if graph_editor_response.clear_editing_node {
                self.editing_node_id = None;
            }

            if graph_editor_response.clear_viewing_node {
                self.viewing_node_id_index = None;
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

            // Open search popup when a connection is dropped on empty space
            if let Some(dropped) = graph_editor_response.dropped_connection {
                self.node_search_popup.open(self.pointer_position, Some(dropped));
            }
        });

        // Open search popup on Tab key (only when popup isn't already open)
        if !self.node_search_popup.is_open
            && node_graph_rect.contains(self.pointer_position)
        {
            let tab_pressed = ctx.input(|i| i.key_pressed(egui::Key::Tab));
            if tab_pressed {
                self.node_search_popup
                    .open(self.pointer_position, None);
            }
        }

        // Show the search popup and handle selection
        if self.node_search_popup.is_open {
            let popup_response = self.node_search_popup.show(ctx);

            if let Some(operation) = popup_response.selected_operation {
                let graph_pos = view_to_graph_space_pos2(
                    self.graph_editor.zoom,
                    self.node_search_popup.position,
                ) - self.graph_editor.position.to_vec2();

                // Store connection info before closing popup
                let from_connection = self.node_search_popup.from_connection.clone();

                if let Ok(new_node_id) =
                    self.add_node(AddNodeType::Operation(operation.clone()), graph_pos)
                {
                    self.edit_node(new_node_id.clone());

                    // Auto-connect if opened from a dropped connection
                    if let Some(conn) = from_connection {
                        self.auto_connect_node(&new_node_id, &operation, &conn);
                    }
                }
            }

            if popup_response.closed {
                self.node_search_popup.close();
            }
        }

        // dragging from menu
        // mouse leaves app
        // stop dragging
        if !cursor_inside {
            self.dragging_menu_button.operation_being_created = None;
            self.dragging_menu_button.subgraph_being_created = false;
        }

        // release mouse button after dragging menu button
        ui.input(|i| {
            if i.pointer.primary_released() {
                if let Some(operation) = &self.dragging_menu_button.operation_being_created {
                    if node_graph_rect.contains(self.pointer_position) {
                        //let node_position_view_space = Pos2::new(cursor_position.x - bottom_panel_rect.min.x, cursor_position.y - bottom_panel_rect.min.y);
                        if let Ok(node_id) = self.add_node(
                            AddNodeType::Operation(operation.clone()),
                            view_to_graph_space_pos2(self.graph_editor.zoom, self.pointer_position)
                                - self.graph_editor.position.to_vec2(),
                        ) {
                            self.edit_node(node_id);
                        }
                    }
                } else if self.dragging_menu_button.subgraph_being_created {
                    if node_graph_rect.contains(self.pointer_position) {
                        if let Ok(node_id) = self.add_node(
                            AddNodeType::Subgraph,
                            view_to_graph_space_pos2(self.graph_editor.zoom, self.pointer_position)
                                - self.graph_editor.position.to_vec2(),
                        ) {
                            self.edit_node(node_id);
                        }
                    }
                }

                self.dragging_menu_button = MenuItemsResult::default();
            }
        });

        // dragging node from menu
        // draw shape behind mouse being dragged
        if self.dragging_menu_button.subgraph_being_created
            || self.dragging_menu_button.operation_being_created.is_some()
        {
            let mut name = "".to_string();

            if let Some(op) = &self.dragging_menu_button.operation_being_created {
                name = op.settings().name.clone();
            } else if self.dragging_menu_button.subgraph_being_created {
                name = "subgraph".to_string();
            }

            let drag_rect = Rect::from_center_size(self.pointer_position, NODE_SIZE);

            ui.painter().add(egui::Shape::rect_filled(
                drag_rect,
                CornerRadius::ZERO,
                theme.get().node_header_bg,
            ));

            // node name
            ui.painter().text(
                drag_rect.center(),
                egui::Align2::CENTER_CENTER,
                name,
                //egui::style::Style::text_styles(),
                egui::FontId::proportional(graph_to_view_space(self.graph_editor.zoom, 14.0)),
                Color32::from(theme.get().override_text_color),
            );
        }

        // show timing in bottom right corner
        {
            let graph_ms = self.graph_run_time.as_secs_f64() * 1000.0;
            let graph_txt = format!("graph: {:.1}ms", graph_ms);
            let pos = Pos2::new(app_rect.right() - 10.0, app_rect.bottom() - 10.0);
            ui.painter().text(
                pos,
                egui::Align2::RIGHT_BOTTOM,
                graph_txt,
                egui::FontId::monospace(10.0),
                egui::Color32::from(theme.get().text_faint),
            );
        }

        // show help in bottom left
        let pos = Pos2::new(
            app_rect.left() + NODE_MENU_WIDTH + 20.0,
            app_rect.bottom() - 10.0,
        );
        let txt =
            "left click: edit      right click: view      ctrl + left click: delete".to_string();
        ui.painter().text(
            pos,
            egui::Align2::LEFT_BOTTOM,
            txt,
            egui::FontId::proportional(12.0),
            egui::Color32::from(theme.get().text_faint),
        );

        // // if a node is busy request redraw
        // for (_, node) in self.graph_editor.graph_nodes.iter() {
        //     if node.is_busy {
        //         ctx.request_repaint();
        //         break;
        //     }
        // }
    }

    pub fn add_node(
        &mut self,
        node_type: AddNodeType,
        position_graph_space: Pos2,
    ) -> Result<String, ManglerError> {
        let node_id = get_id();

        let add_node_message = ChangeGraphMessage::AddNode {
            node_id: node_id.clone(),
            node_type,
            position: glam::f32::Vec2::new(position_graph_space.x, position_graph_space.y),
        };

        match self.tx_change_graph.try_send(add_node_message) {
            Ok(_) => Ok(node_id),
            Err(err) => Err(ManglerError(format!("{:?}", err))),
        }
    }

    pub fn remove_node(&mut self, node_id: String) {
        let remove_node_message = ChangeGraphMessage::RemoveNode { node_id };

        match self.tx_change_graph.try_send(remove_node_message) {
            Ok(_) => {}
            Err(err) => {
                println!("Error sending RemoveNodeMessage: {:?}", err);
            }
        }
    }

    pub fn view_node(&mut self, node_id: String, output_index: usize) {
        self.viewing_node_id_index = Some((node_id, output_index));
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
        let message = ChangeGraphMessage::AddConnection {
            input_node_id,
            input_connection_index,
            output_node_id,
            output_connection_index,
        };

        match self.tx_change_graph.try_send(message) {
            Ok(_) => {}
            Err(err) => {
                println!("Error sending ChangeGraphMessage::AddConnection: {:?}", err);
            }
        }
    }

    pub fn remove_connection(&mut self, node_id: String, input_index: usize) {
        let message = ChangeGraphMessage::RemoveConnection {
            node_id,
            input_index,
        };

        match self.tx_change_graph.try_send(message) {
            Ok(_) => {}
            Err(err) => {
                println!(
                    "Error sending ChangeGraphMessage::RemoveConnection: {:?}",
                    err
                );
            }
        }
    }

    /// Auto-connects a newly created node to the source of a dropped connection.
    ///
    /// Finds the first compatible input or output port on the new node and
    /// creates a connection to the original node the connection was dragged from.
    fn auto_connect_node(
        &mut self,
        new_node_id: &str,
        operation: &mangler_core::operations::Operation,
        conn: &TempConnection,
    ) {
        match conn.from_connection_type {
            // Dragged from an output: connect the output to the new node's first compatible input
            ConnectionType::Output => {
                let inputs = operation.create_inputs();
                if let Some(input_index) = inputs.iter().position(|input| {
                    input.accepts_any_type
                        || input
                            .value
                            .value_type()
                            .valid_conversions()
                            .contains(&conn.from_value_type)
                }) {
                    self.add_connection(
                        new_node_id.to_string(),
                        input_index,
                        conn.from_node_id.clone(),
                        conn.from_connection_index,
                    );
                }
            }
            // Dragged from an input: connect the new node's first compatible output to the input
            ConnectionType::Input => {
                let valid_from = conn.from_value_type.valid_conversions_from();
                let outputs = operation.create_outputs();
                if let Some(output_index) = outputs.iter().position(|output| {
                    valid_from.contains(&output.value.value_type())
                }) {
                    self.add_connection(
                        conn.from_node_id.clone(),
                        conn.from_connection_index,
                        new_node_id.to_string(),
                        output_index,
                    );
                }
            }
        }
    }
}

#[derive(Debug)]
pub struct NewConnection {
    pub input_node_id: String,
    pub input_connection_index: usize,
    pub output_node_id: String,
    pub output_connection_index: usize,
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
