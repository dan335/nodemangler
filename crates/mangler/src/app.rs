use tokio::sync::mpsc;

use crate::{ChangeGraphMessage, ChangeNodeMessage, NodeChangedMessage, GraphChangedMessage, graph::Graph};

pub struct App {
    rx_change_graph: mpsc::Receiver<ChangeGraphMessage>,
    rx_change_node: mpsc::Receiver<ChangeNodeMessage>,
    tx_node_changed: mpsc::Sender<NodeChangedMessage>,
    tx_graph_changed: mpsc::Sender<GraphChangedMessage>,
    graph: Graph,
}

impl App {
    pub fn new(
        rx_change_graph: mpsc::Receiver<ChangeGraphMessage>,
        rx_change_node: mpsc::Receiver<ChangeNodeMessage>,
        tx_node_changed: mpsc::Sender<NodeChangedMessage>,
        tx_graph_changed: mpsc::Sender<GraphChangedMessage>
    ) -> Self {
        Self {
            rx_change_graph: mpsc::Receiver<ChangeGraphMessage>,
            rx_change_node: mpsc::Receiver<ChangeNodeMessage>,
            tx_node_changed: mpsc::Sender<NodeChangedMessage>,
            tx_graph_changed: mpsc::Sender<GraphChangedMessage>,
            graph: todo!(),
        }
    }

    pub fn load(
        rx_change_graph: mpsc::Receiver<ChangeGraphMessage>,
        rx_change_node: mpsc::Receiver<ChangeNodeMessage>,
        tx_node_changed: mpsc::Sender<NodeChangedMessage>,
        tx_graph_changed: mpsc::Sender<GraphChangedMessage>
    ) -> Self {
        Self {
            rx_change_graph: mpsc::Receiver<ChangeGraphMessage>,
            rx_change_node: mpsc::Receiver<ChangeNodeMessage>,
            tx_node_changed: mpsc::Sender<NodeChangedMessage>,
            tx_graph_changed: mpsc::Sender<GraphChangedMessage>,
            graph: todo!(),
        }
    }

    pub async fn run(&mut self) {
        while let Ok(change_graph_message) = self.rx_change_graph.try_recv() {
            match change_graph_message {
                ChangeGraphMessage::AddNode {
                    node_id,
                    node_type,
                    position,
                } => {
                    self.graph.add_node(node_id, node_type, position).await;
                }
                ChangeGraphMessage::RemoveNode { node_id } => {
                    self.graph.remove_node(node_id).await;
                }
                ChangeGraphMessage::AddConnection {
                    input_node_id,
                    input_connection_index,
                    output_node_id,
                    output_connection_index,
                } => {
                    self.graph
                        .add_connection(
                            input_node_id,
                            input_connection_index,
                            output_node_id,
                            output_connection_index,
                        )
                        .await;
                }
                ChangeGraphMessage::RemoveConnection {
                    node_id,
                    input_index,
                } => {
                    self.graph.remove_connection(node_id, input_index).await;
                }
                ChangeGraphMessage::SetSavePath(save_path) => {
                    self.graph.set_save_path(save_path);
                }
                ChangeGraphMessage::SetGraphName(graph_name) => {
                    self.graph.name = graph_name;
                    self.graph.save_to_file();
                }
            }
        }

        while let Ok(change_node_message) = self.rx_change_node.try_recv() {
            match change_node_message {
                ChangeNodeMessage::SetInput {
                    node_id,
                    input_index,
                    value,
                } => {
                    self.graph.set_input(node_id, input_index, value);
                }
                ChangeNodeMessage::SetPosition {
                    node_id,
                    position
                } => {
                    self.graph.set_node_position(
                        node_id,
                        position,
                    );
                }
                ChangeNodeMessage::SetExposeInput {
                    node_id,
                    input_index,
                    set_to,
                } => {
                    if let Some(node) = self.graph.nodes.get_mut(&node_id) {
                        if let Some(input) = node.inputs.get_mut(input_index) {
                            input.is_exposed = set_to;
                            self.graph.save_to_file();
                        }
                    }
                }
                ChangeNodeMessage::SetExposeOutput {
                    node_id,
                    output_index,
                    set_to,
                } => {
                    if let Some(node) = self.graph.nodes.get_mut(&node_id) {
                        if let Some(output) = node.outputs.get_mut(output_index) {
                            output.is_exposed = set_to;
                            self.graph.save_to_file();
                        }
                    }
                }
            }
        }

        // while let Ok(node_input_message) = rx_set_input.try_recv() {
        //     graph.set_input(
        //         node_input_message.node_id,
        //         node_input_message.input_index,
        //         node_input_message.value,
        //     );
        // }

        // while let Ok(node_position_message) = rx_node_position.try_recv() {
        //     graph.set_node_position(
        //         node_position_message.node_id,
        //         node_position_message.position,
        //     );
        // }

        self.graph.run().await;
    }
}