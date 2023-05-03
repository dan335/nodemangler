use std::time::Duration;

use crate::{output::{Output, OutputSettings}, input::{Input, InputSettings}, value::Value};

#[derive(Debug)]
pub struct NodeAttributes {
    pub id: String,
    pub inputs: Vec<Input>,
    pub outputs: Vec<Output>,
    pub time: Option<Duration>,
}

impl NodeAttributes {
    pub fn new(id: String, input_settings: &Vec<InputSettings>, output_settings: &Vec<OutputSettings>) -> NodeAttributes {

        let inputs: Vec<Input> = input_settings.iter().map(|settings| Input {
            name: settings.name.to_owned(),
            value: settings.default_value.clone(),
            connection: None,
            valid_types: settings.valid_types.to_vec(),
        }).collect();

        let outputs: Vec<Output> = output_settings.iter().map(|settings| Output {
            name: settings.name.to_owned(),
            value: settings.default_value.clone(),
            connection: None,
        }).collect();

        NodeAttributes {
            id,
            inputs,
            outputs,
            time: None,
        }
    }
    
    pub fn set_intput_value(&mut self, index: usize, value: Value) {
        if let Some(input) = self.inputs.get_mut(index) {
            input.value = value;
        } else {
            panic!("Invalid input index: {}", index);
        }
    }

    pub fn print_output(&self) -> String {
        format!("{:?} {:.4}ms", self.outputs[0].value, self.time.unwrap().as_nanos() as f64 / 1_000_000.0)
    }
}