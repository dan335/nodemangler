use eframe::egui;
use epaint::{Color32, ColorImage, CornerRadius, Pos2, Rect};
use mangler_core::{
    get_id,
    node_type::NodeType,
    value::{Value, ValueType},
    AddNodeType, ChangeGraphMessage, ChangeNodeMessage, GraphChangedMessage, NewGraphError,
    NodeChangedMessage,
};
use crate::graph::clipboard::Clipboard;
use mangler_core::float_image::FloatImage;
use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::mpsc;

use crate::{
    graph::{
        graph_editor::{GraphCamera, GraphEditor, GraphEditorResponse, TempConnection},
        graph_node::ConnectionType,
        graph_node::GraphNode,
        graph_node_thumbnail::GraphNodeThumbnail,
        node_search_popup::NodeSearchPopup,
    },
    graph_to_view_space,
    libraries::libraries_state::LibrariesState,
    node_menu::{menu_item::MenuItemsResult, menu_panel::MenuPanel},
    panels::{panel_kind::PanelKind, panel_tree::LeafId},
    settings::{graph_settings_panel, node_settings_panel},
    themes::theme::Theme,
    view_to_graph_space_pos2,
    view_window::{
        image_viewer::ImageViewer,
        material_channels::{material_input_channel, MaterialAssignment, MaterialChannel},
        preview_2d,
        preview_3d::{self, Preview3dPanel},
    },
    ManglerError, NODE_SIZE,
};

/// A library image loaded from disk and shown in this program's 2D preview
/// panel, independent of any graph node output. Clicking an image in the
/// Libraries panel sets this; viewing a node output clears it (last action
/// wins).
struct LibraryImagePreview {
    /// The source file, used to highlight the matching Libraries row and as the
    /// texture cache key.
    path: PathBuf,
    /// The decoded image.
    image: Arc<FloatImage>,
}

pub struct Program {
    pub app: mangler_core::app::App,
    tx_change_graph: mpsc::Sender<ChangeGraphMessage>,
    tx_change_node: mpsc::Sender<ChangeNodeMessage>,
    rx_node_changed: mpsc::Receiver<NodeChangedMessage>,
    rx_graph_changed: mpsc::Receiver<GraphChangedMessage>,
    graph_editor: GraphEditor,
    menu_panel: MenuPanel,
    editing_node_id: Option<String>,
    viewing_node_id_index: Option<(String, usize)>, // id and output index
    dragging_menu_button: MenuItemsResult,
    pointer_position: Pos2,
    graph_run_time: Duration,
    node_search_popup: NodeSearchPopup,
    /// Temporary status message shown on screen (text, expiry time).
    status_message: Option<(String, std::time::Instant)>,
    /// Persistent, user-dismissible warning about the loaded file (saved by
    /// a newer NodeMangler, and/or unknown nodes preserved as placeholders —
    /// see `GraphChangedMessage::LoadWarnings`). Shown as a banner at the
    /// top-center of the work area until the close button is clicked;
    /// deliberately NOT the 2-second fading `status_message`, because the
    /// user needs to actually read this one.
    load_warning: Option<String>,
    /// Save file that was modified externally while local edits were pending
    /// (`GraphChangedMessage::FileConflict`). While set, a blocking modal
    /// asks the user to reload or overwrite; the engine holds auto-saves in
    /// the meantime, so leaving the modal open is safe.
    file_conflict: Option<PathBuf>,
    /// Whether any panel tree (main window or a secondary window) currently
    /// has a Preview2D leaf open. Recomputed by `App` every frame from the
    /// union of trees — `Program` cannot see the panel tree itself — and used
    /// to hint the user when viewing a node with nowhere to show it.
    pub has_preview_2d_panel: bool,
    /// Per-leaf 2D preview pan/zoom state, keyed by panel leaf id.
    viewers_2d: HashMap<LeafId, ImageViewer>,
    /// Per-leaf 3D preview state (arcball camera + material channel bindings).
    viewers_3d: HashMap<LeafId, Preview3dPanel>,
    /// Per-leaf graph pan/zoom camera, keyed by panel leaf id — mirrors
    /// `viewers_2d`/`viewers_3d` so every Graph-kind panel pans and zooms
    /// independently instead of sharing one camera.
    graph_cameras: HashMap<LeafId, GraphCamera>,
    /// Which graph panel the node-search popup was opened over; its camera
    /// converts the popup position to graph space.
    popup_graph_leaf: Option<LeafId>,
    /// Last frame's main-window graph panel rects, kept for pointer→graph
    /// conversions that run before panels render (paste, dropped files).
    main_graph_rects: Vec<(LeafId, Rect)>,
    /// Screen-space registry of every graph panel across all OS windows:
    /// leaf → (rect in screen points, its window's content origin in screen
    /// points). Refreshed by each window's `show_menu_drag` pass and pruned
    /// with the viewers. Needed because a cross-window drag delivers all
    /// pointer events to the *source* window (OS mouse capture) in that
    /// window's local coordinates — screen space is the common frame.
    graph_rects_screen: HashMap<LeafId, (Rect, Pos2)>,
    /// Pointer position in screen points while a node-list drag is active,
    /// published by the window holding the mouse capture so every window can
    /// hit-test and draw the ghost node.
    menu_drag_pointer_screen: Option<Pos2>,
    /// Display name to show when this graph has no save path yet (a brand-new
    /// unsaved tab). Once a save path exists the name is derived purely from
    /// the file stem — see [`Self::display_name`].
    fallback_name: String,
    /// Editable buffer backing the graph-settings name field. Kept in sync
    /// with the authoritative display name while the field isn't focused, and
    /// its committed value is what drives a file rename (see
    /// `show_settings_panel`).
    graph_name_buffer: String,
    /// `.mangler.json` files dropped onto this program's window this frame.
    /// Opening a graph needs a new program tab, which only `App` can create,
    /// so the drop handler queues the paths here for `App` to drain after
    /// `update` (see `take_pending_open_graphs`).
    pending_open_graphs: Vec<PathBuf>,
    /// A library image being previewed in the 2D panel (see
    /// [`LibraryImagePreview`]). When set, it takes precedence over
    /// `viewing_node_id_index` in the 2D preview.
    library_image_preview: Option<LibraryImagePreview>,
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
                status_message: None,
                load_warning: None,
                file_conflict: None,
                has_preview_2d_panel: false,
                viewers_2d: HashMap::new(),
                viewers_3d: HashMap::new(),
                graph_cameras: HashMap::new(),
                popup_graph_leaf: None,
                main_graph_rects: Vec::new(),
                graph_rects_screen: HashMap::new(),
                menu_drag_pointer_screen: None,
                fallback_name: "new graph".to_string(),
                graph_name_buffer: String::new(),
                pending_open_graphs: Vec::new(),
                library_image_preview: None,
            }),
            Err(error) => Err(NewGraphError(format!(
                "Error creating program. {:?}",
                error
            ))),
        }
    }

    /// Whether this program's graph currently has no nodes. `graph_editor`'s
    /// node map is the GUI-side mirror of the engine graph (kept in sync by the
    /// `LoadedNode`/`NodeRemoved` handlers and `GraphCleared`), so this is an
    /// accurate "is the graph blank right now" check. Used at shutdown and on
    /// tab close to spot throwaway auto-created "untitled" graphs worth deleting
    /// so blank files don't accumulate in the default library.
    pub fn is_empty(&self) -> bool {
        self.graph_editor.graph_nodes.is_empty()
    }

    /// This graph's display name: derived purely from the save-path file
    /// stem (so the tab title and the Libraries panel always agree), falling
    /// back to [`Self::fallback_name`] for a brand-new graph with no file yet.
    pub fn display_name(&self) -> String {
        match &self.app.save_path {
            Some(path) => mangler_core::naming::graph_display_name_from_path(path),
            None => self.fallback_name.clone(),
        }
    }

    /// Points this program's graph at a save path. Used by the Libraries panel
    /// when creating a graph inside a library folder and when re-targeting a
    /// tab after its file was renamed on disk. The display name follows the
    /// path automatically (see [`Self::display_name`]), so there's no separate
    /// name to set: update the GUI-side save path, then tell the engine, which
    /// auto-saves to the new path ~1s later.
    pub fn set_save_location(&mut self, path: PathBuf) {
        self.app.save_path = Some(path.clone());

        if let Err(err) = self
            .tx_change_graph
            .try_send(ChangeGraphMessage::SetSavePath(path))
        {
            println!("Error sending graph_message: {:?}", err);
        }
    }

    /// Creates an "image from file" input node wired to `path` and drops it
    /// near the focused graph panel's centre (with a little random jitter so
    /// repeated adds don't stack exactly). Shared by the drag-and-drop handler
    /// and the Libraries panel's "add to current graph" action.
    pub fn add_image_from_file(&mut self, path: PathBuf) {
        // Pick a screen point inside the focused graph panel, then map it into
        // graph space through that panel's camera. `main_graph_rects` holds
        // last frame's panel rects — the same source `camera_at` relies on for
        // pre-render pointer conversions.
        let rect = self
            .main_graph_rects
            .first()
            .map(|(_, r)| *r)
            .unwrap_or_else(|| Rect::from_min_size(Pos2::ZERO, egui::vec2(800.0, 600.0)));
        let jitter = rect.width().min(rect.height()) * 0.3;
        let screen = Pos2::new(
            rect.center().x + fastrand::f32() * jitter - jitter * 0.5,
            rect.center().y + fastrand::f32() * jitter - jitter * 0.5,
        );
        let (zoom, position) = self.camera_at(screen);
        let graph_pos = view_to_graph_space_pos2(zoom, screen) - position.to_vec2();

        match self.add_node(
            AddNodeType::Operation(mangler_core::operations::Operation::OpImageInputFile),
            graph_pos,
            true,
            None,
            Vec::new(),
        ) {
            Ok(node_id) => {
                let message = ChangeNodeMessage::SetInput {
                    node_id,
                    input_index: 0,
                    value: Value::Path(path),
                };
                if let Err(err) = self.tx_change_node.try_send(message) {
                    println!("Error sending graph_message: {:?}", err);
                }
            }
            Err(err) => println!("Error adding image node: {}", err.0),
        }
    }

    /// Takes (and clears) the `.mangler.json` files dropped onto this program's
    /// window this frame. `App` drains these after `update` and opens each in
    /// a tab (via `open_or_focus`), which the program itself can't do.
    pub fn take_pending_open_graphs(&mut self) -> Vec<PathBuf> {
        std::mem::take(&mut self.pending_open_graphs)
    }

    /// Shows `message` in the fading status overlay (see
    /// `show_status_message`). Used by `App` for one-off notices that don't
    /// warrant the blocking error modal — e.g. "no default library" when a
    /// brand-new graph can't be given an auto-save location.
    pub fn queue_status_message(&mut self, message: String) {
        self.status_message = Some((message, std::time::Instant::now()));
    }

    /// Once-per-frame logic that must run before any panel rendering: pointer
    /// tracking, copy/paste, the engine message pumps, dropped-file handling,
    /// and the repaint policy. Must be called before `show_panel` /
    /// `show_overlays` each frame (mirrors the head of the old `show`).
    pub fn update(&mut self, ctx: &egui::Context, ui: &mut egui::Ui) {
        // Update pointer position early so paste places nodes at the current cursor.
        if let Some(pos) = ctx.pointer_latest_pos() {
            self.pointer_position = pos;
        }

        // Copy/paste keyboard shortcuts.
        {
            let (ctrl_c, paste_text) = ctx.input(|i| detect_copy_paste(&i.events));

            // Ctrl+C: copy selected nodes to system clipboard
            if ctrl_c {
                let mut selection = self.graph_editor.selected_node_ids.clone();
                if selection.is_empty() {
                    if let Some(editing_id) = &self.editing_node_id {
                        selection.insert(editing_id.clone());
                    }
                }

                if selection.is_empty() {
                    self.status_message = Some(("Nothing to copy — select a node first".to_string(), std::time::Instant::now()));
                } else if let Some(cb) = Clipboard::from_selection(
                    &selection,
                    &self.graph_editor.graph_nodes,
                ) {
                    let count = cb.nodes.len();
                    ctx.copy_text(cb.to_clipboard_string());
                    self.status_message = Some((
                        format!("Copied {} node{}", count, if count == 1 { "" } else { "s" }),
                        std::time::Instant::now(),
                    ));
                }
            }

            // Ctrl+V: paste nodes from system clipboard
            if let Some(text) = paste_text {
                if let Some(cb) = Clipboard::from_clipboard_string(&text) {
                    let count = cb.nodes.len();
                    self.paste_clipboard(&cb);
                    self.status_message = Some((
                        format!("Pasted {} node{}", count, if count == 1 { "" } else { "s" }),
                        std::time::Instant::now(),
                    ));
                }
                // Non-node clipboard content is silently ignored.
            }
        }

        let mut received_messages = false;
        while let Ok(graph_changed_message) = self.rx_graph_changed.try_recv() {
            received_messages = true;
            match graph_changed_message {
                GraphChangedMessage::AddedNode {
                    node_id,
                    settings,
                    inputs,
                    outputs,
                    position,
                    is_subgraph,
                    node_type,
                    is_enabled,
                    custom_name,
                } => {
                    self.graph_editor.add_node(
                        node_id,
                        settings,
                        inputs,
                        outputs,
                        Pos2::new(position.x, position.y),
                        is_subgraph,
                        Some(node_type),
                        is_enabled,
                        custom_name,
                    );
                }
                GraphChangedMessage::LoadedNode { node } => {
                    let (is_subgraph, add_node_type, subgraph_path) = match &node.node_type {
                        NodeType::Operation { operation } => {
                            (false, Some(AddNodeType::Operation(operation.clone())), None)
                        }
                        NodeType::Subgraph { path, .. } => {
                            let path_opt = if path.as_os_str().is_empty() {
                                None
                            } else {
                                Some(path.clone())
                            };
                            (true, Some(AddNodeType::Subgraph), path_opt)
                        }
                        NodeType::Unknown { .. } => {
                            // Placeholder node from a newer-version save (see
                            // `mangler_core::saved_nodes`). There's no
                            // `AddNodeType` for it, so it renders with
                            // `node_type: None` for now (clipboard copy/paste
                            // already skips nodes with no op — see
                            // `Clipboard::from_selection`). Full display
                            // support (error badge, non-runnable styling)
                            // lands in a later pass.
                            (false, None, None)
                        }
                    };

                    let mut graph_node = GraphNode::new(
                        node.id.clone(),
                        Pos2::new(node.position.x, node.position.y),
                        node.settings,
                        node.inputs,
                        node.outputs,
                        is_subgraph,
                        add_node_type,
                        node.is_enabled,
                        node.custom_name,
                    );
                    graph_node.subgraph_path = subgraph_path;

                    self.graph_editor.graph_nodes.insert(node.id, graph_node);
                }
                GraphChangedMessage::RemovedNode { node_id } => {
                    if self.editing_node_id.as_ref() == Some(&node_id) {
                        self.editing_node_id = None;
                    }
                    if self.viewing_node_id_index.as_ref().map(|(id, _)| id) == Some(&node_id) {
                        self.viewing_node_id_index = None;
                    }
                    self.graph_editor.selected_node_ids.remove(&node_id);
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
                            node.clear_output_connection(output_index, &node_id, input_index);
                        }
                    }

                    //self.needs_to_save = true;
                }
                GraphChangedMessage::LoadWarnings {
                    file_version,
                    is_newer_than_app,
                    unknown_nodes,
                } => {
                    // Compose the dismissible banner text. Both conditions
                    // can be true at once (newer file AND unknown nodes), so
                    // build the message from parts.
                    let mut parts: Vec<String> = Vec::new();
                    if is_newer_than_app {
                        parts.push(format!(
                            "Saved with NodeMangler {} — you're on {}. Auto-save paused until you edit.",
                            file_version,
                            mangler_core::APP_VERSION,
                        ));
                    }
                    if !unknown_nodes.is_empty() {
                        parts.push(format!(
                            "{} unknown node(s) preserved as placeholders.",
                            unknown_nodes.len(),
                        ));
                    }
                    if !parts.is_empty() {
                        self.load_warning = Some(parts.join(" "));
                    }
                }
                GraphChangedMessage::FileConflict { path } => {
                    // Save file rewritten externally while local edits are
                    // pending — remember the path; show_overlays renders the
                    // Reload-vs-Overwrite modal while this is set.
                    self.file_conflict = Some(path);
                }
                GraphChangedMessage::SaveError { path, message } => {
                    // Writing the save file failed (missing/unwritable
                    // directory, disk full, ...). Not fatal — the edit is
                    // still in memory and the next auto-save tick will try
                    // again — so this is a fading status message rather
                    // than a blocking modal.
                    self.status_message = Some((
                        format!("couldn't save {}: {}", path.display(), message),
                        std::time::Instant::now(),
                    ));
                }
                GraphChangedMessage::GraphCleared => {
                    // The engine is replacing the graph wholesale (conflict
                    // resolved with "reload from disk"): drop every node,
                    // selection, and in-progress connection, and stop
                    // viewing/editing nodes that are about to vanish. The
                    // fresh LoadedNode stream that follows repopulates the
                    // editor.
                    self.graph_editor.clear();
                    self.editing_node_id = None;
                    self.viewing_node_id_index = None;
                }
                GraphChangedMessage::FileRenamed { new_path } => {
                    // The engine renamed our file on disk (in response to a
                    // RenameFile). Adopt the new path; the tab title and the
                    // name field follow automatically via `display_name`.
                    self.app.save_path = Some(new_path);
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
            received_messages = true;
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
                                // Image outputs with `thumbnail: None`
                                // are the "deferred to the async service"
                                // cases. Leave the existing thumbnail in
                                // place so the node preview doesn't flash
                                // blank between OutputChanged and
                                // ThumbnailReady.
                                let is_deferred = matches!(
                                    (&value, &thumbnail),
                                    (Value::Image { .. }, None)
                                );
                                if !is_deferred {
                                    node.thumbnail = build_graph_node_thumbnail(
                                        ui.ctx(),
                                        &node.id,
                                        thumbnail,
                                        &value,
                                    );
                                }
                            }
                        }
                    }
                }

                NodeChangedMessage::ThumbnailReady {
                    node_id,
                    output_index,
                    change_id,
                    thumbnail,
                } => {
                    // Only the slot-0 thumbnail drives the visible node
                    // preview today; still, honour the output_index so this
                    // stays correct if slot-N previews are added later.
                    if output_index != 0 {
                        continue;
                    }
                    if let Some(node) = self.graph_editor.graph_nodes.get_mut(&node_id) {
                        if let Some(output) = node.outputs.get(output_index) {
                            // Stale-reject: if the output's current value no
                            // longer matches the id this thumbnail was built
                            // for, the engine has already produced a newer
                            // value and dropping here avoids flashing an
                            // outdated preview.
                            let is_current = match &output.value {
                                Value::Image { change_id: cid, .. } => *cid == change_id,
                                _ => false,
                            };
                            if !is_current {
                                continue;
                            }
                            node.thumbnail = build_graph_node_thumbnail(
                                ui.ctx(),
                                &node.id,
                                Some(thumbnail),
                                &output.value,
                            );
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

        // Dropped files: collect the paths under the ctx.input borrow, then
        // act on them afterwards (adding a node / queueing a graph-open both
        // touch `self`, which we can't mutate while the input closure holds
        // its borrow).
        let mut dropped_image_paths: Vec<PathBuf> = Vec::new();
        let mut dropped_graph_paths: Vec<PathBuf> = Vec::new();
        ctx.input(|i| {
            for file in i.raw.dropped_files.iter() {
                let Some(path) = &file.path else { continue };
                let file_name = path
                    .file_name()
                    .map(|n| n.to_string_lossy().into_owned())
                    .unwrap_or_default();
                if crate::libraries::library_scanner::is_graph_file(&file_name) {
                    // A NodeMangler graph: let `App` open it in a tab.
                    dropped_graph_paths.push(path.clone());
                } else if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
                    if ValueType::file_extensions(&ValueType::Image)
                        .contains(&ext.to_lowercase())
                    {
                        dropped_image_paths.push(path.clone());
                    }
                }
            }
        });
        for path in dropped_image_paths {
            self.add_image_from_file(path);
        }
        // Bubble dropped graphs up for `App` to open (needs the programs map).
        self.pending_open_graphs.extend(dropped_graph_paths);

        // Request repaint only when needed:
        // - Immediately if we received engine messages this frame
        // - Immediately if a status message animation is active
        // - Otherwise poll at 10fps for new engine messages
        if received_messages {
            ctx.request_repaint();
        } else if self.status_message.is_some() {
            ctx.request_repaint();
        } else {
            ctx.request_repaint_after(Duration::from_millis(100));
        }
    }

    /// Render one panel's content given its leaf id and kind. Called once per
    /// visible leaf per frame by the panel-tree renderer.
    pub fn show_panel(
        &mut self,
        ui: &mut egui::Ui,
        leaf_id: LeafId,
        kind: PanelKind,
        theme: &Theme,
        libraries: &mut LibrariesState,
    ) {
        match kind {
            PanelKind::NodeList => self.show_node_list_panel(ui, theme),
            PanelKind::Settings => self.show_settings_panel(ui, theme),
            PanelKind::Graph => self.show_graph_panel(ui, leaf_id, theme),
            PanelKind::Preview2D => self.show_preview_2d_panel(ui, leaf_id, theme),
            PanelKind::Preview3D => self.show_preview_3d_panel(ui, leaf_id, theme),
            // Libraries state is app-global (one browser shared by every
            // program tab), so the panel renders it rather than `self`. This
            // program is the focused one, so its save path tells the panel
            // which graph row to highlight as "currently open".
            PanelKind::Libraries => crate::libraries::libraries_panel::show(
                ui,
                libraries,
                theme,
                self.app.save_path.as_deref(),
                self.previewed_library_image(),
            ),
        }
    }

    fn show_node_list_panel(&mut self, ui: &mut egui::Ui, theme: &Theme) {
        puffin::profile_scope!("menu panel");
        let r = self.menu_panel.show(ui, theme);

        if r.subgraph_being_created {
            self.dragging_menu_button.subgraph_being_created = true;
        }

        if r.operation_being_created.is_some() {
            self.dragging_menu_button.operation_being_created = r.operation_being_created;
        }
    }

    fn show_settings_panel(&mut self, ui: &mut egui::Ui, theme: &Theme) {
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
            // Scroll the settings content so long help text and tall input
            // lists stay reachable when they exceed the panel height.
            egui::ScrollArea::vertical()
                .auto_shrink([false, false])
                .show(ui, |ui| {
            let mut show_graph_settings = true;

            // show node settings
            if let Some(editing_node_id) = &self.editing_node_id {
                if let Some(node) = self.graph_editor.graph_nodes.get_mut(editing_node_id) {
                    // Seed file-dialog directories with this graph's own
                    // folder, so a "save/open file" input starts next to the
                    // graph rather than wherever rfd last landed. An input with
                    // an explicit `set_directory` overrides this in the panel.
                    let graph_dir = self.app.save_path.as_deref().and_then(|p| p.parent());
                    let node_settings_response =
                        node_settings_panel::show(
                            ui,
                            node,
                            &self.tx_change_node,
                            theme,
                            graph_dir,
                        );
                    show_graph_settings = false;

                    if node_settings_response.deselect_node {
                        self.graph_editor.selected_node_ids.remove(editing_node_id);
                        self.editing_node_id = None;
                    }
                }
            }

            if show_graph_settings {
                let display_name = self.display_name();
                let graph_settings_response = graph_settings_panel::show(
                    ui,
                    &mut self.graph_name_buffer,
                    &display_name,
                    &self.app.save_path,
                    theme,
                );

                // name committed -> rename the file on disk. We do NOT
                // optimistically update save_path here: the rename can fail
                // (name collision), and the engine's SaveError → status
                // message explains it. On success FileRenamed updates the
                // path, and display_name (hence the tab title) follows.
                if let Some(new_stem) = graph_settings_response.new_name {
                    let message = ChangeGraphMessage::RenameFile { new_stem };

                    match self.tx_change_graph.try_send(message) {
                        Ok(_) => {}
                        Err(err) => {
                            println!("Error sending graph_message: {:?}", err);
                        }
                    }
                }

                // auto arrange requested
                if graph_settings_response.auto_arrange {
                    let moved_nodes = self.graph_editor.auto_arrange();
                    for (node_id, new_pos) in moved_nodes {
                        let message = ChangeNodeMessage::SetPosition {
                            node_id,
                            position: glam::f32::vec2(new_pos.x, new_pos.y),
                        };
                        let _ = self.tx_change_node.try_send(message);
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
    }

    fn show_graph_panel(&mut self, ui: &mut egui::Ui, leaf_id: LeafId, theme: &Theme) {
        puffin::profile_scope!("graph panel");

        // Destructure so the per-leaf camera and the graph editor can be
        // borrowed simultaneously (disjoint fields), same pattern as
        // `show_preview_2d_panel`.
        let Self {
            graph_cameras,
            graph_editor,
            editing_node_id,
            viewing_node_id_index,
            node_search_popup,
            ..
        } = self;
        let camera = graph_cameras.entry(leaf_id).or_insert_with(GraphCamera::new);

        let graph_editor_response: GraphEditorResponse = graph_editor.show(
            ui,
            camera,
            &*editing_node_id,
            &*viewing_node_id_index,
            theme,
            node_search_popup.is_open,
        );

        for (node_id, pos) in graph_editor_response.new_node_positions {
            let node_position_message = ChangeNodeMessage::SetPosition {
                node_id,
                position: glam::f32::vec2(pos.x, pos.y),
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

        // Right-clicked a material export node: bind the 3D preview panels'
        // channels from its input connections instead of viewing an output.
        if let Some(node_id) = graph_editor_response.view_material_node {
            self.bind_material_node_to_3d(&node_id);
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
            self.node_search_popup
                .open(self.pointer_position, Some(dropped));
            self.popup_graph_leaf = Some(leaf_id);
        }

        // Graph run timing and interaction help live inside the graph panel —
        // they describe the graph, not the whole app — anchored to this
        // panel's corners (the clip rect keeps them from spilling out).
        let panel_rect = ui.max_rect();
        {
            let graph_ms = self.graph_run_time.as_secs_f64() * 1000.0;
            let status_txt = format!("graph: {:.1}ms", graph_ms);
            let pos = Pos2::new(panel_rect.right() - 10.0, panel_rect.bottom() - 10.0);
            ui.painter().text(
                pos,
                egui::Align2::RIGHT_BOTTOM,
                status_txt,
                egui::FontId::monospace(10.0),
                egui::Color32::from(theme.get().text_faint),
            );
        }
        {
            let pos = Pos2::new(panel_rect.left() + 10.0, panel_rect.bottom() - 10.0);
            let txt =
                "left click: edit      right click: view      ctrl + left click: delete      delete/backspace: delete selected      shift + click: multi-select      ctrl+c: copy      ctrl+v: paste".to_string();
            ui.painter().text(
                pos,
                egui::Align2::LEFT_BOTTOM,
                txt,
                egui::FontId::proportional(12.0),
                egui::Color32::from(theme.get().text_faint),
            );
        }
    }

    fn show_preview_2d_panel(&mut self, ui: &mut egui::Ui, leaf_id: LeafId, theme: &Theme) {
        // Destructure so the per-leaf viewer and the graph nodes can be
        // borrowed simultaneously (disjoint fields).
        let Self {
            viewers_2d,
            graph_editor,
            viewing_node_id_index,
            library_image_preview,
            ..
        } = self;

        let viewer = viewers_2d.entry(leaf_id).or_insert_with(ImageViewer::new);

        // A clicked library image takes precedence over a viewed node output.
        // Its cache key is the file path, so switching images rebuilds the
        // texture; a synthetic node id keeps it distinct from real outputs.
        if let Some(preview) = library_image_preview.as_ref() {
            viewer.show(
                ui,
                "__library_image_preview__".to_string(),
                0,
                preview.path.to_string_lossy().into_owned(),
                &preview.image,
                true, // fit each newly-opened library image to the view
                theme,
            );
            return;
        }

        if let Some((viewing_node_id, output_index)) = viewing_node_id_index.as_ref() {
            if let Some(graph_node) = graph_editor.graph_nodes.get(viewing_node_id) {
                preview_2d::show(ui, viewer, graph_node, *output_index, theme);
                return;
            }
        }

        preview_2d::show_empty(ui, theme);
    }

    fn show_preview_3d_panel(&mut self, ui: &mut egui::Ui, leaf_id: LeafId, theme: &Theme) {
        let Self {
            viewers_3d,
            graph_editor,
            ..
        } = self;

        let panel = viewers_3d.entry(leaf_id).or_insert_with(Preview3dPanel::new);
        preview_3d::show(panel, ui, &graph_editor.graph_nodes, theme);
    }

    /// Discard per-leaf viewer state for leaves that no longer exist. 3D
    /// viewers hold GL resources, so pruning frees them promptly.
    pub fn prune_viewers(&mut self, live: &HashSet<LeafId>) {
        self.viewers_2d.retain(|id, _| live.contains(id));
        self.viewers_3d.retain(|id, _| live.contains(id));
        self.graph_cameras.retain(|id, _| live.contains(id));
        self.graph_rects_screen.retain(|id, _| live.contains(id));
    }

    /// zoom + position of the camera for `leaf`, falling back to an identity
    /// transform (zoom 1, no pan) when the panel has no camera yet.
    fn camera_transform(&self, leaf: Option<LeafId>) -> (f32, Pos2) {
        leaf.and_then(|id| self.graph_cameras.get(&id))
            .map(|camera| (camera.zoom, camera.position))
            .unwrap_or((1.0, Pos2::ZERO))
    }

    /// Camera (zoom, position) for the main-window graph panel under `pos`,
    /// falling back to the first main-window graph panel (if any), then to
    /// an identity transform. Used for pointer→graph conversions that run
    /// before panels render this frame (paste, dropped files), when we only
    /// have last frame's `main_graph_rects` to go on.
    fn camera_at(&self, pos: Pos2) -> (f32, Pos2) {
        let leaf = self
            .main_graph_rects
            .iter()
            .find(|(_, r)| r.contains(pos))
            .or_else(|| self.main_graph_rects.first())
            .map(|(id, _)| *id);
        self.camera_transform(leaf)
    }

    /// Main-window overlays drawn on top of every panel: Tab-to-search,
    /// delete-key handling, the node-search popup, the main window's
    /// menu-drag handling (see [`Self::show_menu_drag`]), and the status
    /// message. Graph timing and help text render inside each graph panel.
    ///
    /// `graph_rects` are the on-screen rects of the main window's graph panels
    /// (used for hover/hit-tests); `work_rect` is the area below the menu bar
    /// used to anchor the status message.
    pub fn show_overlays(
        &mut self,
        ctx: &egui::Context,
        ui: &mut egui::Ui,
        theme: &Theme,
        graph_rects: &[(LeafId, Rect)],
        work_rect: Rect,
    ) {
        // Keep the main-window graph rects around for pointer→graph
        // conversions that run before panels render this frame (paste,
        // dropped files) — see `camera_at`.
        self.main_graph_rects = graph_rects.to_vec();

        // Open search popup on Tab key (only when popup isn't already open)
        if !self.node_search_popup.is_open {
            let hovered_leaf = graph_rects
                .iter()
                .find(|(_, r)| r.contains(self.pointer_position))
                .map(|(id, _)| *id);
            if let Some(leaf) = hovered_leaf {
                let tab_pressed = ctx.input(|i| i.key_pressed(egui::Key::Tab));
                if tab_pressed {
                    self.node_search_popup.open(self.pointer_position, None);
                    self.popup_graph_leaf = Some(leaf);
                }
            }
        }

        // Delete all selected nodes on Delete/Backspace key.
        // Backspace is included because on macOS the key labelled "delete" is
        // Backspace (true forward-delete is Fn+Delete). Skip when a text field
        // has keyboard focus so backspace still edits text there.
        let typing = ctx.egui_wants_keyboard_input();
        let delete_pressed = !typing
            && ctx.input(|i| {
                i.key_pressed(egui::Key::Delete) || i.key_pressed(egui::Key::Backspace)
            });
        if delete_pressed {
            let node_ids = collect_selected_nodes_to_delete(
                &mut self.graph_editor.selected_node_ids,
                &mut self.editing_node_id,
            );
            for node_id in node_ids {
                self.remove_node(node_id);
            }
        }

        // Show the search popup and handle selection
        if self.node_search_popup.is_open {
            let popup_response = self.node_search_popup.show(ctx, theme);

            if let Some(operation) = popup_response.selected_operation {
                let (zoom, position) = self.camera_transform(self.popup_graph_leaf);
                let graph_pos = view_to_graph_space_pos2(
                    zoom,
                    self.node_search_popup.position,
                ) - position.to_vec2();

                // Store connection info before closing popup
                let from_connection = self.node_search_popup.from_connection.clone();

                if let Ok(new_node_id) =
                    self.add_node(AddNodeType::Operation(operation.clone()), graph_pos, true, None, Vec::new())
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

        // Menu-drag release + ghost node for the main window. Secondary
        // windows make the same call with their own graph rects. (There is
        // deliberately no "cursor left the window → cancel" check: during a
        // cross-window drag the cursor legitimately leaves the source window;
        // the drag always ends on button release instead.)
        self.show_menu_drag(ui, graph_rects, theme);

        self.show_status_message(ui, work_rect);

        self.show_load_warning_banner(ui, work_rect, theme);
        self.show_file_conflict_modal(ui, theme);
    }

    /// Persistent, dismissible load-warning banner (newer-version file /
    /// unknown-node placeholders), top-center of the work area. Unlike the
    /// fading `status_message`, this stays until the user closes it — it
    /// carries information they need to act on (auto-save is being held).
    fn show_load_warning_banner(&mut self, ui: &mut egui::Ui, work_rect: Rect, theme: &Theme) {
        let Some(warning) = self.load_warning.clone() else {
            return;
        };
        let colors = theme.get();

        // An Area gets its own layer above the panels and supports widgets,
        // which plain painter text can't do (the close button needs to be
        // clickable).
        let mut dismissed = false;
        egui::Area::new(egui::Id::new("load_warning_banner"))
            .order(egui::Order::Foreground)
            .pivot(egui::Align2::CENTER_TOP)
            .fixed_pos(Pos2::new(work_rect.center().x, work_rect.top() + 8.0))
            .show(ui.ctx(), |ui| {
                // All chrome colors come from the theme (see CLAUDE.md);
                // window_* values are what popups/modals already use, so the
                // banner matches them in every theme.
                egui::Frame::NONE
                    .fill(colors.window_fill)
                    .stroke(colors.window_stroke)
                    .corner_radius(colors.window_corner_radius)
                    .inner_margin(egui::Margin::symmetric(12, 8))
                    .show(ui, |ui| {
                        ui.horizontal(|ui| {
                            ui.label(
                                egui::RichText::new(&warning)
                                    .color(colors.override_text_color),
                            );
                            ui.add_space(4.0);
                            // Small close button; frameless so it reads as a
                            // dismiss glyph rather than a chunky button.
                            if ui
                                .add(egui::Button::new("✕").frame(false))
                                .on_hover_text("dismiss")
                                .clicked()
                            {
                                dismissed = true;
                            }
                        });
                    });
            });

        if dismissed {
            self.load_warning = None;
        }
    }

    /// Blocking Reload-vs-Overwrite prompt shown while `file_conflict` is
    /// set (the save file was rewritten externally with local edits
    /// pending). Same `egui::Modal` pattern as the Libraries panel dialogs.
    /// Esc / clicking outside deliberately does NOT close it: there is no
    /// safe "neither" answer, and the engine holds auto-saves until a
    /// `ResolveFileConflict` arrives, so staying open loses nothing.
    fn show_file_conflict_modal(&mut self, ui: &mut egui::Ui, theme: &Theme) {
        let Some(path) = self.file_conflict.clone() else {
            return;
        };
        let colors = theme.get();

        // None = still deciding; Some(keep_ours) = a button was clicked.
        let mut choice: Option<bool> = None;

        egui::Modal::new(egui::Id::new("file_conflict_modal")).show(ui.ctx(), |ui| {
            ui.set_width(320.0);

            // Match the Libraries dialogs: make sure any control surfaces
            // stay legible against the modal background in every theme.
            ui.style_mut().visuals.extreme_bg_color = colors.widgets_interactive_bg_fill;

            ui.heading("file changed on disk");
            ui.add_space(8.0);

            let name = path
                .file_name()
                .map(|n| n.to_string_lossy().into_owned())
                .unwrap_or_else(|| path.display().to_string());
            ui.label(format!(
                "'{}' was modified outside this tab while you have unsaved edits.",
                name
            ));
            ui.label("Auto-save is paused until you choose.");

            ui.add_space(12.0);
            ui.horizontal(|ui| {
                if ui.button("reload from disk (discard my changes)").clicked() {
                    choice = Some(false);
                }
                if ui.button("overwrite (keep mine)").clicked() {
                    choice = Some(true);
                }
            });
        });
        // Note: `modal.should_close()` (Esc / outside click) is intentionally
        // ignored — see the doc comment above.

        if let Some(keep_ours) = choice {
            if let Err(err) = self
                .tx_change_graph
                .try_send(ChangeGraphMessage::ResolveFileConflict { keep_ours })
            {
                println!("Error sending ResolveFileConflict: {:?}", err);
            }
            self.file_conflict = None;
        }
    }

    /// Menu-drag handling for one window: while a node-list drag is active,
    /// paint the ghost node under the drag pointer, and on primary release
    /// over any graph panel (in any window) create the dragged node there.
    ///
    /// Cross-window detail: the OS gives the *source* window mouse capture
    /// for the whole drag, so only that window receives pointer/release
    /// events — in its own local coordinates, even when the cursor is
    /// physically over another window. The capturing window therefore
    /// publishes the pointer in *screen* points (`menu_drag_pointer_screen`),
    /// every window registers its graph rects in screen points
    /// (`graph_rects_screen`), and hit-tests/ghost drawing happen in that
    /// shared frame.
    pub fn show_menu_drag(&mut self, ui: &mut egui::Ui, graph_rects: &[(LeafId, Rect)], theme: &Theme) {
        // This window's content origin in screen points; unavailable e.g.
        // while minimized, in which case it can't participate this frame.
        let Some(origin) = ui
            .ctx()
            .input(|i| i.viewport().inner_rect)
            .map(|r| r.min)
        else {
            return;
        };

        // Keep the screen-space registry fresh even while no drag is active,
        // so it is correct the moment one starts.
        for (leaf, rect) in graph_rects {
            self.graph_rects_screen
                .insert(*leaf, (rect.translate(origin.to_vec2()), origin));
        }

        if !self.dragging_menu_button.subgraph_being_created
            && self.dragging_menu_button.operation_being_created.is_none()
        {
            return;
        }

        let (primary_down, primary_released, local_pointer) = ui.ctx().input(|i| {
            (
                i.pointer.primary_down(),
                i.pointer.primary_released(),
                i.pointer.latest_pos(),
            )
        });

        // Only the capturing window holds the button during the drag, so this
        // updates from exactly one window per frame — with live coordinates
        // even when the cursor is outside its bounds.
        if primary_down || primary_released {
            if let Some(local) = local_pointer {
                self.menu_drag_pointer_screen = Some(origin + local.to_vec2());
            }
        }

        let Some(pointer_screen) = self.menu_drag_pointer_screen else {
            return;
        };

        // release mouse button after dragging menu button — delivered to the
        // capturing window only; the drop target may be any window's panel.
        if primary_released {
            let target = self
                .graph_rects_screen
                .iter()
                .find(|(_, (screen_rect, _))| screen_rect.contains(pointer_screen))
                .map(|(leaf, (_, target_origin))| (*leaf, *target_origin));
            if let Some((leaf, target_origin)) = target {
                let node_type =
                    if let Some(operation) = &self.dragging_menu_button.operation_being_created {
                        AddNodeType::Operation(operation.clone())
                    } else {
                        AddNodeType::Subgraph
                    };
                // Graph-space position from the target window's local coords
                // and the target panel's camera.
                let local = pointer_screen - target_origin.to_vec2();
                let (zoom, position) = self.camera_transform(Some(leaf));
                //let node_position_view_space = Pos2::new(cursor_position.x - bottom_panel_rect.min.x, cursor_position.y - bottom_panel_rect.min.y);
                if let Ok(node_id) = self.add_node(
                    node_type,
                    view_to_graph_space_pos2(zoom, local) - position.to_vec2(),
                    true,
                    None,
                    Vec::new(),
                ) {
                    self.edit_node(node_id);
                }
            }

            self.dragging_menu_button = MenuItemsResult::default();
            self.menu_drag_pointer_screen = None;
            return;
        }

        // Ghost node: drawn by whichever window the drag pointer is currently
        // over (converted from screen points to this window's local coords).
        let pointer = pointer_screen - origin.to_vec2();
        if !ui.ctx().content_rect().contains(pointer) {
            return;
        }

        // dragging node from menu
        // draw shape behind mouse being dragged
        let mut name = "".to_string();

        if let Some(op) = &self.dragging_menu_button.operation_being_created {
            name = op.settings().name.clone();
        } else if self.dragging_menu_button.subgraph_being_created {
            name = "subgraph".to_string();
        }

        let drag_rect = Rect::from_center_size(pointer, NODE_SIZE);

        ui.painter().add(egui::Shape::rect_filled(
            drag_rect,
            CornerRadius::ZERO,
            theme.get().node_header_bg,
        ));

        // Ghost node font size follows the zoom of whichever graph panel the
        // pointer is currently over, falling back to zoom 1.0 when it isn't
        // over any graph panel.
        let hovered_zoom = graph_rects
            .iter()
            .find(|(_, r)| r.contains(pointer))
            .map(|(id, _)| self.camera_transform(Some(*id)).0)
            .unwrap_or(1.0);

        // node name
        ui.painter().text(
            drag_rect.center(),
            egui::Align2::CENTER_CENTER,
            name,
            //egui::style::Style::text_styles(),
            egui::FontId::proportional(graph_to_view_space(hovered_zoom, 14.0)),
            Color32::from(theme.get().override_text_color),
        );
    }

    /// Fading status message (copy/paste feedback etc.), centered near the
    /// bottom of the main window's work area.
    fn show_status_message(&mut self, ui: &mut egui::Ui, work_rect: Rect) {
        // show status message (copy/paste feedback)
        if let Some((msg, created)) = &self.status_message {
            let elapsed = created.elapsed();
            if elapsed < std::time::Duration::from_secs(2) {
                // Fade out over the last 0.5s
                let alpha = if elapsed.as_secs_f32() > 1.5 {
                    ((2.0 - elapsed.as_secs_f32()) / 0.5 * 255.0) as u8
                } else {
                    255
                };
                let pos = Pos2::new(work_rect.center().x, work_rect.bottom() - 40.0);
                ui.painter().text(
                    pos,
                    egui::Align2::CENTER_BOTTOM,
                    msg,
                    egui::FontId::proportional(14.0),
                    egui::Color32::from_rgba_unmultiplied(255, 255, 255, alpha),
                );
            } else {
                self.status_message = None;
            }
        }
    }

    pub fn add_node(
        &mut self,
        node_type: AddNodeType,
        position_graph_space: Pos2,
        is_enabled: bool,
        custom_name: Option<String>,
        input_values: Vec<(usize, Value)>,
    ) -> Result<String, ManglerError> {
        let node_id = get_id();

        let add_node_message = ChangeGraphMessage::AddNode {
            node_id: node_id.clone(),
            node_type,
            position: glam::f32::Vec2::new(position_graph_space.x, position_graph_space.y),
            is_enabled,
            custom_name,
            input_values,
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
        // A node output replaces any library image being previewed (last
        // action wins), so the 2D panel shows what the user just picked.
        self.library_image_preview = None;
        if !self.has_preview_2d_panel {
            self.status_message = Some((
                "no 2D preview panel open — use a panel's corner menu to add one".to_string(),
                std::time::Instant::now(),
            ));
        }
        //self.needs_to_save = true;
    }

    /// Loads `path` off the graph and shows it in the 2D preview panel. Takes
    /// precedence over any node output being viewed (`view_node` clears this in
    /// the other direction). Returns an error string if decoding fails.
    pub fn preview_library_image(&mut self, path: PathBuf) -> Result<(), String> {
        let image = mangler_core::operations::images::inputs::file::load_image_from_path(&path)?;
        self.library_image_preview = Some(LibraryImagePreview {
            path,
            image: Arc::new(image),
        });
        if !self.has_preview_2d_panel {
            self.status_message = Some((
                "no 2D preview panel open — use a panel's corner menu to add one".to_string(),
                std::time::Instant::now(),
            ));
        }
        Ok(())
    }

    /// The library image currently shown in the 2D preview, if any. Used by the
    /// Libraries panel to highlight the matching row.
    pub fn previewed_library_image(&self) -> Option<&Path> {
        self.library_image_preview.as_ref().map(|p| p.path.as_path())
    }

    /// Binds all of the 3D preview panels' material channels from a material
    /// export node's input connections (right-click on the node in the graph).
    ///
    /// There's no "focused panel" concept for the 3D viewers — the default
    /// layout has a single 3D panel anyway — so this deliberately applies the
    /// binding to every open 3D panel rather than picking one.
    ///
    /// Purely a GUI-side state change: no engine messages are sent. The
    /// channels are resolved from live node data next frame by the existing
    /// `resolve_material` (called from the 3D panel's own show code).
    fn bind_material_node_to_3d(&mut self, node_id: &str) {
        let Some(node) = self.graph_editor.graph_nodes.get(node_id) else {
            // Node vanished (e.g. deleted the same frame) — nothing to bind.
            return;
        };

        // Collect (channel, upstream connection) pairs first so the immutable
        // borrow of graph_nodes ends before we mutate self.viewers_3d below.
        let bindings: Vec<(MaterialChannel, Option<(String, usize)>)> = (0..=7)
            .filter_map(|input_index| {
                let channel = material_input_channel(input_index)?;
                let connection = node
                    .inputs
                    .get(input_index)
                    .and_then(|input| input.connection.clone());
                Some((channel, connection))
            })
            .collect();

        for panel in self.viewers_3d.values_mut() {
            for (channel, connection) in &bindings {
                match connection {
                    Some((upstream_node_id, output_index)) => {
                        panel.assignments.set(
                            *channel,
                            MaterialAssignment {
                                node_id: upstream_node_id.clone(),
                                output_index: *output_index,
                            },
                        );
                    }
                    None => panel.assignments.clear(*channel),
                }
            }
        }

        self.status_message = Some(if self.viewers_3d.is_empty() {
            (
                "no 3D preview panel open — use a panel's corner menu to add one".to_string(),
                std::time::Instant::now(),
            )
        } else {
            (
                "material bound to 3D view".to_string(),
                std::time::Instant::now(),
            )
        });
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

    /// Paste nodes from the clipboard into the graph.
    ///
    /// Creates new nodes at positions offset from the cursor, restores input values
    /// and internal connections, then selects all newly pasted nodes.
    fn paste_clipboard(&mut self, cb: &Clipboard) {
        // Compute paste offset: center the pasted nodes on the current pointer position,
        // using the camera of the main-window graph panel under the pointer (falling
        // back to the first main-window graph panel, then identity).
        let centroid = cb.centroid();
        let (zoom, position) = self.camera_at(self.pointer_position);
        let paste_target = view_to_graph_space_pos2(
            zoom,
            self.pointer_position,
        ) - position.to_vec2();
        let offset = egui::Vec2::new(
            paste_target.x - centroid.x,
            paste_target.y - centroid.y,
        );

        // Map old node IDs to new node IDs.
        let mut id_map: HashMap<String, String> = HashMap::new();

        // Create nodes.
        for clipboard_node in &cb.nodes {
            let new_pos = Pos2::new(
                clipboard_node.position.x + offset.x,
                clipboard_node.position.y + offset.y,
            );

            // The input values travel with the AddNode message so the engine
            // applies them before echoing the node back — the local node is
            // then built with the pasted values, not defaults. (Images are
            // excluded by the clipboard; connected inputs get their values
            // from propagation once connections are restored below.)
            if let Ok(new_id) = self.add_node(
                clipboard_node.node_type.clone(),
                new_pos,
                clipboard_node.is_enabled,
                clipboard_node.custom_name.clone(),
                clipboard_node.input_values.clone(),
            ) {
                id_map.insert(clipboard_node.original_id.clone(), new_id.clone());
            }
        }

        // Recreate internal connections using remapped IDs.
        for conn in &cb.connections {
            if let (Some(new_output_id), Some(new_input_id)) = (
                id_map.get(&conn.output_node_id),
                id_map.get(&conn.input_node_id),
            ) {
                self.add_connection(
                    new_input_id.clone(),
                    conn.input_index,
                    new_output_id.clone(),
                    conn.output_index,
                );
            }
        }

        // Select all newly pasted nodes.
        self.graph_editor.selected_node_ids.clear();
        for new_id in id_map.values() {
            self.graph_editor.selected_node_ids.insert(new_id.clone());
        }

        // Edit the first pasted node.
        if let Some(first_id) = id_map.values().next() {
            self.editing_node_id = Some(first_id.clone());
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
                    !input.hide_in_graph
                        && (input.accepts_any_type
                            || input
                                .value
                                .value_type()
                                .valid_conversions()
                                .contains(&conn.from_value_type))
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
                if let Some(output_index) = outputs
                    .iter()
                    .position(|output| valid_from.contains(&output.value.value_type()))
                {
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

/// Drain the selected-node set and clear the editing node, returning the IDs to delete.
///
/// Returns an empty vec when there is nothing selected.
fn collect_selected_nodes_to_delete(
    selected_node_ids: &mut std::collections::HashSet<String>,
    editing_node_id: &mut Option<String>,
) -> Vec<String> {
    if selected_node_ids.is_empty() {
        return Vec::new();
    }
    let ids: Vec<String> = selected_node_ids.drain().collect();
    *editing_node_id = None;
    ids
}

/// Scan a frame's events and return `(copy, paste_text)`.
///
/// - `copy` is true when `Event::Copy` fires (Ctrl/Cmd+C).
/// - `paste_text` contains the system clipboard text when `Event::Paste` fires (Ctrl/Cmd+V).
///   Returns `None` if no paste event occurred.
///
/// We rely entirely on `Event::Copy` and `Event::Paste` which are emitted by egui-winit.
/// `Event::Key` is not used because egui-winit intercepts Ctrl+C/V on key-down and only
/// emits key-release events with unreliable modifier state.
fn detect_copy_paste(events: &[egui::Event]) -> (bool, Option<String>) {
    let mut copy = false;
    let mut paste_text: Option<String> = None;
    for event in events {
        match event {
            egui::Event::Copy => copy = true,
            egui::Event::Paste(text) => paste_text = Some(text.clone()),
            _ => {}
        }
    }
    (copy, paste_text)
}

/// Convert a `Thumbnail` + output `Value` into the GUI's per-node thumbnail
/// representation. Used by both the `OutputChanged` handler (with an inline
/// thumbnail) and the `ThumbnailReady` handler (where the async service
/// delivers the thumbnail after the value has already been stored).
///
/// Passing `None` for `thumbnail` produces `Text("None")` — the UI's
/// equivalent of "no thumbnail data" — which mirrors the pre-async
/// behaviour. Callers that want to preserve the previous thumbnail (e.g.
/// mid-scrub, before the async one arrives) should skip calling this and
/// leave `node.thumbnail` untouched.
fn build_graph_node_thumbnail(
    ctx: &egui::Context,
    node_id: &str,
    thumbnail: Option<mangler_core::thumbnail::Thumbnail>,
    value: &Value,
) -> Option<GraphNodeThumbnail> {
    use mangler_core::thumbnail::Thumbnail;
    match thumbnail {
        Some(Thumbnail::Image(thumbnail)) => match value {
            Value::Color(_) => {
                let pixels = thumbnail.as_flat_samples();
                let size = [thumbnail.width() as usize, thumbnail.height() as usize];
                let color_image =
                    ColorImage::from_rgba_unmultiplied(size, pixels.as_slice());
                Some(GraphNodeThumbnail::Color {
                    texture_handle: ctx.load_texture(
                        node_id.to_owned(),
                        color_image,
                        Default::default(),
                    ),
                })
            }
            Value::Image { data, change_id: _ } => {
                let pixels = thumbnail.as_flat_samples();
                let size = [thumbnail.width() as usize, thumbnail.height() as usize];
                let color_image =
                    ColorImage::from_rgba_unmultiplied(size, pixels.as_slice());
                Some(GraphNodeThumbnail::Image {
                    texture_handle: ctx.load_texture(
                        node_id.to_owned(),
                        color_image,
                        Default::default(),
                    ),
                    width: data.width(),
                    height: data.height(),
                    channels: data.channels(),
                })
            }
            _ => None,
        },
        Some(Thumbnail::Text(v)) => Some(GraphNodeThumbnail::Text(v)),
        None => Some(GraphNodeThumbnail::Text("None".to_string())),
    }
}

#[cfg(test)]
#[path = "program_tests.rs"]
mod tests;
