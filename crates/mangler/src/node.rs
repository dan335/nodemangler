use tokio::sync::mpsc::Sender;
use tokio::time::{Duration};
use crate::{AddNodeType, NodeChangedMessage};
use crate::graph::Graph;
use crate::node_type::NodeType;
use glam::f32::Vec2;
use serde::{Deserialize, Serialize};
use std::fmt::Debug;
use std::path::PathBuf;


use crate::{input::Input, output::Output, value::Value};

use super::node_settings::NodeSettings;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Node {
    pub id: String,
    pub settings: NodeSettings,
    pub inputs: Vec<Input>,
    pub outputs: Vec<Output>,
    pub time: Option<Duration>,
    pub is_dirty: bool, // node needs to be re-run
    pub position: Vec2,
    pub node_type: NodeType,
}


impl PartialEq for Node {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id
    }
}


impl Node {
    pub fn new(id: String, node_type: AddNodeType, position: glam::f32::Vec2) -> Node {
        match node_type {
            AddNodeType::Operation(operation) => Node {
                id,
                settings: operation.settings(),
                inputs: operation.create_inputs(),
                outputs: operation.create_outputs(),
                time: None,
                is_dirty: true,
                position,
                node_type: NodeType::Operation { operation }
            },
            AddNodeType::Subgraph => Node {
                id,
                settings: NodeSettings { name: "subgraph".to_string() },
                inputs: Vec::new(),
                outputs: Vec::new(),
                time: None,
                is_dirty: true,
                position,
                node_type: NodeType::Subgraph { path: PathBuf::new(), graph: None, rx_node_changed: None }
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

    pub async fn run(&mut self, tx_output: Option<Sender<NodeChangedMessage>>) {
        match &mut self.node_type {
            NodeType::Operation { operation } => {
                if let Ok(operation_response) = operation.run(&self.inputs).await {
                    self.time = Some(operation_response.time);
println!("running op {:?} {:?}", operation, self.id);
                    for (index, response) in operation_response.responses.into_iter().enumerate() {
                        if let Some(tx) = tx_output.clone() {
                            let message = NodeChangedMessage::OutputChanged {
                                node_id: self.id.clone(),
                                output_index: index,
                                value: response.value.clone(),
                                time: operation_response.time,
                                thumbnail: response.value.create_thumbnail(),
                            };
            
                            match tx.try_send(message) {
                                Ok(_) => {}
                                Err(err) => {
                                    println!("Error sending NodeChangedMessage::OutputChanged: {:?}", err);
                                }
                            }
                        }
        
                        if let Some(output) = self.outputs.get_mut(index) {
                            output.value = response.value;
                            println!("output value {:?}", output.value);
                        }
                    }
                }
            },

            NodeType::Subgraph { path:_, graph: subgraph_option, rx_node_changed } => {                
                match subgraph_option {
                    Some(subgraph) => {
                        // todo:
                        // need to pass this node's inputs to inputs that are exposed in graph
                        // send message that this node's inputs changed

                        for (_input_index, input) in self.inputs.iter().enumerate() {
                            if let Value::Path(_) = input.value {
                                // nothing
                            } else {
                                if let Some(link) = &input.link {
                                            if let Some(subgraph_node) = subgraph.nodes.get_mut(&link.node_id) {
                                                if let Some(i) = subgraph_node.inputs.iter_mut().position(|i| i.id == link.input_id) {
                                                    subgraph_node.set_input_value(i, input.value.clone());
                                                }
                                            }
                                }
                            }
                        }
                        
                        subgraph.run().await;

                        if let Some(rx) = rx_node_changed {
                            while let Ok(node_changed_message) = rx.try_recv() {
                                match node_changed_message {
                                    //NodeChangedMessage::InputChanged { node_id, input_index, value } => todo!(),
                                    NodeChangedMessage::OutputChanged { node_id: subgraph_node_id, output_index: subgraph_output_index, value: subgraph_value, time: subgraph_time, thumbnail: subgraph_thumbnail } => {
                                        for (output_index, output) in self.outputs.iter_mut().enumerate() {
                                            if let Some(link) = &mut output.link {
                                                if link.node_id == subgraph_node_id && link.output_index == subgraph_output_index {
                                                    if let Some(tx) = tx_output.clone() {
                                                        let message = NodeChangedMessage::OutputChanged {
                                                            node_id: self.id.clone(),
                                                            output_index: output_index,
                                                            value: subgraph_value.clone(),
                                                            time: subgraph_time,
                                                            thumbnail: subgraph_thumbnail.clone(),
                                                        };
                                        
                                                        match tx.try_send(message) {
                                                            Ok(_) => {}
                                                            Err(err) => {
                                                                println!("Error sending NodeChangedMessage::OutputChanged: {:?}", err);
                                                            }
                                                        }
                                                    }
                                    
                                                    output.value = subgraph_value.clone();
                                                    println!("output value {:?}", output.value);
                                                }
                                            }
                                        }
                                    },
                                    //NodeChangedMessage::ExposeInputChanged { node_id, input_index, set_to } => todo!(),
                                    //NodeChangedMessage::ExposeOutputChanged { node_id, output_index, set_to } => todo!(),
                                    //NodeChangedMessage::SubgraphLoaded { node_id, settings, inputs, outputs } => todo!(),
                                    _ => {}
                                }
                            }
                        }
                        
                        // todo
                        // need to pass exposed graph's outputs to this node's outputs
                        // also need to send message that node changed    
                    println!("{:?}", self.inputs);
                        // let ui know that outputs changed
                        if let Some(tx) = tx_output {
                            println!("has output");
                            for (output_index, output) in self.outputs.iter().enumerate() {
                                println!("output {}", output_index);
                                let message = NodeChangedMessage::OutputChanged {
                                    node_id: self.id.clone(),
                                    output_index,
                                    value: output.value.clone(),
                                    time: self.time.unwrap_or_default(),
                                    thumbnail: output.value.create_thumbnail(),
                                };
    //println!("{:?}", message);
                                match tx.try_send(message) {
                                    Ok(_) => {}
                                    Err(err) => {
                                        println!("Error sending NodeChangedMessage::OutputChanged: {:?}", err);
                                    }
                                }
                            }
                        }
                        
                    },
                    None => {},
                }
            },
        };
    }
}



// #[derive(Debug)]
// pub enum Data {
//     Subgraph(Option<SubgraphData>)
// }

// #[derive(Debug)]
// pub struct SubgraphData {
//     pub graph: Graph
// }