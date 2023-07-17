use crate::input::Input;
use crate::node_settings::NodeSettings;
use crate::operations::{OperationResponse, OperationError, OutputResponse};
use crate::output::Output;
use crate::value::{Value, ValueType};
use serde::{Deserialize, Serialize};
use std::time::Instant;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpNumberRandomInteger {}

impl OpNumberRandomInteger {
    pub fn settings() -> NodeSettings {
        NodeSettings {
            name: "random integer".to_string(),
            description: "Generates a random integer number between min and max.".to_string(),
        }
    }

    pub fn create_inputs() -> Vec<Input> {
        vec![
            Input::new("generate".to_string(), Value::Trigger, None, None),
            Input::new("min".to_string(), Value::Integer(i32::MIN), None, None),
            Input::new("max".to_string(), Value::Integer(i32::MAX), None, None),
        ]
    }

    pub fn create_outputs() -> Vec<Output> {
        vec![
            Output::new("output".to_string(), Value::Decimal(0.0), None)
        ]
    }

    pub async fn run(inputs: &mut Vec<Input>) -> Result<OperationResponse, OperationError> {
        let start_time = Instant::now();
        let mut input_errors: Vec<(usize, String)> = vec![];

        // convert inputs
        let min_converted = inputs[1].value.try_convert_to(ValueType::Integer);
        let max_converted = inputs[2].value.try_convert_to(ValueType::Integer);

        // gather errors
        if min_converted.is_err() { input_errors.push((1, min_converted.as_ref().err().unwrap().message.clone())); }
        if max_converted.is_err() { input_errors.push((2, max_converted.as_ref().err().unwrap().message.clone())); }

        // return if error
        if input_errors.len() > 0 { return Err(OperationError { input_errors, node_error: None }); }

        // get values
        let Ok(Value::Integer(minimum)) = min_converted else { return Err(OperationError { input_errors, node_error: Some("Error converting.".to_string()) }); };
        let Ok(Value::Integer(mut maximum)) = max_converted else { return Err(OperationError { input_errors, node_error: Some("Error converting.".to_string()) }); };

        // run node
        maximum = maximum.max(minimum+1);

        Ok(OperationResponse {
            time: Instant::now().duration_since(start_time),
            responses: vec![OutputResponse {
                value: Value::Integer(fastrand::i32(minimum..maximum)),
            }],
        })
    }
}
