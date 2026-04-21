use std::{path::PathBuf, time::Duration};
use tokio::{sync::mpsc, time::Instant, task::JoinHandle};
use crate::{ChangeGraphMessage, ChangeNodeMessage, NodeChangedMessage, GraphChangedMessage, graph::Graph, get_id};
use crate::node_type::NodeType;

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
                    loop {
                        let mut sleep_time = Instant::now() + Duration::from_millis(16);

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
                                    // For manual-run nodes, mark dirty but don't clear the
                                    // cached input hash — the node won't auto-execute.
                                    // Send DirtyChanged so the UI shows the dirty indicator.
                                    let is_manual = graph.nodes.get(&node_id).is_some_and(|n| {
                                        matches!(&n.node_type, NodeType::Operation { operation } if operation.requires_manual_run())
                                    });
                                    graph.set_input(node_id.clone(), input_index, value);
                                    if is_manual {
                                        // Restore cached_input_hash so graph.run() skips this node
                                        // (set_input clears it, but we want manual-run nodes to wait).
                                        if let Some(node) = graph.nodes.get_mut(&node_id) {
                                            // Keep dirty flag but don't let it auto-execute:
                                            // we rely on manual_run_requested instead.
                                            node.is_dirty = false;
                                        }
                                        if let Some(tx) = &graph.tx_node_changed {
                                            let _ = tx.try_send(NodeChangedMessage::DirtyChanged {
                                                node_id,
                                                is_dirty: true,
                                            });
                                        }
                                    }
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
                                ChangeNodeMessage::ManualRun { node_id } => {
                                    if let Some(node) = graph.nodes.get_mut(&node_id) {
                                        node.manual_run_requested = true;
                                        node.is_dirty = true;
                                        node.cached_input_hash = None;
                                        // Send status log: starting
                                        if let Some(tx) = &graph.tx_node_changed {
                                            let _ = tx.try_send(NodeChangedMessage::StatusLog {
                                                node_id: node_id.clone(),
                                                message: "Sending request...".to_string(),
                                            });
                                            let _ = tx.try_send(NodeChangedMessage::DirtyChanged {
                                                node_id,
                                                is_dirty: false,
                                            });
                                        }
                                    }
                                }
                                ChangeNodeMessage::CancelRun { node_id } => {
                                    if let Some(node) = graph.nodes.get_mut(&node_id) {
                                        if node.is_busy {
                                            // Abort the running task if we have a handle
                                            if let Some(handle) = node.abort_handle.take() {
                                                handle.abort();
                                            }
                                            node.is_busy = false;
                                            node.manual_run_requested = false;
                                            if let Some(tx) = &graph.tx_node_changed {
                                                let _ = tx.try_send(NodeChangedMessage::Busy {
                                                    node_id: node_id.clone(),
                                                    is_busy: false,
                                                });
                                                let _ = tx.try_send(NodeChangedMessage::StatusLog {
                                                    node_id: node_id.clone(),
                                                    message: "Cancelled.".to_string(),
                                                });
                                                let _ = tx.try_send(NodeChangedMessage::DirtyChanged {
                                                    node_id,
                                                    is_dirty: true,
                                                });
                                            }
                                        }
                                    }
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