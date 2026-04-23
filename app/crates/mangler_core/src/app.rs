use std::{path::PathBuf, time::Duration};
use tokio::{sync::mpsc, time::Instant, task::JoinHandle};
use crate::{ChangeGraphMessage, ChangeNodeMessage, NodeChangedMessage, GraphChangedMessage, graph::Graph, get_id};

/// Engine-side application wrapper. Owns a `Graph` and runs it on a dedicated
/// tokio task, continuously draining UI change messages and re-executing dirty
/// nodes each tick (~60 Hz target, 2 ms minimum between ticks).
pub struct App {
    pub id: String,
    pub name: String,
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
                let name = graph.name.clone();
                let save_path = graph.save_path.clone();
                let mut needs_to_save = false;

                // Main engine loop: drain messages, execute graph, auto-save
                let thread_handle = tokio::spawn(async move {
                    // Optional background render task. The engine keeps running
                    // interactively while this runs; we only track it to reject
                    // a second concurrent StartRender.
                    #[cfg(feature = "video")]
                    let mut render_task: Option<JoinHandle<()>> = None;
                    loop {
                        let mut sleep_time = Instant::now() + Duration::from_millis(16);

                        // Detect cross-tab / external edits to any referenced
                        // subgraph files and reload them before we do anything
                        // else this tick. One stat() per subgraph node — cheap.
                        graph.check_subgraphs_for_changes();

                        // Process graph-level changes (add/remove nodes, connections, save path)
                        while let Ok(change_graph_message) = rx_change_graph.try_recv() {
                            match change_graph_message {
                                ChangeGraphMessage::AddNode {
                                    node_id,
                                    node_type,
                                    position,
                                    is_enabled,
                                    custom_name,
                                } => {
                                    graph.add_node(node_id, node_type, position, is_enabled, custom_name).await;
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
                                    graph.set_save_path(save_path);
                                    needs_to_save = true;
                                }
                                ChangeGraphMessage::SetGraphName(graph_name) => {
                                    graph.name = graph_name;
                                    needs_to_save = true;
                                }
                                ChangeGraphMessage::StartRender { output_node_id } => {
                                    start_render_on_engine(
                                        &graph,
                                        output_node_id,
                                        #[cfg(feature = "video")]
                                        &mut render_task,
                                    );
                                }
                            }
                        }

                        // Process node-level changes (input values, positions, expose toggles)
                        while let Ok(change_node_message) = rx_change_node.try_recv() {
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

                        // Execute any dirty nodes in the graph
                        graph.run().await;

                        // Auto-save after any mutation
                        if needs_to_save {
                            graph.save_to_file();
                        }

                        // Sleep until next tick, minimum 2 ms to avoid busy-spinning
                        sleep_time = sleep_time.max(Instant::now() + Duration::from_millis(2));
                        tokio::time::sleep_until(sleep_time).await;
                    }


                    
                });

                Ok(App {
                    thread_handle,
                    id,
                    name,
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


/// Error returned when graph creation or loading fails during `App::new`.
#[derive(Debug)]
pub struct NewAppError(pub String);

/// Handle a `StartRender` message: spawn a detached render task if none is
/// active, otherwise emit `RenderFailed` with "already in progress".
///
/// When the `video` feature is disabled, all render attempts fail fast so
/// the user gets a clear message instead of silence.
fn start_render_on_engine(
    graph: &crate::graph::Graph,
    output_node_id: String,
    #[cfg(feature = "video")] render_task: &mut Option<JoinHandle<()>>,
) {
    let _ = output_node_id; // suppress unused-var when the feature is off

    #[cfg(feature = "video")]
    {
        // If a render is still active, don't start a second one.
        let busy = render_task
            .as_ref()
            .map(|h| !h.is_finished())
            .unwrap_or(false);
        if busy {
            if let Some(tx) = &graph.tx_graph_changed {
                let _ = tx.try_send(GraphChangedMessage::RenderFailed {
                    message: "render already in progress".to_string(),
                });
            }
            return;
        }

        let Some(tx_graph_changed) = graph.tx_graph_changed.clone() else {
            return;
        };
        let snapshot = graph.detached();
        *render_task = Some(tokio::spawn(crate::render::run_render(
            snapshot,
            output_node_id,
            tx_graph_changed,
        )));
    }

    #[cfg(not(feature = "video"))]
    {
        if let Some(tx) = &graph.tx_graph_changed {
            let _ = tx.try_send(GraphChangedMessage::RenderFailed {
                message: "Video support is not enabled in this build (rebuild with --features video)."
                    .to_string(),
            });
        }
    }
}