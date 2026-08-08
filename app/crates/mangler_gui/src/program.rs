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
use std::sync::{Arc, Mutex};
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
        curve_overlay,
        spatial_overlay,
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

/// A library image being decoded on a background `std::thread` (see
/// [`Program::preview_library_image`]). Decoding is off the UI thread because
/// it can take hundreds of milliseconds â€” a camera raw has to be developed, and
/// large TIFF/EXR/HEIC files are no faster â€” which would otherwise freeze the
/// window for the whole decode.
struct PendingLibraryPreview {
    /// The file being decoded. Keeps the Libraries row highlighted while the
    /// decode is in flight and names the loading placeholder.
    path: PathBuf,
    /// Which request this is. Compared against
    /// [`Program::library_preview_generation`] when the result lands so a
    /// superseded decode is discarded instead of stealing the panel â€” the same
    /// stale-check discipline `mangler_core::thumbnail_service` uses.
    generation: u64,
    /// Where the worker thread leaves its result. `None` until it finishes.
    slot: Arc<Mutex<Option<Result<FloatImage, String>>>>,
}

/// Outcome of checking a background decode's slot.
#[derive(Debug, PartialEq)]
enum PreviewPoll<T> {
    /// Still decoding â€” keep showing the loading placeholder.
    Pending,
    /// Finished and still the newest request.
    Ready(T),
    /// Finished, but a newer request superseded it: drop the result.
    Stale,
}

/// Reads a finished background decode out of `slot`, discarding it when
/// `generation` no longer matches `current_generation` (last click wins).
///
/// Generic over the payload so the stale/ready decision is testable without a
/// decoded image or a live egui context.
fn poll_preview_slot<T>(
    slot: &Mutex<Option<T>>,
    generation: u64,
    current_generation: u64,
) -> PreviewPoll<T> {
    let Some(result) = slot.lock().unwrap().take() else {
        return PreviewPoll::Pending;
    };
    if generation == current_generation {
        PreviewPoll::Ready(result)
    } else {
        PreviewPoll::Stale
    }
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
    /// a newer NodeMangler, and/or unknown nodes preserved as placeholders â€”
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
    /// union of trees â€” `Program` cannot see the panel tree itself â€” and used
    /// to hint the user when viewing a node with nowhere to show it.
    pub has_preview_2d_panel: bool,
    /// Per-leaf 2D preview pan/zoom state, keyed by panel leaf id.
    viewers_2d: HashMap<LeafId, ImageViewer>,
    /// Per-leaf 3D preview state (arcball camera + material channel bindings).
    viewers_3d: HashMap<LeafId, Preview3dPanel>,
    /// Per-leaf 2D-preview backdrop override: `true` means show the output the
    /// user explicitly right-clicked instead of the gizmo node's source image.
    /// Set for every live leaf by `view_node` (an explicit act beats the
    /// automatic choice) and cleared by `edit_node` when the selection changes
    /// (a new node re-arms the automatic choice). Cleared from the conflict
    /// strip's "Show source" button when the panel is not showing the source.
    /// Absent / false means "automatic".
    gizmo_backdrop_prefer_viewed: HashMap<LeafId, bool>,
    /// Per-leaf graph pan/zoom camera, keyed by panel leaf id â€” mirrors
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
    /// window's local coordinates â€” screen space is the common frame.
    graph_rects_screen: HashMap<LeafId, (Rect, Pos2)>,
    /// Pointer position in screen points while a node-list drag is active,
    /// published by the window holding the mouse capture so every window can
    /// hit-test and draw the ghost node.
    menu_drag_pointer_screen: Option<Pos2>,
    /// Image file being dragged out of the Libraries panel, if any. Set when
    /// an image row's drag starts (via `LibraryAction::BeginImageDrag`) and
    /// dropped onto a graph panel by `show_menu_drag`, which creates an "image
    /// from file" node at the drop position â€” mirroring the node-list drag
    /// (`dragging_menu_button`) but carrying a path instead of an operation.
    dragging_library_image: Option<PathBuf>,
    /// Display name to show when this graph has no save path yet (a brand-new
    /// unsaved tab). Once a save path exists the name is derived purely from
    /// the file stem â€” see [`Self::display_name`].
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
    /// A library image whose decode is still running (see
    /// [`PendingLibraryPreview`]). While set, the 2D panel draws a loading
    /// placeholder and the Libraries row for `path` stays highlighted.
    pending_library_preview: Option<PendingLibraryPreview>,
    /// Monotonic id handed to each background decode. Bumped by every request
    /// *and* by `view_node`, so anything already in flight when the user picks
    /// something else lands stale and is thrown away.
    library_preview_generation: u64,
    /// Failure from a background decode, held until `App` drains it into the
    /// Libraries panel's error line ([`Self::take_library_preview_error`]).
    /// Errors can no longer be returned from the request call â€” they only
    /// exist a frame or more later.
    library_preview_error: Option<String>,
    /// Fit-request counter for the 2D preview panels: bumped whenever the user
    /// explicitly picks something to view (right-clicks a node output, clicks
    /// a library image), so each per-leaf [`ImageViewer`] centers and frames
    /// the image once (it remembers the last value it consumed).
    view_fit_seq: u64,
    /// Set when the engine confirms a `SetSavePath`-triggered write
    /// ([`GraphChangedMessage::SavedTo`]). `App`'s deferred-close state
    /// machine takes it via [`Self::take_confirmed_save`] to complete a
    /// close that was waiting on the save.
    confirmed_save: Option<PathBuf>,
    /// State of an in-progress batch run over a "from folder" node's images:
    /// `(node_id, completed, total)`. Driven by the engine's
    /// [`GraphChangedMessage::BatchProgress`] stream and cleared on
    /// [`GraphChangedMessage::BatchFinished`]. Per-`Program` (each tab has its
    /// own engine), so no app-global state is needed. Used to render the node
    /// settings panel's batch progress bar for the matching node.
    batch_run: Option<(String, usize, usize)>,
    /// The active watch on a "from folder" node, if any (see [`WatchRun`]).
    /// Set from the engine's [`GraphChangedMessage::WatchStatus`] snapshots
    /// and cleared on [`GraphChangedMessage::WatchStopped`]. Mutually
    /// exclusive with `batch_run` â€” both drive the same node.
    watch_run: Option<WatchRun>,
}

/// An active watch session: which "from folder" node the engine is watching,
/// plus the last counters it reported. One per `Program`, since a `Program`
/// owns a single engine.
struct WatchRun {
    /// The watched node.
    node_id: String,
    /// The latest whole-state snapshot from the engine.
    status: node_settings_panel::WatchStatusView,
}

/// Status-message text for a watch that has ended.
fn watch_stopped_message(
    captured: usize,
    skipped: usize,
    reason: mangler_core::WatchStopReason,
) -> String {
    use mangler_core::WatchStopReason;
    match reason {
        WatchStopReason::Stopped => {
            let mut text = format!("watch stopped: {captured} frames captured");
            if skipped > 0 {
                text.push_str(&format!(", {skipped} skipped"));
            }
            text
        }
        WatchStopReason::NodeDeleted => "watch stopped: node deleted".to_string(),
        WatchStopReason::FolderChanged => "watch stopped: the folder input changed".to_string(),
        WatchStopReason::Refused => {
            "can't watch: check the node's folder, or stop the running batch".to_string()
        }
    }
}

/// Status-message text for a batch run that has ended. A cancel with
/// `total == 0` means it never started, and `watching` separates the two
/// causes of that: an empty/bad folder, or the watch that already owns the
/// node (the engine refuses a batch while watching).
fn batch_finished_message(
    completed: usize,
    total: usize,
    cancelled: bool,
    watching: bool,
) -> String {
    if !cancelled {
        format!("batch finished: {completed} images")
    } else if total > 0 {
        format!("batch cancelled at {completed}/{total}")
    } else if watching {
        "can't run a batch while watching a folder".to_string()
    } else {
        "batch: no images found in the folder".to_string()
    }
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
                gizmo_backdrop_prefer_viewed: HashMap::new(),
                viewers_3d: HashMap::new(),
                graph_cameras: HashMap::new(),
                popup_graph_leaf: None,
                main_graph_rects: Vec::new(),
                graph_rects_screen: HashMap::new(),
                menu_drag_pointer_screen: None,
                dragging_library_image: None,
                fallback_name: "new graph".to_string(),
                graph_name_buffer: String::new(),
                pending_open_graphs: Vec::new(),
                library_image_preview: None,
                pending_library_preview: None,
                library_preview_generation: 0,
                library_preview_error: None,
                view_fit_seq: 0,
                confirmed_save: None,
                batch_run: None,
                watch_run: None,
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
    /// accurate "is the graph blank right now" check. Gates the unsaved-close
    /// prompt: an empty unsaved tab has nothing to lose and closes silently.
    pub fn is_empty(&self) -> bool {
        self.graph_editor.graph_nodes.is_empty()
    }

    /// True when closing this program should prompt the user: never saved
    /// AND has content worth saving. Saved graphs are covered by auto-save;
    /// empty unsaved graphs have nothing to lose.
    pub fn has_unsaved_content(&self) -> bool {
        self.app.save_path.is_none() && !self.is_empty()
    }

    /// Takes the engine's confirmation that a `SetSavePath` write hit disk
    /// (see [`GraphChangedMessage::SavedTo`]). One-shot by design: `App`'s
    /// close state machine polls this each frame while waiting to close a
    /// just-saved tab.
    pub fn take_confirmed_save(&mut self) -> Option<PathBuf> {
        self.confirmed_save.take()
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

    /// Points this program's graph at a save path. Used for the first save of
    /// an unsaved graph, save-as, the Libraries panel's create-graph flow, and
    /// re-targeting a tab after its file was renamed on disk. The display name
    /// follows the path automatically (see [`Self::display_name`]), so there's
    /// no separate name to set: update the GUI-side save path, then tell the
    /// engine, which writes the file immediately and acks with
    /// [`GraphChangedMessage::SavedTo`].
    pub fn set_save_location(&mut self, path: PathBuf) {
        self.app.save_path = Some(path.clone());

        if let Err(err) = self
            .tx_change_graph
            .try_send(ChangeGraphMessage::SetSavePath(path))
        {
            println!("Error sending graph_message: {:?}", err);
        }
    }

    /// Creates an image input node wired to `path` (see
    /// [`Self::add_image_from_file_at`] for how the node type is chosen) and
    /// drops it near the focused graph panel's centre (with a little random
    /// jitter so repeated adds don't stack exactly). Shared by the
    /// drag-and-drop handler and the Libraries panel's "add to current graph"
    /// action.
    pub fn add_image_from_file(&mut self, path: PathBuf) {
        // Pick a screen point inside the focused graph panel, then map it into
        // graph space through that panel's camera. `main_graph_rects` holds
        // last frame's panel rects â€” the same source `camera_at` relies on for
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
        self.add_image_from_file_at(path, graph_pos);
    }

    /// Creates an image input node wired to `path` at an explicit graph-space
    /// position. Used by the Libraries-panel drag-and-drop handler, which knows
    /// exactly where the user dropped the image; `add_image_from_file` is the
    /// jittered-center wrapper over this.
    ///
    /// Camera raw files get a `from raw` node rather than `from file`. Both can
    /// open a raw, but `from file` develops it with the camera's as-shot
    /// settings and offers no way to change them, so landing on `from raw`
    /// puts the white-balance, encoding and resolution controls in front of the
    /// user instead of making them swap the node out first. Their `path` input
    /// is index 0 in both cases.
    pub fn add_image_from_file_at(&mut self, path: PathBuf, graph_pos: Pos2) {
        use mangler_core::operations::Operation;
        let operation = if mangler_core::operations::images::inputs::file::is_raw_file(&path) {
            Operation::OpImageInputRaw
        } else {
            Operation::OpImageInputFile
        };
        // The path goes through `AddNode`'s initial input values (not a
        // follow-up `SetInput`) so the engine's `AddedNode` echo â€” which the
        // GUI builds its local node from â€” already carries it. A `SetInput`
        // sent after `AddNode` is never echoed back, leaving the settings
        // panel showing an empty path even though the engine loaded the file.
        if let Err(err) = self.add_node(
            AddNodeType::Operation(operation),
            graph_pos,
            true,
            None,
            vec![(0, Value::Path(path))],
        ) {
            println!("Error adding image node: {}", err.0);
        }
    }

    /// Begins dragging a Libraries-panel image into a graph. Records the path
    /// so `show_menu_drag` can draw the ghost and drop an "image from file"
    /// node wherever the drag ends over a graph panel.
    pub fn begin_library_image_drag(&mut self, path: PathBuf) {
        self.dragging_library_image = Some(path);
    }

    /// Takes (and clears) the `.mangler.json` files dropped onto this program's
    /// window this frame. `App` drains these after `update` and opens each in
    /// a tab (via `open_or_focus`), which the program itself can't do.
    pub fn take_pending_open_graphs(&mut self) -> Vec<PathBuf> {
        std::mem::take(&mut self.pending_open_graphs)
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

        // Also polled per 2D panel; doing it here as well means a decode still
        // completes (and a failure still reports) with no 2D panel open.
        self.poll_library_preview();

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
                    self.status_message = Some(("Nothing to copy â€” select a node first".to_string(), std::time::Instant::now()));
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
                            // already skips nodes with no op â€” see
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
                            "Saved with NodeMangler {} â€” you're on {}. Auto-save paused until you edit.",
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
                    // pending â€” remember the path; show_overlays renders the
                    // Reload-vs-Overwrite modal while this is set.
                    self.file_conflict = Some(path);
                }
                GraphChangedMessage::SaveError { path, message } => {
                    // Writing the save file failed (missing/unwritable
                    // directory, disk full, ...). Not fatal â€” the edit is
                    // still in memory and the next auto-save tick will try
                    // again â€” so this is a fading status message rather
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
                GraphChangedMessage::SavedTo { path } => {
                    // The engine confirmed a SetSavePath-triggered write hit
                    // disk (first save or save-as). Adopting the path is
                    // normally a no-op (the save-path setter already did it);
                    // the remembered confirmation is what App's deferred-close
                    // state machine waits on before aborting the engine task.
                    self.app.save_path = Some(path.clone());
                    self.confirmed_save = Some(path);
                }
                GraphChangedMessage::BatchProgress {
                    node_id,
                    completed,
                    total,
                } => {
                    // One iteration of the active batch finished; remember the
                    // latest progress so the node settings panel can draw its
                    // bar. Arrives once per item, in order.
                    self.batch_run = Some((node_id, completed, total));
                }
                GraphChangedMessage::BatchFinished {
                    node_id: _,
                    completed,
                    total,
                    cancelled,
                } => {
                    // The batch ended â€” clear the running state and surface a
                    // fading status message describing the outcome. A cancel
                    // with total == 0 means it never started (no images found /
                    // bad folder / wrong node / a watch already owns the node).
                    self.batch_run = None;
                    let text = batch_finished_message(
                        completed,
                        total,
                        cancelled,
                        self.watch_run.is_some(),
                    );
                    self.status_message = Some((text, std::time::Instant::now()));
                }
                GraphChangedMessage::WatchStatus {
                    node_id,
                    captured,
                    pending,
                    skipped,
                    last_file,
                    error,
                } => {
                    // A whole-state snapshot, so replace rather than merge â€”
                    // that's what makes a dropped message self-heal.
                    self.watch_run = Some(WatchRun {
                        node_id,
                        status: node_settings_panel::WatchStatusView {
                            captured,
                            pending,
                            skipped,
                            last_file,
                            error,
                        },
                    });
                }
                GraphChangedMessage::WatchStopped {
                    node_id: _,
                    captured,
                    skipped,
                    reason,
                } => {
                    // Also covers a refused start, which never set the state.
                    self.watch_run = None;
                    self.status_message = Some((
                        watch_stopped_message(captured, skipped, reason),
                        std::time::Instant::now(),
                    ));
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
                // Source image for the tone-curve editor's histogram: the
                // upstream output feeding this node's first connected image
                // input. Resolved before the mutable node borrow below (the
                // upstream node lives in the same map), as a cheap Arc clone.
                // `None` when the node has no editable tone-curve input.
                let upstream_image = self
                    .graph_editor
                    .graph_nodes
                    .get(editing_node_id)
                    .filter(|node| {
                        node.inputs.iter().any(|i| {
                            matches!(i.settings, Some(mangler_core::input::InputSettings::ToneCurve))
                                && i.connection.is_none()
                        })
                    })
                    .and_then(|node| {
                        node.inputs
                            .iter()
                            .find(|i| matches!(i.value, Value::Image { .. }) && i.connection.is_some())
                            .and_then(|i| i.connection.as_ref())
                            .and_then(|(nid, oidx)| {
                                self.graph_editor.graph_nodes.get(nid)?.outputs.get(*oidx)
                            })
                            .and_then(|o| match &o.value {
                                Value::Image { data, change_id } => {
                                    Some((data.clone(), change_id.clone()))
                                }
                                _ => None,
                            })
                    });

                // Batch progress for the panel: `Some` only when a batch is
                // running for *this* node. Resolved before the mutable node
                // borrow below (immutable read of the disjoint `batch_run`).
                let batch_progress = self.batch_run.as_ref().and_then(|(id, c, t)| {
                    (id == editing_node_id).then_some((*c, *t))
                });

                // Same again for the watch: its counters only when *this* node
                // is the watched one, plus the two flags the mutually
                // exclusive start buttons need.
                let watch = node_settings_panel::WatchPanelState {
                    here: self
                        .watch_run
                        .as_ref()
                        .filter(|w| &w.node_id == editing_node_id)
                        .map(|w| w.status.clone()),
                    watch_active: self.watch_run.is_some(),
                    batch_active: self.batch_run.is_some(),
                };

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
                            upstream_image,
                            batch_progress,
                            watch,
                        );
                    show_graph_settings = false;

                    // Start a batch run over this from-folder node's images.
                    // Handled before the deselect below, which nulls
                    // `editing_node_id` (whose borrow this clone still needs).
                    if node_settings_response.run_batch {
                        let message = ChangeGraphMessage::RunBatch {
                            node_id: editing_node_id.clone(),
                        };
                        if let Err(err) = self.tx_change_graph.try_send(message) {
                            println!("Error sending graph_message: {:?}", err);
                        }
                    }

                    // Cancel the active batch run after the in-flight item.
                    if node_settings_response.cancel_batch {
                        if let Err(err) =
                            self.tx_change_graph.try_send(ChangeGraphMessage::CancelBatch)
                        {
                            println!("Error sending graph_message: {:?}", err);
                        }
                    }

                    // Start watching this from-folder node's folder for newly
                    // arriving photos (tethered shooting).
                    if node_settings_response.start_watch {
                        let message = ChangeGraphMessage::StartWatch {
                            node_id: editing_node_id.clone(),
                        };
                        if let Err(err) = self.tx_change_graph.try_send(message) {
                            println!("Error sending graph_message: {:?}", err);
                        }
                    }

                    // Stop the watch after the in-flight frame.
                    if node_settings_response.stop_watch {
                        if let Err(err) =
                            self.tx_change_graph.try_send(ChangeGraphMessage::StopWatch)
                        {
                            println!("Error sending graph_message: {:?}", err);
                        }
                    }

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

                // name committed. Saved graph -> rename the file on disk. We
                // do NOT optimistically update save_path here: the rename can
                // fail (name collision), and the engine's SaveError → status
                // message explains it. On success FileRenamed updates the
                // path, and display_name (hence the tab title) follows.
                // Unsaved graph -> there is no file to rename; the name is a
                // GUI-side pending value that becomes the on-disk stem at
                // first save (it seeds the save dialog's file name).
                if let Some(new_stem) = graph_settings_response.new_name {
                    if self.app.save_path.is_some() {
                        let message = ChangeGraphMessage::RenameFile { new_stem };

                        match self.tx_change_graph.try_send(message) {
                            Ok(_) => {}
                            Err(err) => {
                                println!("Error sending graph_message: {:?}", err);
                            }
                        }
                    } else {
                        let trimmed = new_stem.trim();
                        if !trimmed.is_empty() {
                            self.fallback_name = trimmed.to_string();
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

        // Graph run timing and interaction help live inside the graph panel â€”
        // they describe the graph, not the whole app â€” anchored to this
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

    /// Send one `SetInput` per touched input, reading each input's
    /// *accumulated local value* rather than a per-frame payload â€” an overlay's
    /// drag-release frame carries no pointer movement, so the frame's `changed`
    /// list is empty by then (see `overlay::Gesture`).
    ///
    /// All the messages land in one engine tick, and the engine drains its whole
    /// node channel before calling `graph.run()`, so a four-input crop gesture
    /// still costs exactly one graph run.
    fn commit_node_inputs(
        nodes: &HashMap<String, GraphNode>,
        tx: &mpsc::Sender<ChangeNodeMessage>,
        node_id: &str,
        input_indices: &[usize],
    ) {
        let mut sent: Vec<usize> = Vec::with_capacity(input_indices.len());
        for &input_index in input_indices {
            if sent.contains(&input_index) {
                continue;
            }
            sent.push(input_index);
            let Some(value) = nodes
                .get(node_id)
                .and_then(|node| node.inputs.get(input_index))
                .map(|input| input.value.clone())
            else {
                continue;
            };
            let message =
                ChangeNodeMessage::SetInput { node_id: node_id.to_owned(), input_index, value };
            if let Err(err) = tx.try_send(message) {
                println!("Error sending SetInput: {:?}", err);
            }
        }
    }

    fn show_preview_2d_panel(&mut self, ui: &mut egui::Ui, leaf_id: LeafId, theme: &Theme) {
        // Before the borrow below, so a decode that landed this frame is drawn
        // now rather than a frame late.
        self.poll_library_preview();

        // Destructure so the per-leaf viewer and the graph nodes can be
        // borrowed simultaneously (disjoint fields). `tx_change_node` is taken
        // here too so an overlay can commit without re-borrowing `self`.
        let Self {
            viewers_2d,
            graph_editor,
            viewing_node_id_index,
            library_image_preview,
            pending_library_preview,
            view_fit_seq,
            editing_node_id,
            tx_change_node,
            gizmo_backdrop_prefer_viewed,
            ..
        } = self;
        let view_fit_seq = *view_fit_seq;

        let viewer = viewers_2d.entry(leaf_id).or_insert_with(ImageViewer::new);
        let prefer_viewed = *gizmo_backdrop_prefer_viewed.entry(leaf_id).or_insert(false);

        // Capture the panel rect before any child drawing advances the cursor
        // (same formula as `image_viewer::show`).
        let view_rect = Rect::from_min_size(ui.cursor().left_top(), ui.available_size());

        // Overlay editing is selection-driven: it acts on the *edited*
        // (settings-panel) node, never the viewed one. Resolved up front as an
        // immutable read that ends here via clone, so the mutable node borrow
        // further down is a fresh sequential borrow.
        let editor = resolve_preview_editor(&graph_editor.graph_nodes, editing_node_id.as_deref());

        // A gizmo's numbers are relative to one specific image, so the panel
        // shows that image rather than whatever was last right-clicked — which
        // for the flagship nodes is exactly the useless one (`sample pixel`
        // outputs a colour swatch, `crop` outputs the already-cropped result).
        let gizmo_backdrop = match &editor {
            Some(PreviewEditor::Gizmos { node_id, .. }) if !prefer_viewed => {
                gizmo_backdrop_source(&graph_editor.graph_nodes, node_id)
            }
            _ => None,
        };

        // Normal content dispatch. Records the displayed image's dimensions so
        // an overlay can map onto the same on-screen rect `draw_image` uses.
        let mut displayed_dims: Option<(f32, f32)> = None;
        // The backdrop's pixels, but only when they really are the gizmo node's
        // spatial source — the colour readout must never sample an unrelated
        // image and present it as the sampled value.
        let mut source_pixels: Option<std::sync::Arc<mangler_core::float_image::FloatImage>> = None;
        let mut backdrop_label: Option<String> = None;

        // A clicked library image takes precedence over everything: it is the
        // most recent explicit act, and `view_node` already clears it.
        if let Some(preview) = library_image_preview.as_ref() {
            viewer.show(
                ui,
                "__library_image_preview__".to_string(),
                0,
                preview.path.to_string_lossy().into_owned(),
                &preview.image,
                true, // fit each newly-opened library image to the view
                view_fit_seq,
                theme,
            );
            displayed_dims = Some((preview.image.width() as f32, preview.image.height() as f32));
            backdrop_label = Some(format!(
                "{} (not this node's source)",
                preview
                    .path
                    .file_name()
                    .map(|n| n.to_string_lossy().into_owned())
                    .unwrap_or_else(|| preview.path.to_string_lossy().into_owned())
            ));
        } else if let Some(pending) = pending_library_preview.as_ref() {
            // Decoding on a background thread: hold the panel with a named
            // placeholder rather than falling back to a node output the user
            // didn't just ask for.
            let name = pending
                .path
                .file_name()
                .map(|n| n.to_string_lossy().into_owned())
                .unwrap_or_else(|| pending.path.to_string_lossy().into_owned());
            preview_2d::show_loading(ui, &name, theme);
        } else if let Some((backdrop_id, backdrop_index)) = gizmo_backdrop.as_ref() {
            if let Some((data, change_id)) =
                image_output(&graph_editor.graph_nodes, backdrop_id, *backdrop_index)
            {
                let (w, h) = (data.width() as f32, data.height() as f32);
                // Switching backdrops can swap a 512px mask for a 6000px photo,
                // so frame it once per new source rather than leaving it
                // off-screen under the previous pan/zoom.
                viewer.fit_on_source_change(backdrop_id, *backdrop_index, view_rect, w, h);
                viewer.show(
                    ui,
                    backdrop_id.clone(),
                    *backdrop_index,
                    change_id,
                    &data,
                    false,
                    view_fit_seq,
                    theme,
                );
                displayed_dims = Some((w, h));
                backdrop_label =
                    Some(describe_output(&graph_editor.graph_nodes, backdrop_id, *backdrop_index));
                source_pixels = Some(data);
            } else {
                show_editor_hint(ui, &editor, theme);
            }
        } else if let Some((viewing_node_id, output_index)) = viewing_node_id_index.as_ref() {
            if let Some(graph_node) = graph_editor.graph_nodes.get(viewing_node_id) {
                preview_2d::show(ui, viewer, graph_node, *output_index, view_fit_seq, theme);
                if let Some(output) = graph_node.outputs.get(*output_index) {
                    if let Value::Image { data, .. } = &output.value {
                        displayed_dims = Some((data.width() as f32, data.height() as f32));
                        // Only the gizmo node's own source may feed the colour
                        // readout; anything else is a coincidence of whatever
                        // the user happened to be viewing.
                        if let Some(PreviewEditor::Gizmos { node_id, .. }) = &editor {
                            let is_source =
                                gizmo_backdrop_source(&graph_editor.graph_nodes, node_id)
                                    .is_some_and(|(id, idx)| {
                                        &id == viewing_node_id && idx == *output_index
                                    });
                            if is_source {
                                source_pixels = Some(data.clone());
                            }
                        }
                    }
                }
                backdrop_label = Some(describe_output(
                    &graph_editor.graph_nodes,
                    viewing_node_id,
                    *output_index,
                ));
            } else {
                show_editor_hint(ui, &editor, theme);
            }
        } else {
            show_editor_hint(ui, &editor, theme);
        }

        // Draw the editing overlay on top of whatever was displayed.
        let Some(editor) = editor else {
            return;
        };

        // Map onto the displayed image's screen rect (the same call
        // `draw_image` makes), else a letterboxed fallback canvas.
        let image_rect = match displayed_dims {
            Some((w, h)) => viewer.displayed_image_rect(view_rect, w, h),
            None => crate::overlay::mapping::fallback_canvas_rect(view_rect),
        };

        match editor {
            PreviewEditor::Curve { node_id, input_index, curve } => {
                let resp = curve_overlay::show(ui, leaf_id, view_rect, image_rect, &curve, theme);

                if let Some(new_curve) = resp.changed {
                    // Local mutate every frame for instant feedback; the
                    // immutable borrows above are done, so this `get_mut` is a
                    // fresh sequential borrow.
                    if let Some(node) = graph_editor.graph_nodes.get_mut(&node_id) {
                        if let Some(input) = node.inputs.get_mut(input_index) {
                            input.value = Value::Curve(new_curve);
                        }
                    }
                }
                // Push to the engine only when the gesture completed.
                if resp.commit {
                    Self::commit_node_inputs(
                        &graph_editor.graph_nodes,
                        tx_change_node,
                        &node_id,
                        &[input_index],
                    );
                }
            }
            PreviewEditor::Gizmos { node_id, specs } => {
                let Some(node) = graph_editor.graph_nodes.get(&node_id) else {
                    return;
                };
                let resp = spatial_overlay::show(
                    ui,
                    leaf_id,
                    view_rect,
                    image_rect,
                    &spatial_overlay::GizmoContext {
                        specs,
                        inputs: &node.inputs,
                        image_dims: displayed_dims.map(|(w, h)| (w as u32, h as u32)),
                    },
                    theme,
                );

                if !resp.changed.is_empty() {
                    if let Some(node) = graph_editor.graph_nodes.get_mut(&node_id) {
                        for (input_index, value) in resp.changed {
                            if let Some(input) = node.inputs.get_mut(input_index) {
                                input.value = value;
                            }
                        }
                    }
                }
                if resp.commit {
                    Self::commit_node_inputs(
                        &graph_editor.graph_nodes,
                        tx_change_node,
                        &node_id,
                        &resp.commit_inputs,
                    );
                }

                // Silent when auto-showing the gizmo's spatial source (the common
                // case). Only surface chrome when the panel is showing something
                // else — library preview, a pinned "keep viewed" output, etc. —
                // so the user can recover without living under a permanent bar.
                // `source_pixels` is set only when the displayed image *is* that
                // source (see the dispatch above).
                let showing_source = source_pixels.is_some();
                if !showing_source {
                    if let Some(label) = backdrop_label {
                        let restore = crate::overlay::strip::top_left(
                            ui,
                            view_rect,
                            egui::Vec2::new(420.0, 26.0),
                            theme,
                            |ui| {
                                ui.label(
                                    egui::RichText::new(format!(
                                        "Showing {label} — not this node's source"
                                    ))
                                    .color(theme.get().text_faint),
                                );
                                ui.small_button("Show source").clicked()
                            },
                        );
                        if restore {
                            gizmo_backdrop_prefer_viewed.insert(leaf_id, false);
                            // Library preview outranks the gizmo backdrop in the
                            // dispatch above; clear it so auto can take over.
                            *library_image_preview = None;
                            *pending_library_preview = None;
                        }
                    }
                }
            }
        }
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
        self.gizmo_backdrop_prefer_viewed.retain(|id, _| live.contains(id));
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
        // dropped files) â€” see `camera_at`.
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
        // Use `text_edit_focused()` (not `egui_wants_keyboard_input()`, which is
        // true whenever *any* widget has focus, including a just-clicked node) so
        // that selecting a node doesn't suppress the delete key.
        let typing = ctx.text_edit_focused();
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

        self.show_status_message(ui, work_rect, theme);

        self.show_load_warning_banner(ui, work_rect, theme);
        self.show_file_conflict_modal(ui, theme);
    }

    /// Persistent, dismissible load-warning banner (newer-version file /
    /// unknown-node placeholders), top-center of the work area. Unlike the
    /// fading `status_message`, this stays until the user closes it â€” it
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
        // ignored â€” see the doc comment above.

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
    /// events â€” in its own local coordinates, even when the cursor is
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
            && self.dragging_library_image.is_none()
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
        // updates from exactly one window per frame â€” with live coordinates
        // even when the cursor is outside its bounds.
        if primary_down || primary_released {
            if let Some(local) = local_pointer {
                self.menu_drag_pointer_screen = Some(origin + local.to_vec2());
            }
        }

        let Some(pointer_screen) = self.menu_drag_pointer_screen else {
            return;
        };

        // release mouse button after dragging menu button â€” delivered to the
        // capturing window only; the drop target may be any window's panel.
        if primary_released {
            let target = self
                .graph_rects_screen
                .iter()
                .find(|(_, (screen_rect, _))| screen_rect.contains(pointer_screen))
                .map(|(leaf, (_, target_origin))| (*leaf, *target_origin));
            if let Some((leaf, target_origin)) = target {
                // Graph-space position from the target window's local coords
                // and the target panel's camera.
                let local = pointer_screen - target_origin.to_vec2();
                let (zoom, position) = self.camera_transform(Some(leaf));
                let graph_pos = view_to_graph_space_pos2(zoom, local) - position.to_vec2();
                //let node_position_view_space = Pos2::new(cursor_position.x - bottom_panel_rect.min.x, cursor_position.y - bottom_panel_rect.min.y);
                if let Some(path) = self.dragging_library_image.take() {
                    // Dropped a Libraries image: create the "image from file"
                    // node here, already wired to the dropped path.
                    self.add_image_from_file_at(path, graph_pos);
                } else {
                    let node_type = if let Some(operation) =
                        &self.dragging_menu_button.operation_being_created
                    {
                        AddNodeType::Operation(operation.clone())
                    } else {
                        AddNodeType::Subgraph
                    };
                    if let Ok(node_id) = self.add_node(node_type, graph_pos, true, None, Vec::new())
                    {
                        self.edit_node(node_id);
                    }
                }
            }

            self.dragging_menu_button = MenuItemsResult::default();
            self.dragging_library_image = None;
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
        } else if self.dragging_library_image.is_some() {
            name = "image".to_string();
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
    fn show_status_message(&mut self, ui: &mut egui::Ui, work_rect: Rect, theme: &Theme) {
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
                // Derive the base color from the theme (same "strong" readable
                // text color used elsewhere, e.g. graph node headers) instead of
                // hardcoding white, which was illegible on light themes. Only
                // the alpha channel fades over time, so the message keeps the
                // theme's text color throughout instead of darkening as it
                // fades toward transparent.
                let base = theme.get().override_text_color;
                let color = egui::Color32::from_rgba_unmultiplied(base.r(), base.g(), base.b(), alpha);
                ui.painter().text(
                    pos,
                    egui::Align2::CENTER_BOTTOM,
                    msg,
                    egui::FontId::proportional(14.0),
                    color,
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
        // Explicitly picking an output beats the gizmo overlay's automatic
        // choice of backdrop, in every panel â€” the automatic one re-arms when
        // the user selects a different node (see `edit_node`).
        for prefer in self.gizmo_backdrop_prefer_viewed.values_mut() {
            *prefer = true;
        }
        // A node output replaces any library image being previewed (last
        // action wins), so the 2D panel shows what the user just picked. The
        // generation bump also disowns a decode still in flight, which would
        // otherwise land later and take the panel back.
        self.library_image_preview = None;
        self.pending_library_preview = None;
        self.library_preview_generation += 1;
        // Explicitly viewing an output always re-frames it, even if it was
        // already showing (the user may have panned/zoomed it out of view).
        self.view_fit_seq += 1;
        if !self.has_preview_2d_panel {
            self.status_message = Some((
                "no 2D preview panel open â€” use a panel's corner menu to add one".to_string(),
                std::time::Instant::now(),
            ));
        }
        //self.needs_to_save = true;
    }

    /// Requests `path` be loaded off the graph and shown in the 2D preview
    /// panel. Takes precedence over any node output being viewed (`view_node`
    /// clears this in the other direction).
    ///
    /// Returns immediately: the decode runs on a plain `std::thread` (same
    /// choice as `library_scanner` â€” no coupling to the tokio runtime) and the
    /// result is picked up by [`Self::poll_library_preview`]. Decode failures
    /// surface through [`Self::take_library_preview_error`] rather than a
    /// return value, since they aren't known yet when this returns.
    pub fn preview_library_image(&mut self, path: PathBuf, ctx: &egui::Context) {
        self.library_preview_generation += 1;
        let generation = self.library_preview_generation;

        // Drop what's on screen now: the placeholder should replace the old
        // image immediately, not leave the previous click's picture up while a
        // different file loads.
        self.library_image_preview = None;

        let slot: Arc<Mutex<Option<Result<FloatImage, String>>>> = Arc::new(Mutex::new(None));
        let thread_slot = Arc::clone(&slot);
        let thread_path = path.clone();
        let thread_ctx = ctx.clone();
        std::thread::spawn(move || {
            let result =
                mangler_core::operations::images::inputs::file::load_image_from_path(&thread_path);
            *thread_slot.lock().unwrap() = Some(result);
            // Wake the UI: an idle app repaints on demand, so without this the
            // result could sit in the slot until the next unrelated event.
            thread_ctx.request_repaint();
        });

        self.pending_library_preview = Some(PendingLibraryPreview {
            path,
            generation,
            slot,
        });

        if !self.has_preview_2d_panel {
            self.status_message = Some((
                "no 2D preview panel open â€” use a panel's corner menu to add one".to_string(),
                std::time::Instant::now(),
            ));
        }
    }

    /// Promotes a finished background decode into the shown preview. Cheap and
    /// idempotent, so it's safe to call once per frame and again per 2D panel.
    ///
    /// A result whose generation is stale is dropped without touching the
    /// panel; a failure is stashed for `App` to surface.
    fn poll_library_preview(&mut self) {
        let Some(pending) = self.pending_library_preview.as_ref() else {
            return;
        };
        match poll_preview_slot(
            &pending.slot,
            pending.generation,
            self.library_preview_generation,
        ) {
            PreviewPoll::Pending => return,
            PreviewPoll::Ready(Ok(image)) => {
                let path = pending.path.clone();
                self.library_image_preview = Some(LibraryImagePreview {
                    path,
                    image: Arc::new(image),
                });
                // Fit when the image actually appears â€” fitting at click time
                // would frame whatever the panel was showing before.
                self.view_fit_seq += 1;
            }
            PreviewPoll::Ready(Err(err)) => {
                self.library_preview_error = Some(format!(
                    "couldn't preview '{}': {}",
                    pending.path.display(),
                    err
                ));
            }
            PreviewPoll::Stale => {}
        }
        self.pending_library_preview = None;
    }

    /// Takes the last background decode failure, if any, so `App` can show it
    /// on the Libraries panel's error line.
    pub fn take_library_preview_error(&mut self) -> Option<String> {
        self.library_preview_error.take()
    }

    /// The library image shown in (or on its way to) the 2D preview, if any.
    /// Used by the Libraries panel to highlight the matching row â€” a file being
    /// decoded counts, so the row stays lit for the whole wait.
    pub fn previewed_library_image(&self) -> Option<&Path> {
        self.library_image_preview
            .as_ref()
            .map(|p| p.path.as_path())
            .or_else(|| {
                self.pending_library_preview
                    .as_ref()
                    .map(|p| p.path.as_path())
            })
    }

    /// Binds all of the 3D preview panels' material channels from a material
    /// export node's input connections (right-click on the node in the graph).
    ///
    /// There's no "focused panel" concept for the 3D viewers â€” the default
    /// layout has a single 3D panel anyway â€” so this deliberately applies the
    /// binding to every open 3D panel rather than picking one.
    ///
    /// Purely a GUI-side state change: no engine messages are sent. The
    /// channels are resolved from live node data next frame by the existing
    /// `resolve_material` (called from the 3D panel's own show code).
    fn bind_material_node_to_3d(&mut self, node_id: &str) {
        let Some(node) = self.graph_editor.graph_nodes.get(node_id) else {
            // Node vanished (e.g. deleted the same frame) â€” nothing to bind.
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
                "no 3D preview panel open â€” use a panel's corner menu to add one".to_string(),
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
        // Selecting a different node re-arms the automatic gizmo backdrop: the
        // override means "I want to keep looking at what I picked for *this*
        // node", not "never auto-switch again".
        if self.editing_node_id.as_deref() != Some(node_id.as_str()) {
            self.gizmo_backdrop_prefer_viewed.clear();
        }
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
            // applies them before echoing the node back â€” the local node is
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
/// Passing `None` for `thumbnail` produces `Text("None")` â€” the UI's
/// equivalent of "no thumbnail data" â€” which mirrors the pre-async
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
            // A curve rasterizes to a small mask preview; show it as a caption-
            // less texture swatch, reusing the `Color` variant's upload path.
            Value::Curve(_) => {
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

/// What the 2D preview is editing on behalf of the settings-panel node.
///
/// At most one is active. The curve overlay's empty-space catcher covers the
/// whole panel and would swallow gizmo clicks, so making the two mutually
/// exclusive *here* is what stops a node that has both (`directional blur` has
/// a spatial `path` curve as well as an angle) from getting a half-working
/// gizmo. Curve wins: if a node offers a path to draw, that is what the user
/// came to the preview for.
pub enum PreviewEditor {
    Curve { node_id: String, input_index: usize, curve: mangler_core::curve::Curve },
    Gizmos { node_id: String, specs: &'static [mangler_core::gizmo::GizmoSpec] },
}

/// Pick the overlay editor for the node currently in the settings panel.
///
/// Curve editing stays *rule-based* (any unconnected, non-tone `Value::Curve`
/// input) rather than being listed in the gizmo table: roughly two dozen
/// operations across `curves/`, `shapes/` and the simulations take a spatial
/// curve, and enumerating them would mean silently losing an editor for every
/// one that was missed.
pub fn resolve_preview_editor(
    nodes: &HashMap<String, GraphNode>,
    editing_node_id: Option<&str>,
) -> Option<PreviewEditor> {
    let node = nodes.get(editing_node_id?)?;

    // Tone-curve inputs are excluded — they map values, not space, and are
    // edited in the node settings panel's embedded box instead.
    let curve_input = node.inputs.iter().position(|inp| {
        matches!(inp.value, Value::Curve(_))
            && inp.connection.is_none()
            && !matches!(inp.settings, Some(mangler_core::input::InputSettings::ToneCurve))
    });
    if let Some(idx) = curve_input {
        if let Value::Curve(curve) = &node.inputs[idx].value {
            return Some(PreviewEditor::Curve {
                node_id: node.id.clone(),
                input_index: idx,
                curve: curve.clone(),
            });
        }
    }

    // `NodeType::Unknown` placeholders and subgraphs carry no operation, so
    // they never reach the gizmo table.
    let AddNodeType::Operation(op) = node.node_type.as_ref()? else {
        return None;
    };
    let specs = mangler_core::gizmo::gizmos(op);
    (!specs.is_empty()).then(|| PreviewEditor::Gizmos { node_id: node.id.clone(), specs })
}

/// The image a node's spatial inputs are expressed against, as
/// `(node id, output index)`.
///
/// A dichotomy, deliberately not a fallback chain:
/// - An operation that **consumes** an image (crop, sample pixel) works in its
///   *source* image's space, resolved upstream through the first connected
///   image input.
/// - An operation that **produces** one from nothing (line, circle, text) works
///   in its own output's space.
///
/// A consumer with nothing connected returns `None` rather than falling back to
/// its own output: that output is the 1×1 white `default_image()` placeholder,
/// and a fallback chain would cheerfully draw a crop box on it.
pub fn gizmo_backdrop_source(
    nodes: &HashMap<String, GraphNode>,
    node_id: &str,
) -> Option<(String, usize)> {
    let node = nodes.get(node_id)?;
    let consumes_image = node.inputs.iter().any(|i| matches!(i.value, Value::Image { .. }));

    if consumes_image {
        let (upstream_id, upstream_index) = node
            .inputs
            .iter()
            .find(|i| matches!(i.value, Value::Image { .. }) && i.connection.is_some())
            .and_then(|i| i.connection.clone())?;
        // Verify the far end really produces an image before naming it.
        image_output(nodes, &upstream_id, upstream_index)?;
        Some((upstream_id, upstream_index))
    } else {
        let index = node.outputs.iter().position(|o| matches!(o.value, Value::Image { .. }))?;
        Some((node_id.to_owned(), index))
    }
}

/// The image at `(node id, output index)`, with its change id, when that output
/// holds one.
fn image_output(
    nodes: &HashMap<String, GraphNode>,
    node_id: &str,
    output_index: usize,
) -> Option<(std::sync::Arc<mangler_core::float_image::FloatImage>, String)> {
    match &nodes.get(node_id)?.outputs.get(output_index)?.value {
        Value::Image { data, change_id } => Some((data.clone(), change_id.clone())),
        _ => None,
    }
}

/// `"node name ▸ output name"`, for the gizmo caption strip.
fn describe_output(
    nodes: &HashMap<String, GraphNode>,
    node_id: &str,
    output_index: usize,
) -> String {
    let Some(node) = nodes.get(node_id) else {
        return node_id.to_owned();
    };
    let name = node.custom_name.clone().unwrap_or_else(|| node.settings.name.clone());
    match node.outputs.get(output_index) {
        Some(output) => format!("{name} ▸ {}", output.name),
        None => name,
    }
}

/// The placeholder for a panel with an active editor but nothing to draw over.
fn show_editor_hint(ui: &mut egui::Ui, editor: &Option<PreviewEditor>, theme: &Theme) {
    match editor {
        Some(PreviewEditor::Curve { .. }) => preview_2d::show_curve_hint(ui, theme),
        Some(PreviewEditor::Gizmos { .. }) => preview_2d::show_gizmo_hint(ui, theme),
        None => preview_2d::show_empty(ui, theme),
    }
}

#[cfg(test)]
#[path = "program_tests.rs"]
mod tests;
