use serde::{Serialize, Deserialize};
use tokio::sync::mpsc::Sender;
use crate::{NodeOutputChangedMessage, value::Value};
use core::fmt::Debug;
use std::time::Duration;
use crate::operations::{
    float::new_float,
    add::add,
    image_from_clipboard::image_from_clipboard,
    image_from_url::image_from_url,
    image_resize::image_resize,
    integer::new_integer,
    subtract::subtract,
    text_from_clipboard::text_from_clipboard,
};

use crate::{
    input::Input,
    output::Output,
    value::ValueType,
};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Operation {
    Add,
    Subtract,
    Float,
    Integer,
    ImageFromUrl,
    ImageResize,
    ImageFromClipboard,
    TextFromClipboard,
}

impl Operation {
    pub async fn run(&self, node_id: &String, inputs: &Vec<Input>, outputs: &mut Vec<Output>, tx_output: Sender<NodeOutputChangedMessage>) -> Duration {
        let p_operation_responses: Result<Vec<OperationResponse>, OperationError> = match self {
            Operation::Float => new_float(inputs).await,
            Operation::Integer => new_integer(inputs).await,
            Operation::Add => add(inputs).await,
            Operation::Subtract => subtract(inputs).await,
            Operation::ImageFromUrl => image_from_url(inputs).await,
            Operation::ImageResize => image_resize(inputs).await,
            Operation::ImageFromClipboard => image_from_clipboard(inputs).await,
            Operation::TextFromClipboard => text_from_clipboard(inputs).await,
        };

        if let Ok(operation_responses) = p_operation_responses {
            let time = operation_responses[0].time;

            for operation_response in operation_responses.into_iter() {

                let node_output_message = NodeOutputChangedMessage {
                    node_id: node_id.clone(),
                    output_index: operation_response.index,
                    value: operation_response.value.clone(),
                    value_type: operation_response.value.value_type(),
                    time: operation_response.time,
                    thumbnail: operation_response.value.create_thumbnail(),
                };

                outputs[operation_response.index].value = operation_response.value;

                match tx_output.try_send(node_output_message.clone()) {
                    Ok(_) => {},
                    Err(err) => {
                        println!("Error sending NodeOutputChangedMessage: {:?}", err);
                    },
                }
            }

            return time;
        }
        
        Duration::ZERO
    }
}

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


pub struct OperationResponse {
    pub index: usize,
    pub value: Value,
    pub time: Duration,
}


#[derive(Debug)]
pub struct OperationError {
    pub message: String,
}