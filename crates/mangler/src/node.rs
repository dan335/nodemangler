use crate::node_type::NodeType;
use crate::{AddNodeType, NodeChangedMessage};
use glam::f32::Vec2;
use serde::{Deserialize, Serialize};
use std::fmt::Debug;
use std::path::PathBuf;
use tokio::sync::mpsc::Sender;
use tokio::time::Duration;
use crate::{input::Input, output::Output, value::Value};
use super::node_settings::NodeSettings;


#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Node {
    pub id: String,
    pub settings: NodeSettings,
    pub inputs: Vec<Input>,
    pub outputs: Vec<Output>,
    pub time: Option<Duration>,
    pub is_dirty: bool, // node needs to be re-run
    pub position: Vec2,
    pub node_type: NodeType,
    pub is_busy: bool,
    pub is_error: bool,
    pub error_message: Option<String>,
    #[serde(skip)]
    pub cached_input_hash: Option<u64>,
}

impl PartialEq for Node {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id
    }
}

impl Node {
    pub fn new(id: String, node_type: AddNodeType, position: glam::f32::Vec2) -> Self {
        match node_type {            
            AddNodeType::Operation(operation) => Node {
                id,
                settings: operation.settings(),
                inputs: operation.create_inputs(),
                outputs: operation.create_outputs(),
                time: None,
                is_dirty: true,
                position,
                node_type: NodeType::Operation { operation },
                is_busy: false,
                is_error: false,
                error_message: None,
                cached_input_hash: None,
            },
            AddNodeType::Subgraph => Node {
                id,
                settings: NodeSettings {
                    name: "subgraph".to_string(),
                    description: "A subgraph.".to_string(),
                },
                inputs: Vec::new(),
                outputs: Vec::new(),
                time: None,
                is_dirty: true,
                position,
                node_type: NodeType::Subgraph {
                    path: PathBuf::new(),
                    graph: None,
                    rx_node_changed: None,
                },
                is_busy: false,
                is_error: false,
                error_message: None,
                cached_input_hash: None,
            },
        }
    }

    pub fn set_input_value(&mut self, index: usize, value: Value) {
        if let Some(input) = self.inputs.get_mut(index) {
            input.value = value; //value = value;
            self.is_dirty = true;
        } else {
            panic!("Invalid input index: {}", index);
        }
    }

    pub fn get_input(&self, index: usize) -> &Input {
        &self.inputs[index]
    }

    pub fn get_inputs(&self) -> &Vec<Input> {
        &self.inputs
    }

    pub fn set_input_connection(
        &mut self,
        input_index: usize,
        output_id: String,
        output_index: usize,
    ) {
        self.inputs[input_index].connection = Some((output_id, output_index));
    }

    pub fn clear_input_connection(&mut self, input_index: usize) {
        self.inputs[input_index].connection = None;
    }

    pub fn set_output_connection(
        &mut self,
        output_index: usize,
        input_id: String,
        input_index: usize,
    ) {
        if self.outputs[output_index].connection.is_some() {
            self.outputs[output_index]
                .connection
                .as_mut()
                .unwrap()
                .push((input_id, input_index));
        } else {
            self.outputs[output_index].connection = Some(vec![(input_id, input_index)]);
        }
    }

    pub async fn run(&mut self, tx_node_changed: Option<Sender<NodeChangedMessage>>) {
        match &mut self.node_type {
            // if node is an operation
            NodeType::Operation { operation } => {
                // run operation
                // collect results

                if let Some(tx) = tx_node_changed.clone() {
                    let message = NodeChangedMessage::Busy { node_id: self.id.clone(), is_busy: true };

                    match tx.try_send(message) {
                        Ok(_) => {}
                        Err(err) => {
                            println!(
                                "Error sending NodeChangedMessage::OutputChanged: {:?}",
                                err
                            );
                        }
                    }
                }

                // clear node error
                if self.is_error {
                    self.is_error = false;

                    if let Some(tx) = &tx_node_changed {
                        let message = NodeChangedMessage::Error {
                            node_id: self.id.clone(),
                            is_error: false,
                            message: None,
                        };
    
                        match tx.try_send(message) {
                            Ok(_) => {}
                            Err(err) => {
                                println!("Error sending NodeChangedMessage::InputChanged: {:?}", err);
                            }
                        }
                    }
                }

                // clear input errors
                // notify ui of change
                for (input_index, input) in self.inputs.iter_mut().enumerate() {
                    if input.is_error {
                        input.is_error = false;

                        if let Some(tx) = &tx_node_changed {
                            let message = NodeChangedMessage::InputErrorChanged {
                                node_id: self.id.clone(),
                                input_index: input_index,
                                is_error: false,
                                message: None,
                            };
        
                            match tx.try_send(message) {
                                Ok(_) => {}
                                Err(err) => {
                                    println!("Error sending NodeChangedMessage::InputChanged: {:?}", err);
                                }
                            }
                        }
                    }
                }

                match operation.run(&mut self.inputs).await {
                    Ok(operation_response) => {
                        // time node took to run
                        self.time = Some(operation_response.time);

                        if let Some(tx) = tx_node_changed.clone() {
                            let message = NodeChangedMessage::InfoChanged {
                                node_id: self.id.clone(),
                                time: operation_response.time,
                            };

                            match tx.try_send(message) {
                                Ok(_) => {}
                                Err(err) => {
                                    println!(
                                        "Error sending NodeChangedMessage::OutputChanged: {:?}",
                                        err
                                    );
                                }
                            }
                        }

                        // TODO: change response to a Result?
                        for (index, response) in operation_response.responses.into_iter().enumerate() {
                            // send messages to ui that outputs changed
                            if let Some(tx) = tx_node_changed.clone() {
                                let message = NodeChangedMessage::OutputChanged {
                                    node_id: self.id.clone(),
                                    output_index: index,
                                    value: response.value.clone(),
                                    thumbnail: response.value.create_thumbnail(),
                                };

                                match tx.try_send(message) {
                                    Ok(_) => {}
                                    Err(err) => {
                                        println!(
                                            "Error sending NodeChangedMessage::OutputChanged: {:?}",
                                            err
                                        );
                                    }
                                }
                            }

                            // set output's value
                            if let Some(output) = self.outputs.get_mut(index) {
                                output.value = response.value;
                            }
                        }
                    },
                    Err(operation_error) => {
                        // store node error message or none
                        let mut node_error_message: Option<String> = operation_error.node_error.clone();

                        // update inputs
                        // send input error messages
                        for input_error in operation_error.input_errors.iter() {
                            let (input_index, error_message) = input_error;

                            if let Some(input) = self.inputs.get_mut(*input_index) {
                                input.is_error = true;
                                input.error_message = Some(error_message.clone());

                                // if node error is empty fill it in
                                if node_error_message.is_none() {
                                    node_error_message = Some("Input error.".to_string());
                                }

                                // send message
                                if let Some(tx) = tx_node_changed.clone() {
                                    let message = NodeChangedMessage::InputErrorChanged {
                                        node_id: self.id.clone(),
                                        input_index: input_index.clone(),
                                        is_error: true,
                                        message: Some(error_message.clone()),
                                    };
    
                                    match tx.try_send(message) {
                                        Ok(_) => {}
                                        Err(err) => {
                                            println!(
                                                "Error sending NodeChangedMessage::OutputChanged: {:?}",
                                                err
                                            );
                                        }
                                    }
                                }
                            } else {
                                panic!("Invalid input index: {}", input_index);
                            }
                        }

                        // set node error
                        self.is_error = true;
                        self.error_message = node_error_message.clone();

                        // send node error changed
                        if let Some(tx) = tx_node_changed.clone() {
                            let message = NodeChangedMessage::Error {
                                node_id: self.id.clone(),
                                is_error: true,
                                message: node_error_message,
                            };

                            match tx.try_send(message) {
                                Ok(_) => {}
                                Err(err) => {
                                    println!(
                                        "Error sending NodeChangedMessage::OutputChanged: {:?}",
                                        err
                                    );
                                }
                            }
                        }
                    },
                }

                if let Some(tx) = tx_node_changed.clone() {
                    let message = NodeChangedMessage::Busy { node_id: self.id.clone(), is_busy: false };

                    match tx.try_send(message) {
                        Ok(_) => {}
                        Err(err) => {
                            println!(
                                "Error sending NodeChangedMessage::OutputChanged: {:?}",
                                err
                            );
                        }
                    }
                }
            }

            // if node is a subgraph
            NodeType::Subgraph {
                path: _,
                graph: subgraph_option,
                rx_node_changed,
            } => {
                match subgraph_option {
                    Some(subgraph) => {
                        // pass node's input to subgraph's input before running
                        for (_input_index, input) in self.inputs.iter().enumerate() {
                            if let Value::Path(_) = input.value {
                                // nothing
                            } else {
                                if let Some(link) = &input.link {
                                    if let Some(subgraph_node) =
                                        subgraph.nodes.get_mut(&link.node_id)
                                    {
                                        if let Some(i) = subgraph_node
                                            .inputs
                                            .iter_mut()
                                            .position(|i| i.id == link.input_id)
                                        {
                                            subgraph_node.set_input_value(i, input.value.clone());
                                        }
                                    }
                                }
                            }
                        }

                        // run subgraph
                        subgraph.run().await;

                        // receive messages about which nodes changed in subgraph
                        // if one changed that is exposed then pass it's output to this node's output
                        if let Some(rx) = rx_node_changed {
                            // receive messages
                            while let Ok(node_changed_message) = rx.try_recv() {
                                match node_changed_message {
                                    NodeChangedMessage::OutputChanged {
                                        node_id: subgraph_node_id,
                                        output_index: subgraph_output_index,
                                        value: subgraph_value,
                                        thumbnail: _subgraph_thumbnail,
                                    } => {
                                        // find output that is linked to subgraph output that changed
                                        for (_output_index, output) in
                                            self.outputs.iter_mut().enumerate()
                                        {
                                            if let Some(link) = &mut output.link {
                                                if link.node_id == subgraph_node_id
                                                    && link.output_index == subgraph_output_index
                                                {
                                                    // set output value to subgraph's new value
                                                    output.value = subgraph_value.clone();
                                                }
                                            }
                                        }

                                        //self.time = Some(subgraph_time);
                                    }
                                    // don't care about other messages
                                    _ => {}
                                }
                            }
                        }

                        // let ui know that outputs changed
                        if let Some(tx) = tx_node_changed {
                            for (output_index, output) in self.outputs.iter().enumerate() {
                                let message = NodeChangedMessage::OutputChanged {
                                    node_id: self.id.clone(),
                                    output_index,
                                    value: output.value.clone(),
                                    thumbnail: output.value.create_thumbnail(),
                                };

                                match tx.try_send(message) {
                                    Ok(_) => {}
                                    Err(err) => {
                                        println!(
                                            "Error sending NodeChangedMessage::OutputChanged: {:?}",
                                            err
                                        );
                                    }
                                }
                            }
                        }
                    }
                    None => {}
                }
            }
        };
    }
}
