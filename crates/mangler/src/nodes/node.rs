use std::collections::HashMap;
use std::time::Duration;

use crate::{input::Input, output::Output, value::Value};

use super::{
    node_settings::NodeSettings,
    operation::{ConnectionSettings, Operation},
};

#[derive(Debug, Clone)]
pub struct Node {
    pub operation: Operation,
    pub id: String,
    pub settings: NodeSettings,
    pub inputs: Vec<Input>,
    pub outputs: Vec<Output>,
    pub time: Option<Duration>,
    pub is_dirty: bool, // node needs to be re-run
}

impl Node {
    pub fn new(
        id: String,
        settings: NodeSettings,
        input_settings: &[ConnectionSettings],
        output_settings: &[ConnectionSettings],
        operation: Operation,
    ) -> Node {
        let inputs: Vec<Input> = input_settings
            .iter()
            .map(|settings| Input {
                name: settings.name.to_owned(),
                value: settings.default_value.clone(),
                connection: None,
                valid_types: settings.valid_types.to_vec(),
                ui_type: settings.ui_type.clone(),
            })
            .collect();

        let outputs: Vec<Output> = output_settings
            .iter()
            .map(|settings| Output {
                name: settings.name.to_owned(),
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
            //dependencies_are_dirty: true,
            is_dirty: true,
        }
    }

    pub fn set_input_value(&mut self, index: usize, value: Value) {
        if let Some(input) = self.inputs.get_mut(index) {
            input.value = value;
        } else {
            panic!("Invalid input index: {}", index);
        }
    }

    pub fn set_input_connection(
        &mut self,
        input_index: usize,
        output_id: String,
        output_index: usize,
    ) {
        self.inputs[input_index].connection = Some((output_id, output_index));
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

    pub fn pass_outputs_to_connections(&self, nodes: &mut HashMap<String, Node>) {
        for output in &self.outputs {
            output.pass_value_to_connections(nodes);
        }
    }

    pub fn run(&mut self) {
        self.time = Some(self.operation.run(&self.inputs, &mut self.outputs));
    }
}
