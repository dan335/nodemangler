use std::{path::PathBuf, time::Duration};
use tokio::{sync::mpsc, time::Instant, task::JoinHandle};
use crate::{ChangeGraphMessage, ChangeNodeMessage, NodeChangedMessage, GraphChangedMessage, graph::Graph, get_id};

pub struct App {
    pub id: String,
    pub name: String,
    pub save_path: Option<PathBuf>,
    pub thread_handle: JoinHandle<()>,
}

impl App {
    pub fn new(
        id: Option<String>,
        save_file: Option<PathBuf>,
        mut rx_change_graph: mpsc::Receiver<ChangeGraphMessage>,
        mut rx_change_node: mpsc::Receiver<ChangeNodeMessage>,
        tx_node_changed: mpsc::Sender<NodeChangedMessage>,
        tx_graph_changed: mpsc::Sender<GraphChangedMessage>
    ) -> Result<Self, NewAppError> {

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

                let thread_handle = tokio::spawn(async move {
                    loop {
                        let mut sleep_time = Instant::now() + Duration::from_millis(16);

                        while let Ok(change_graph_message) = rx_change_graph.try_recv() {
                            match change_graph_message {
                                ChangeGraphMessage::AddNode {
                                    node_id,
                                    node_type,
                                    position,
                                } => {
                                    graph.add_node(node_id, node_type, position).await;
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
                            }
                        }

                        graph.run().await;

                        if needs_to_save {
                            graph.save_to_file();
                        }

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


#[derive(Debug)]
pub struct NewAppError(pub String);