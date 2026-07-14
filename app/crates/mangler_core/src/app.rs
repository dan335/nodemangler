use std::{path::PathBuf, time::Duration};
use tokio::{sync::mpsc, time::Instant, task::JoinHandle};
use crate::{ChangeGraphMessage, ChangeNodeMessage, NodeChangedMessage, GraphChangedMessage, graph::Graph, get_id};
use crate::node_type::NodeType;
use crate::operations::images::inputs::from_folder;
use crate::operations::Operation;
use crate::value::Value;

/// Engine-side application wrapper. Owns a `Graph` and runs it on a dedicated
/// tokio task, continuously draining UI change messages and re-executing dirty
/// nodes each tick (~60 Hz target, 2 ms minimum between ticks).
pub struct App {
    pub id: String,
    pub save_path: Option<PathBuf>,
    pub thread_handle: JoinHandle<()>,
}

impl App {
    /// Creates a new engine instance. Loads an existing graph from `save_file`
    /// if provided, otherwise creates a fresh empty graph. Spawns the
    /// async run loop that processes incoming messages and executes the graph.
    pub fn new(
        id: Option<String>,
        save_file: Option<PathBuf>,
        mut rx_change_graph: mpsc::Receiver<ChangeGraphMessage>,
        mut rx_change_node: mpsc::Receiver<ChangeNodeMessage>,
        tx_node_changed: mpsc::Sender<NodeChangedMessage>,
        tx_graph_changed: mpsc::Sender<GraphChangedMessage>
    ) -> Result<Self, NewAppError> {

        // Load from file or create a new graph
        let graph_result = match save_file {
            Some(path) => Graph::load(path, Some(tx_node_changed), Some(tx_graph_changed), false),
            None => {
                let graph_id = match id {
                    Some(graph_id) => graph_id,
                    None => get_id(),
                };

                Graph::new(graph_id, tx_node_changed, tx_graph_changed, false)
            }
        };

        match graph_result {
            Ok(mut graph) => {
                let id = graph.id.clone();
                let save_path = graph.save_path.clone();
                // Auto-save debounce state. `needs_to_save` flips true on any
                // mutation this tick; `last_save` is when we last wrote to disk.
                let mut needs_to_save = false;
                let mut last_save = Instant::now();
                const AUTO_SAVE_INTERVAL: Duration = Duration::from_secs(1);
                // External subgraph edits are rare (seconds-to-minutes apart);
                // stat()-ing every subgraph file at 60 Hz is wasted blocking
                // syscall traffic on the engine task. Poll at 500 ms instead.
                let mut last_subgraph_check = Instant::now();
                const SUBGRAPH_CHECK_INTERVAL: Duration = Duration::from_millis(500);
                // A file loaded from a *newer* NodeMangler must not be
                // silently downgraded by the next auto-save before the user
                // has even looked at it. Seeded from the load itself; any
                // subsequent edit (see the two message-drain loops below)
                // releases the hold, since at that point re-saving is an
                // intentional user action, not an unattended background
                // write. `graph.load_report` is `None` for a brand-new graph
                // (nothing to hold).
                let mut hold_saves = graph
                    .load_report
                    .as_ref()
                    .is_some_and(|r| r.is_newer_than_app);
                // Set once an external modification to the save file is
                // detected mid-edit (see the auto-save block below). Guards
                // against re-sending `FileConflict` every tick while the user
                // decides; cleared when `ResolveFileConflict` is handled.
                let mut conflict_pending = false;
                // Active batch run, if any (see `ChangeGraphMessage::RunBatch`
                // and `BatchState`). One iteration is armed per tick — before
                // the tick's `graph.run()` — and completed right after it, so
                // the loop keeps draining messages (cancel, live edits)
                // between iterations instead of blocking for the whole batch.
                let mut batch: Option<BatchState> = None;

                // Main engine loop: drain messages, execute graph, auto-save
                let thread_handle = tokio::spawn(async move {
                    loop {
                        let mut sleep_time = Instant::now() + Duration::from_millis(16);

                        // Detect cross-tab / external edits to any referenced
                        // subgraph files and reload them. Throttled: one
                        // stat() per subgraph node per SUBGRAPH_CHECK_INTERVAL.
                        if last_subgraph_check.elapsed() >= SUBGRAPH_CHECK_INTERVAL {
                            graph.check_subgraphs_for_changes();
                            last_subgraph_check = Instant::now();
                        }

                        // Process graph-level changes (add/remove nodes, connections, save path)
                        while let Ok(change_graph_message) = rx_change_graph.try_recv() {
                            // Any graph-structure message means the user (or
                            // a paste/duplicate/auto-layout action) touched
                            // the graph — release the "newer file" auto-save
                            // hold so subsequent saves proceed normally.
                            // `ResolveFileConflict` re-derives its own value
                            // for `hold_saves` below when it reloads, so
                            // clearing it here first is harmless.
                            hold_saves = false;
                            match change_graph_message {
                                ChangeGraphMessage::AddNode {
                                    node_id,
                                    node_type,
                                    position,
                                    is_enabled,
                                    custom_name,
                                    input_values,
                                } => {
                                    graph.add_node(node_id, node_type, position, is_enabled, custom_name, input_values).await;
                                    needs_to_save = true;
                                }
                                ChangeGraphMessage::RemoveNode { node_id } => {
                                    graph.remove_node(node_id).await;
                                    needs_to_save = true;
                                }
                                ChangeGraphMessage::AddConnection {
                                    input_node_id,
                                    input_connection_index,
                                    output_node_id,
                                    output_connection_index,
                                } => {
                                    graph
                                        .add_connection(
                                            input_node_id,
                                            input_connection_index,
                                            output_node_id,
                                            output_connection_index,
                                        )
                                        .await;
                                    needs_to_save = true;
                                }
                                ChangeGraphMessage::RemoveConnection {
                                    node_id,
                                    input_index,
                                } => {
                                    graph.remove_connection(node_id, input_index).await;
                                    needs_to_save = true;
                                }
                                ChangeGraphMessage::SetSavePath(save_path) => {
                                    // Like RenameFile below: never re-target
                                    // the save path while a conflict is
                                    // unresolved.
                                    if conflict_pending {
                                        if let Some(tx) = &graph.tx_graph_changed {
                                            if let Err(err) = tx.try_send(GraphChangedMessage::SaveError {
                                                path: save_path,
                                                message: "resolve the file conflict first".to_string(),
                                            }) {
                                                println!("Error sending SaveError: {:?}", err);
                                            }
                                        }
                                    } else {
                                        graph.set_save_path(save_path.clone());
                                        // Save synchronously rather than via the
                                        // debounced auto-save: the GUI's close
                                        // flow ("save then close this unsaved
                                        // tab") aborts this task right after,
                                        // which would race a deferred write.
                                        // Deliberately not gated on
                                        // disk_conflicts(): the target was chosen
                                        // by the user through a save dialog
                                        // (which already confirms overwrites).
                                        match graph.save_to_file() {
                                            Ok(()) => {
                                                if let Some(tx) = &graph.tx_graph_changed {
                                                    if let Err(err) = tx.try_send(GraphChangedMessage::SavedTo {
                                                        path: save_path,
                                                    }) {
                                                        println!("Error sending SavedTo: {:?}", err);
                                                    }
                                                }
                                                last_save = Instant::now();
                                                needs_to_save = false;
                                            }
                                            Err(message) => {
                                                if let Some(tx) = &graph.tx_graph_changed {
                                                    if let Err(err) = tx.try_send(GraphChangedMessage::SaveError {
                                                        path: save_path,
                                                        message,
                                                    }) {
                                                        println!("Error sending SaveError: {:?}", err);
                                                    }
                                                }
                                                // Let the debounced loop retry.
                                                needs_to_save = true;
                                            }
                                        }
                                    }
                                }
                                ChangeGraphMessage::RenameFile { new_stem } => {
                                    // Never rename out from under an unresolved
                                    // conflict: the file on disk differs from
                                    // what we think it is, so moving it would
                                    // muddy the resolution. Ask the user to
                                    // settle the conflict first.
                                    if conflict_pending {
                                        if let Some(tx) = &graph.tx_graph_changed {
                                            let path = graph.save_path.clone().unwrap_or_default();
                                            if let Err(err) = tx.try_send(GraphChangedMessage::SaveError {
                                                path,
                                                message: "resolve the file conflict first".to_string(),
                                            }) {
                                                println!("Error sending SaveError: {:?}", err);
                                            }
                                        }
                                    } else {
                                        match graph.rename_file(&new_stem) {
                                            Ok(new_path) => {
                                                if let Some(tx) = &graph.tx_graph_changed {
                                                    if let Err(err) = tx.try_send(GraphChangedMessage::FileRenamed {
                                                        new_path,
                                                    }) {
                                                        println!("Error sending FileRenamed: {:?}", err);
                                                    }
                                                }
                                                // Persist the write-only mirror
                                                // `name` into the newly-named
                                                // file. rename_file already
                                                // re-stat'd last_synced_mtime
                                                // from the new path, so this
                                                // save can't trip a spurious
                                                // conflict.
                                                needs_to_save = true;
                                            }
                                            Err(message) => {
                                                if let Some(tx) = &graph.tx_graph_changed {
                                                    let path = graph.save_path.clone().unwrap_or_default();
                                                    if let Err(err) = tx.try_send(GraphChangedMessage::SaveError { path, message }) {
                                                        println!("Error sending SaveError: {:?}", err);
                                                    }
                                                }
                                            }
                                        }
                                    }
                                }
                                ChangeGraphMessage::ResolveFileConflict { keep_ours } => {
                                    // A resolution action, not an edit in its
                                    // own right — it must not set
                                    // `needs_to_save` (that would immediately
                                    // re-trigger the very conflict check
                                    // we're in the middle of resolving).
                                    if keep_ours {
                                        // Overwrite: write our in-memory graph.
                                        // save_to_file() refreshes
                                        // last_synced_mtime, so the next
                                        // disk_conflicts check has a fresh
                                        // baseline.
                                        if let Err(message) = graph.save_to_file() {
                                            if let Some(tx) = &graph.tx_graph_changed {
                                                let path = graph.save_path.clone().unwrap_or_default();
                                                if let Err(err) = tx.try_send(GraphChangedMessage::SaveError { path, message }) {
                                                    println!("Error sending SaveError: {:?}", err);
                                                }
                                            }
                                        }
                                    } else {
                                        // Reload: discard local edits and take
                                        // the disk copy. Tell the UI to wipe
                                        // its node list first — the
                                        // LoadedNode stream that follows
                                        // assumes a clean slate.
                                        if let Some(tx) = &graph.tx_graph_changed {
                                            if let Err(err) = tx.try_send(GraphChangedMessage::GraphCleared) {
                                                println!("Error sending GraphCleared: {:?}", err);
                                            }
                                        }
                                        if let Some(reload_path) = graph.save_path.clone() {
                                            match Graph::load(
                                                reload_path,
                                                graph.tx_node_changed.clone(),
                                                graph.tx_graph_changed.clone(),
                                                graph.is_subgraph,
                                            ) {
                                                Ok(fresh_graph) => {
                                                    hold_saves = fresh_graph
                                                        .load_report
                                                        .as_ref()
                                                        .is_some_and(|r| r.is_newer_than_app);
                                                    graph = fresh_graph;
                                                }
                                                Err(_) => {
                                                    // The file became unreadable between
                                                    // conflict detection and resolution
                                                    // (e.g. deleted, or mid-write by
                                                    // whoever we're racing). Keep the
                                                    // existing in-memory graph, but
                                                    // re-emit it so the UI — which we
                                                    // just told to clear — resyncs. The
                                                    // conflict re-detects on the next
                                                    // save attempt.
                                                    graph.emit_loaded_nodes();
                                                }
                                            }
                                        }
                                    }
                                    needs_to_save = false;
                                    conflict_pending = false;
                                }
                                ChangeGraphMessage::RunBatch { node_id } => {
                                    // Starting a batch is an action, not an
                                    // edit — it must not set `needs_to_save`
                                    // (the index stepping below is transient
                                    // state that is restored when the batch
                                    // ends). A second RunBatch while one is
                                    // active is ignored rather than queued.
                                    if batch.is_none() {
                                        batch = start_batch(&graph, node_id);
                                    }
                                }
                                ChangeGraphMessage::CancelBatch => {
                                    // Honored between iterations: the arming
                                    // step below runs after this drain, so a
                                    // cancel always lands before the next
                                    // file starts. No-op when idle.
                                    if let Some(state) = batch.take() {
                                        finish_batch(&mut graph, state, true);
                                    }
                                }
                            }
                        }

                        // Process node-level changes (input values, positions, expose toggles)
                        while let Ok(change_node_message) = rx_change_node.try_recv() {
                            // See the identical note in the graph-message
                            // loop above: any node-level edit releases the
                            // "newer file" auto-save hold.
                            hold_saves = false;
                            match change_node_message {
                                ChangeNodeMessage::SetInput {
                                    node_id,
                                    input_index,
                                    value,
                                } => {
                                    graph.set_input(node_id, input_index, value);
                                    needs_to_save = true;
                                }
                                ChangeNodeMessage::SetPosition {
                                    node_id,
                                    position
                                } => {
                                    graph.set_node_position(
                                        node_id,
                                        position,
                                    );
                                    needs_to_save = true;
                                }
                                ChangeNodeMessage::SetExposeInput {
                                    node_id,
                                    input_index,
                                    set_to,
                                } => {
                                    if let Some(node) = graph.nodes.get_mut(&node_id) {
                                        if let Some(input) = node.inputs.get_mut(input_index) {
                                            input.is_exposed = set_to;
                                            needs_to_save = true;
                                            // Echo the confirmed state back to
                                            // the UI so its mirror of the node's
                                            // exposed flags stays in sync.
                                            if let Some(tx) = &graph.tx_node_changed {
                                                let message = NodeChangedMessage::ExposeInputChanged {
                                                    node_id: node_id.clone(),
                                                    input_index,
                                                    set_to,
                                                };
                                                if let Err(err) = tx.try_send(message) {
                                                    println!("Error sending NodeChangedMessage::ExposeInputChanged: {:?}", err);
                                                }
                                            }
                                        }
                                    }
                                }
                                ChangeNodeMessage::SetExposeOutput {
                                    node_id,
                                    output_index,
                                    set_to,
                                } => {
                                    if let Some(node) = graph.nodes.get_mut(&node_id) {
                                        if let Some(output) = node.outputs.get_mut(output_index) {
                                            output.is_exposed = set_to;
                                            needs_to_save = true;
                                            // Echo the confirmed state back to
                                            // the UI so its mirror of the node's
                                            // exposed flags stays in sync.
                                            if let Some(tx) = &graph.tx_node_changed {
                                                let message = NodeChangedMessage::ExposeOutputChanged {
                                                    node_id: node_id.clone(),
                                                    output_index,
                                                    set_to,
                                                };
                                                if let Err(err) = tx.try_send(message) {
                                                    println!("Error sending NodeChangedMessage::ExposeOutputChanged: {:?}", err);
                                                }
                                            }
                                        }
                                    }
                                }
                                ChangeNodeMessage::SetEnabled {
                                    node_id,
                                    set_to,
                                } => {
                                    if let Some(node) = graph.nodes.get_mut(&node_id) {
                                        node.is_enabled = set_to;
                                        node.is_dirty = true;
                                        node.cached_input_hash = None;
                                        needs_to_save = true;
                                    }
                                }
                                ChangeNodeMessage::SetCustomName {
                                    node_id,
                                    name,
                                } => {
                                    if let Some(node) = graph.nodes.get_mut(&node_id) {
                                        node.custom_name = name;
                                        needs_to_save = true;
                                    }
                                }
                                ChangeNodeMessage::SetSubgraphPath { node_id, path } => {
                                    graph.set_subgraph_path(node_id, path);
                                    needs_to_save = true;
                                }
                            }
                        }

                        // ── batch run: arm the next iteration ──────────────
                        // Placed after both message drains so a RunBatch
                        // received this tick starts immediately and a
                        // CancelBatch is honored before another file begins.
                        if batch.as_ref().is_some_and(|b| !graph.nodes.contains_key(&b.node_id)) {
                            // The iterated node was deleted mid-batch. Abort
                            // cleanly (finish_batch skips the index restore
                            // for a missing node).
                            let state = batch.take().expect("batch checked Some above");
                            finish_batch(&mut graph, state, true);
                        }
                        if let Some(state) = &batch {
                            // Point the from-folder node at this iteration's
                            // file and force output saving on for the run
                            // below. `set_input` marks the node dirty and
                            // clears its input hash, so the run can neither
                            // early-out nor hash-skip the iteration.
                            let file = &state.files[state.next];
                            let stem = file.file_stem().map(|s| s.to_string_lossy().to_string()).unwrap_or_default();
                            graph.batch_item_stem = Some(stem);
                            graph.force_save_outputs = true;
                            let value = Value::Integer(state.next as i32);
                            graph.set_input(state.node_id.clone(), from_folder::INDEX, value.clone());
                            // `set_input` itself notifies nobody — echo the
                            // stepped value so the GUI's mirror of the input
                            // stays in sync (the settings panel's index field
                            // visibly counts up during the batch).
                            if let Some(tx) = &graph.tx_node_changed {
                                if let Err(err) = tx.try_send(NodeChangedMessage::InputChanged {
                                    node_id: state.node_id.clone(),
                                    input_index: from_folder::INDEX,
                                    value,
                                }) {
                                    println!("Error sending InputChanged: {:?}", err);
                                }
                            }
                        }

                        // Execute any dirty nodes in the graph
                        graph.run().await;

                        // ── batch run: complete the iteration armed above ──
                        // `graph.run()` only returns once every dirty node has
                        // fully executed, so at this point the current file
                        // has flowed through the whole graph and any output
                        // nodes have written their (force-saved) files.
                        if let Some(state) = &mut batch {
                            state.next += 1;
                            if let Some(tx) = &graph.tx_graph_changed {
                                if let Err(err) = tx.try_send(GraphChangedMessage::BatchProgress {
                                    node_id: state.node_id.clone(),
                                    completed: state.next,
                                    total: state.files.len(),
                                }) {
                                    println!("Error sending BatchProgress: {:?}", err);
                                }
                            }
                            if state.next >= state.files.len() {
                                let state = batch.take().expect("batch matched Some above");
                                finish_batch(&mut graph, state, false);
                            }
                        }

                        // Auto-save policy: debounced to at most one write per
                        // AUTO_SAVE_INTERVAL. When a mutation is pending and the
                        // interval has elapsed since the last write, save and
                        // clear the flag. The flag stays set across ticks until
                        // the save happens, so a burst of edits coalesces into
                        // one write and a continuous stream of messages can
                        // never postpone the pending save for more than one
                        // interval — the final save is never lost.
                        //
                        // `hold_saves` additionally suppresses this entirely
                        // right after loading a newer-version file, until the
                        // user makes an edit (see above). `conflict_pending`
                        // suppresses it once an external modification has
                        // been detected and reported, until
                        // `ResolveFileConflict` clears it.
                        if needs_to_save && !hold_saves && !conflict_pending && last_save.elapsed() >= AUTO_SAVE_INTERVAL {
                            if graph.disk_conflicts() {
                                // Someone else — another tab, another machine
                                // on a network share — has written this file
                                // since we last read/wrote it. Overwriting
                                // now would silently discard their change.
                                // Pause saving and let the user pick a side;
                                // edits keep accumulating in memory in the
                                // meantime (needs_to_save stays true).
                                conflict_pending = true;
                                if let Some(tx) = &graph.tx_graph_changed {
                                    let path = graph.save_path.clone().unwrap_or_default();
                                    if let Err(err) = tx.try_send(GraphChangedMessage::FileConflict { path }) {
                                        println!("Error sending FileConflict: {:?}", err);
                                    }
                                }
                            } else {
                                if let Err(message) = graph.save_to_file() {
                                    if let Some(tx) = &graph.tx_graph_changed {
                                        let path = graph.save_path.clone().unwrap_or_default();
                                        if let Err(err) = tx.try_send(GraphChangedMessage::SaveError { path, message }) {
                                            println!("Error sending SaveError: {:?}", err);
                                        }
                                    }
                                }
                                last_save = Instant::now();
                                needs_to_save = false;
                            }
                        }

                        // Sleep until next tick, minimum 2 ms to avoid busy-spinning
                        sleep_time = sleep_time.max(Instant::now() + Duration::from_millis(2));
                        tokio::time::sleep_until(sleep_time).await;
                    }


                    
                });

                Ok(App {
                    thread_handle,
                    id,
                    save_path,
                })
            },
            Err(error) => Err(NewAppError(format!(
                "Error creating new graph.  Error: {:?}",
                error
            ))),
        }
    }
}


/// State of an active batch run (see [`ChangeGraphMessage::RunBatch`]): the
/// engine loop steps `next` through `files`, one full graph run per tick.
struct BatchState {
    /// The "from folder" node whose `index` input is being stepped.
    node_id: String,
    /// Snapshot of the folder's image files, taken when the batch started
    /// with the same deterministic listing the node's own `run()` uses
    /// ([`from_folder::list_image_files`]), so driver and node agree on both
    /// set and order whenever the folder is stable. If files are added or
    /// removed mid-batch the node re-lists and clamps, so items may repeat or
    /// be skipped — a documented caveat of editing the folder during a run.
    files: Vec<PathBuf>,
    /// Index of the next file to process — also the count completed so far.
    next: usize,
    /// The `index` input's value before the batch started; restored on finish
    /// so the user gets back the image they were previewing.
    original_index: Value,
}

/// Validate a [`ChangeGraphMessage::RunBatch`] request and snapshot its work
/// list. Returns the armed state, or `None` after reporting the failure to
/// the UI as a `BatchFinished { total: 0, cancelled: true }` — the node
/// doesn't exist, isn't a "from folder" node, or its folder is unset,
/// unreadable, or holds no image files.
fn start_batch(graph: &Graph, node_id: String) -> Option<BatchState> {
    // One reporting path for every way the batch can fail to start; the GUI
    // surfaces it as a status message.
    let fail = |graph: &Graph, node_id: String| {
        if let Some(tx) = &graph.tx_graph_changed {
            if let Err(err) = tx.try_send(GraphChangedMessage::BatchFinished {
                node_id,
                completed: 0,
                total: 0,
                cancelled: true,
            }) {
                println!("Error sending BatchFinished: {:?}", err);
            }
        }
        None
    };

    let Some(node) = graph.nodes.get(&node_id) else {
        return fail(graph, node_id);
    };
    if !matches!(node.node_type, NodeType::Operation { operation: Operation::OpImageInputFromFolder }) {
        return fail(graph, node_id);
    }
    // Read the folder straight off the input value (also correct when the
    // input is connection-driven: propagated values land in `input.value`).
    let Some(Value::Path(folder)) = node.inputs.get(from_folder::FOLDER).map(|i| i.value.clone()) else {
        return fail(graph, node_id);
    };
    // Same folder resolution the node's run() applies via its RunContext.
    let graph_dir = graph.save_path.as_ref().and_then(|p| p.parent().map(|d| d.to_path_buf()));
    let Some(dir) = from_folder::resolve_folder(&folder, graph_dir.as_deref()) else {
        return fail(graph, node_id);
    };
    let files = match from_folder::list_image_files(&dir) {
        Ok(files) if !files.is_empty() => files,
        _ => return fail(graph, node_id),
    };
    let original_index = node
        .inputs
        .get(from_folder::INDEX)
        .map(|i| i.value.clone())
        .unwrap_or(Value::Integer(0));

    Some(BatchState { node_id, files, next: 0, original_index })
}

/// Tear down a batch run — completed, cancelled, failed to start, or aborted
/// because the node vanished: switch the per-iteration forced-save flags back
/// off, restore the from-folder node's `index` input to its pre-batch value
/// (via `set_input`, which marks the node dirty — the next tick re-runs the
/// graph on the original image with the save gates off again, so nothing is
/// written by the restore), echo the restored value to the UI, and report the
/// final outcome.
fn finish_batch(graph: &mut Graph, state: BatchState, cancelled: bool) {
    graph.force_save_outputs = false;
    graph.batch_item_stem = None;

    if graph.nodes.contains_key(&state.node_id) {
        graph.set_input(state.node_id.clone(), from_folder::INDEX, state.original_index.clone());
        if let Some(tx) = &graph.tx_node_changed {
            if let Err(err) = tx.try_send(NodeChangedMessage::InputChanged {
                node_id: state.node_id.clone(),
                input_index: from_folder::INDEX,
                value: state.original_index.clone(),
            }) {
                println!("Error sending InputChanged: {:?}", err);
            }
        }
    }

    if let Some(tx) = &graph.tx_graph_changed {
        if let Err(err) = tx.try_send(GraphChangedMessage::BatchFinished {
            node_id: state.node_id,
            completed: state.next,
            total: state.files.len(),
            cancelled,
        }) {
            println!("Error sending BatchFinished: {:?}", err);
        }
    }
}

/// Error returned when graph creation or loading fails during `App::new`.
#[derive(Debug)]
pub struct NewAppError(pub String);

#[cfg(test)]
#[path = "app_tests.rs"]
mod tests;