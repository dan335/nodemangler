use crate::input::Input;
use crate::node_settings::NodeSettings;
use crate::operation::{OperationError, OperationResponse, OutputResponse};
use crate::output::Output;
use crate::value::{Value, ValueType};
use std::time::Instant;
use serde::{Serialize, Deserialize};



#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OperationInputInteger {}


impl OperationInputInteger {
    pub fn settings() -> NodeSettings {
        NodeSettings { name: "Integer".to_string() }
    }

    pub fn create_inputs() -> Vec<Input> {
        vec![
            Input {
                name: "Integer".to_string(),
                value: Value::Integer(i32::default()),
                connection: None,
                valid_types: vec![],
            },
        ]
    }

    pub fn create_outputs() -> Vec<Output> {
        vec![
            Output {
                name: "Integer".to_string(),
                value: Value::Integer(i32::default()),
                connection: None,
            }
        ]
    }

    pub async fn run(inputs: &[Input]) -> Result<OperationResponse, OperationError> {
        let start_time = Instant::now();

        match &inputs[0].value.try_convert_to(ValueType::Integer) {
            Ok(new_value) => {
                Ok(OperationResponse {
                    time: Instant::now().duration_since(start_time),
                    outputs: vec![OutputResponse{
                        value: new_value.clone()
                    }],
                })
            },
            Err(_) => Err(OperationError{message:"Error converting. {:?}".to_string()}),
        }
    }
}