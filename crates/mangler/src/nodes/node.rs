use std::{time::Duration, thread::JoinHandle};
use crate::{input::Input, output::Output, value::Value, get_id};

use super::{
    node_settings::NodeSettings,
    operation::{ConnectionSettings, Operation, OperationResponse, self},
};

#[derive(Debug)]
pub struct Node {
    pub operation: Operation,
    pub id: String,
    pub settings: NodeSettings,
    pub inputs: Vec<Input>,
    pub outputs: Vec<Output>,
    pub time: Option<Duration>,
    pub is_dirty: bool, // node needs to be re-run
    pub change_id: String,  // id that gets chagned when ui for this node needs to udpate
}

impl PartialEq for Node {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id
    }
}

// impl Eq for Node {}

impl Node {
    // fn hash<H: Hasher>(&self, state: &mut H) {
    //     self.id.hash(state);
    // }

    pub fn new(
        id: String,
        settings: NodeSettings,
        input_settings: &[ConnectionSettings],
        output_settings: &[ConnectionSettings],
        operation: Operation,
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
            change_id: get_id(),
        }
    }

    pub fn set_input_value(&mut self, index: usize, value: Value) {
        if let Some(input) = self.inputs.get_mut(index) {
            input.set_value(value);//value = value;
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

    pub fn run(&mut self) {
        self.time = Some(self.operation.run(&self.inputs, &mut self.outputs));
        self.change_id = get_id();
        
        // let response = operation_handle.join();
        // if let Ok(operation_response) = response {
        //     for (index, o) in operation_response.output_values.iter().enumerate() {
        //         self.outputs[index].value = o.clone();
        //     }
        //     self.time = Some(operation_response.time);
        //     self.change_id = get_id();
        // }
    }
}
