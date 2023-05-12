// use std::time::Duration;

// use crate::{output::{Output, self}, input::{Input}, value::Value, node_trait::ConnectionSettings};

// #[derive(Debug, Clone)]
// pub struct NodeAttributes {
//     pub id: String,
//     pub inputs: Vec<Input>,
//     pub outputs: Vec<Output>,
//     pub time: Option<Duration>,
// }

// impl NodeAttributes {
//     pub fn new(id: String, input_settings: &Vec<ConnectionSettings>, output_settings: &Vec<ConnectionSettings>) -> NodeAttributes {

//         let inputs: Vec<Input> = input_settings.iter().map(|settings| Input {
//             name: settings.name.to_owned(),
//             value: settings.default_value.clone(),
//             connection: None,
//             valid_types: settings.valid_types.to_vec(),
//         }).collect();

//         let outputs: Vec<Output> = output_settings.iter().map(|settings| Output {
//             name: settings.name.to_owned(),
//             value: settings.default_value.clone(),
//             connection: None,
//         }).collect();

//         NodeAttributes {
//             id,
//             inputs,
//             outputs,
//             time: None,
//         }
//     }
    
//     pub fn set_intput_value(&mut self, index: usize, value: Value) {
//         if let Some(input) = self.inputs.get_mut(index) {
//             input.value = value;
//         } else {
//             panic!("Invalid input index: {}", index);
//         }
//     }

//     pub fn print_output(&self) -> String {
//         format!("{:?} {:.4}ms", self.outputs[0].value, self.time.unwrap().as_nanos() as f64 / 1_000_000.0)
//     }

//     pub fn set_input_connection(&mut self, input_index: usize, output_id: String) {
//         self.inputs[input_index].connection = Some(output_id);
//     }

//     pub fn set_output_connection(&mut self, output_index: usize, input_id: String) {
//         if self.outputs[output_index].connection.is_some() {
//             self.outputs[output_index].connection.as_mut().unwrap().push(input_id);
//         } else {
//             self.outputs[output_index].connection = Some(vec![input_id]);
//         }
//     }
// }