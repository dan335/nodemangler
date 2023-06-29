use crate::input::{Input, InputSettings};
use crate::node_settings::NodeSettings;
use crate::operation::{OperationError, OperationResponse, OutputResponse};
use crate::output::Output;
use crate::value::{Value, ValueType};
use serde::{Deserialize, Serialize};
use std::time::Instant;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OperationNumberCastToDecimal {}

impl OperationNumberCastToDecimal {
    pub fn settings() -> NodeSettings {
        NodeSettings {
            name: "to decimal".to_string(),
        }
    }

    pub fn create_inputs() -> Vec<Input> {
        vec![
            Input::new("a".to_string(), Value::Decimal(f32::default()), InputSettings::Decimal(crate::input::DecimalInputType::DragValue { speed:None, clamp: None }), None),
        ]
    }

    pub fn create_outputs() -> Vec<Output> {
        vec![
            Output::new("output".to_string(), Value::Decimal(f32::default()), None)
        ]
    }

    pub async fn run(inputs: &Vec<Input>) -> Result<OperationResponse, OperationError> {
        let start_time = Instant::now();

        let Ok(Value::Decimal(n)) = inputs[0].value.try_convert_to(ValueType::Decimal) else { return Err(OperationError { message: "Unable to convert to decimal.".to_string() })};

        Ok(OperationResponse {
            time: Instant::now().duration_since(start_time),
            responses: vec![OutputResponse {
                value: Value::Decimal(n),
            }],
        })
    }
}
