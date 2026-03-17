use crate::input::{Input, InputSettings};
use crate::node_settings::NodeSettings;
use crate::operations::{OperationResponse, OperationError, OutputResponse};
use crate::output::Output;
use crate::value::{Value, ValueType};
use serde::{Deserialize, Serialize};
use std::time::Instant;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpNumberCastToDecimal {}

impl OpNumberCastToDecimal {
    pub fn settings() -> NodeSettings {
        NodeSettings {
            name: "to decimal".to_string(),
            description: "Converts a number to a decimal.".to_string(),
        }
    }

    pub fn create_inputs() -> Vec<Input> {
        vec![
            Input::new("a".to_string(), Value::Decimal(f32::default()), Some(InputSettings::DragValue { speed:None, clamp:None }), None),
        ]
    }

    pub fn create_outputs() -> Vec<Output> {
        vec![
            Output::new("output".to_string(), Value::Decimal(f32::default()), None)
        ]
    }

    pub async fn run(inputs: &mut Vec<Input>) -> Result<OperationResponse, OperationError> {
        let start_time = Instant::now();
        let input_errors: Vec<(usize, String)> = vec![];

        // convert inputs
        // gather errors

        // return if error
        if input_errors.len() > 0 { return Err(OperationError { input_errors, node_error: None }); }

        // get values
        // run node

        let Ok(Value::Decimal(n)) = inputs[0].value.try_convert_to(ValueType::Decimal) else { return Err(OperationError { input_errors: vec![(0, "Unable to convert to decimal.".to_string())], node_error: None })};

        Ok(OperationResponse {
            time: Instant::now().duration_since(start_time),
            responses: vec![OutputResponse {
                value: Value::Decimal(n),
            }],
        })
    }
}
