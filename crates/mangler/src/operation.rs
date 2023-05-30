use serde::{Serialize, Deserialize};
use crate::node_settings::NodeSettings;
use crate::operations::input_integer::OperationInputInteger;
use crate::value::Value;
use core::fmt::Debug;
use std::time::Duration;


use crate::{
    input::Input,
    output::Output,
    value::ValueType,
};


#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum Operation {
    InputInteger
}

impl Operation {
    pub fn settings(&self) -> NodeSettings {
        match self {
            Operation::InputInteger => OperationInputInteger::settings(),
        }
    }

    pub fn create_inputs(&self) -> Vec<Input> {
        match self {
            Operation::InputInteger => OperationInputInteger::create_inputs(),
        }
    }

    pub fn create_outputs(&self) -> Vec<Output> {
        match self {
            Operation::InputInteger => OperationInputInteger::create_outputs(),
        }
    }

    pub async fn run(&self, inputs: &[Input]) -> Result<OperationResponse, OperationError> {
        match self {
            Operation::InputInteger => OperationInputInteger::run(inputs).await,
        }
    }
}


// #[derive(Debug, Clone, Serialize, Deserialize)]
// pub enum Operation {
//     Add,
//     Subtract,
//     Float,
//     Integer,
//     ImageFromUrl,
//     ImageResize,
//     ImageFromClipboard,
//     TextFromClipboard,
// }

// impl Operation {
//     pub async fn run(&self, node_id: &String, inputs: &Vec<Input>, outputs: &mut Vec<Output>, tx_output: Sender<NodeOutputChangedMessage>) -> Duration {
//         let p_operation_response: Result<OperationResponse, OperationError> = match self {
//             Operation::Float => new_float(inputs).await,
//             Operation::Integer => OperationInputInteger::run(inputs).await,
//             Operation::Add => add(inputs).await,
//             Operation::Subtract => subtract(inputs).await,
//             Operation::ImageFromUrl => image_from_url(inputs).await,
//             Operation::ImageResize => image_resize(inputs).await,
//             Operation::ImageFromClipboard => image_from_clipboard(inputs).await,
//             Operation::TextFromClipboard => text_from_clipboard(inputs).await,
//         };

//         if let Ok(operation_response) = p_operation_response {
//             let time = operation_response.time;

//             for (index, output) in operation_response.outputs.into_iter().enumerate() {

//                 let node_output_message = NodeOutputChangedMessage {
//                     node_id: node_id.clone(),
//                     output_index: index,
//                     value: output.value.clone(),
//                     value_type: output.value.value_type(),
//                     time: operation_response.time,
//                     thumbnail: output.value.create_thumbnail(),
//                 };

//                 outputs[index].value = output.value;

//                 match tx_output.try_send(node_output_message.clone()) {
//                     Ok(_) => {},
//                     Err(err) => {
//                         println!("Error sending NodeOutputChangedMessage: {:?}", err);
//                     },
//                 }
//             }

//             return time;
//         }
        
//         Duration::ZERO
//     }
// }

#[derive(Debug, Clone)]
pub struct ConnectionSettings {
    pub name: String,
    pub default_value: Value,
    pub valid_types: Vec<ValueType>,
    pub ui_type: Option<UiType>, // for output connections it's none
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum UiType {
    DragValue,
    Checkbox,
    Slider,
    TextEdit,
    ComboBox,
    UiButton,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OperationResponse {
    pub responses: Vec<OutputResponse>,
    pub time: Duration,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OutputResponse {
    pub value: Value,
}


#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OperationError {
    pub message: String,
}