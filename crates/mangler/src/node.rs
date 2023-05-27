use std::time::Duration;
use tokio::sync::mpsc::Sender;
use glam::f32::Vec2;
use serde::{Deserialize, Serialize};

use crate::{input::Input, output::Output, value::Value, NodeOutputChangedMessage};

use super::{
    node_settings::NodeSettings,
    operation::{ConnectionSettings, Operation},
};

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
}

// impl Eq for Node {}

impl Node {

    pub fn new(
        id: String,
        settings: NodeSettings,
        input_settings: Vec<ConnectionSettings>,
        output_settings: Vec<ConnectionSettings>,
        operation: Operation,
        position: glam::f32::Vec2,
    ) -> Node {
        let inputs: Vec<Input> = input_settings
            .iter()
            // .map(|settings| Input {
            //     name: settings.name.to_owned(),
            //     value: settings.default_value.clone(),
            //     connection: None,
            //     valid_types: settings.valid_types.to_vec(),
            //     ui_type: settings.ui_type.clone(),
            // })
            .map(|settings| Input::new(settings.clone()))
            .collect();

        let outputs: Vec<Output> = output_settings
            .iter()
            .map(|settings| Output {
                name: settings.name.to_owned(),
                value_type: settings.default_value.value_type(),
                value: settings.default_value.clone(),
                connection: None,
            })
            .collect();

        Node {
            id,
            inputs,
            outputs,
            time: None,
            operation,
            settings,
            is_dirty: true,
            position,
        }
    }

    pub fn set_input_value(&mut self, index: usize, value: Value) {
        if let Some(input) = self.inputs.get_mut(index) {
            input.set_value(value); //value = value;
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
        self.time = Some(
            self.operation
                .run(&self.id, &self.inputs, &mut self.outputs, tx_output)
                .await,
        );
    }
}
