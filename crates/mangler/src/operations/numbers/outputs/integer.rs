use crate::input::Input;
use crate::node_settings::NodeSettings;
use crate::operation::{OperationError, OperationResponse, OutputResponse};
use crate::output::Output;
use crate::value::{Value, ValueType};
use serde::{Deserialize, Serialize};
use std::time::Instant;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OperationNumberOutputInteger {}

impl OperationNumberOutputInteger {
    pub fn settings() -> NodeSettings {
        NodeSettings {
            name: "integer output".to_string(),
        }
    }

    pub fn create_inputs() -> Vec<Input> {
        vec![
            Input::new("integer".to_string(), Value::Integer(i32::default()), None)
        ]
    }

    pub fn create_outputs() -> Vec<Output> {
        vec![
            Output::new("integer".to_string(), Value::Integer(i32::default()), None)
        ]
    }

    pub async fn run(inputs: &Vec<Input>) -> Result<OperationResponse, OperationError> {
        let start_time = Instant::now();

        match &inputs[0].value.try_convert_to(ValueType::Integer) {
            Ok(new_value) => Ok(OperationResponse {
                time: Instant::now().duration_since(start_time),
                responses: vec![OutputResponse {
                    value: new_value.clone(),
                }],
            }),
            Err(_) => Err(OperationError {
                message: "Error converting. {:?}".to_string(),
            }),
        }
    }
}
