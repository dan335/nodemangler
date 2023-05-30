use crate::operation::Operation;
use glam::f32::Vec2;
use serde::{Deserialize, Serialize};
use std::fmt::Debug;
use std::time::Duration;
use tokio::sync::mpsc::Sender;

use crate::{input::Input, output::Output, value::Value, NodeOutputChangedMessage};

use super::node_settings::NodeSettings;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Node {
    pub operation: Operation,
    pub id: String,
    pub settings: NodeSettings,
    pub inputs: Vec<Input>,
    pub outputs: Vec<Output>,
    pub time: Option<Duration>,
    pub is_dirty: bool, // node needs to be re-run
    pub position: Vec2,
}

impl PartialEq for Node {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id
    }

    fn ne(&self, other: &Self) -> bool {
        self.id != other.id
    }
}

// impl Eq for Node {}

impl Node {
    pub fn new(id: String, operation: Operation, position: glam::f32::Vec2) -> Node {
        Node {
            id,
            inputs: operation.create_inputs(),
            outputs: operation.create_outputs(),
            settings: operation.settings(),
            time: None,
            operation,
            is_dirty: true,
            position,
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

    pub async fn run(&mut self, tx_output: Sender<NodeOutputChangedMessage>) {
        if let Ok(operation_response) = self.operation.run(&self.inputs).await {
            self.time = Some(operation_response.time);

            for (index, response) in operation_response.responses.into_iter().enumerate() {
                let node_output_message = NodeOutputChangedMessage {
                    node_id: self.id.clone(),
                    output_index: index,
                    thumbnail: response.value.create_thumbnail(),
                    value: response.value.clone(),
                    time: operation_response.time,
                };

                match tx_output.try_send(node_output_message.clone()) {
                    Ok(_) => {}
                    Err(err) => {
                        println!("Error sending NodeOutputChangedMessage: {:?}", err);
                    }
                }

                if let Some(output) = self.outputs.get_mut(index) {
                    output.value = response.value;
                }
            }
        }
    }
}
