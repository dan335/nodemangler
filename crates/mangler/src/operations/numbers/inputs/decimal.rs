use crate::input::{Input, InputSettings};
use crate::node_settings::NodeSettings;
use crate::operations::{OperationResponse, OperationError, OutputResponse};
use crate::output::Output;
use crate::value::{Value, ValueType};
use serde::{Deserialize, Serialize};
use std::time::Instant;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpNumberInputDecimal {}

impl OpNumberInputDecimal {
    pub fn settings() -> NodeSettings {
        NodeSettings {
            name: "decimal".to_string(),
        }
    }

    pub fn create_inputs() -> Vec<Input> {
        vec![
            Input::new("input".to_string(), Value::Decimal(0.0), Some(InputSettings::DragValue { speed:None, clamp:None }), None)
        ]
    }

    pub fn create_outputs() -> Vec<Output> {
        vec![
            Output::new("output".to_string(), Value::Decimal(0.0), None)
        ]
    }

    pub async fn run(inputs: &Vec<Input>) -> Result<OperationResponse, OperationError> {
        let start_time = Instant::now();

        match &inputs[0].value.try_convert_to(ValueType::Decimal) {
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
