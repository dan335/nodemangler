use crate::input::{Input, InputSettings};
use crate::node_settings::NodeSettings;
use crate::operations::{OperationResponse, OperationError, OutputResponse};
use crate::output::Output;
use crate::value::{Value, ValueType};
use serde::{Deserialize, Serialize};
use std::time::Instant;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpNumberMathClamp {}

impl OpNumberMathClamp {
    pub fn settings() -> NodeSettings {
        NodeSettings {
            name: "clamp".to_string(),
        }
    }

    pub fn create_inputs() -> Vec<Input> {
        vec![
            Input::new("a".to_string(), Value::Decimal(1.0), Some(InputSettings::DragValue { speed:None, clamp:None }), None),
            Input::new("min".to_string(), Value::Decimal(0.0), Some(InputSettings::DragValue { speed:None, clamp:None }), None),
            Input::new("max".to_string(), Value::Decimal(1.0), Some(InputSettings::DragValue { speed:None, clamp:None }), None),
        ]
    }

    pub fn create_outputs() -> Vec<Output> {
        vec![
            Output::new("output".to_string(), Value::Decimal(f32::default()), None)
        ]
    }

    pub async fn run(inputs: &Vec<Input>) -> Result<OperationResponse, OperationError> {
        let start_time = Instant::now();

        let Ok(Value::Decimal(min)) = inputs[1].value.try_convert_to(ValueType::Decimal) else { return Err(OperationError { message: "Unable to convert to integer.".to_string() })};
        let Ok(Value::Decimal(max)) = inputs[2].value.try_convert_to(ValueType::Decimal) else { return Err(OperationError { message: "Unable to convert to integer.".to_string() })};

        let value = match &inputs[0].value {
            Value::Integer(a) => Value::Integer(*a.clamp(&(min as i32), &(max as i32))),
            Value::Decimal(a) => Value::Decimal(a.clone().clamp(min, max)),

            _ => {return Err(OperationError {
                message: "Error converting. {:?}".to_string(),
            });}
        };

        Ok(OperationResponse {
            time: Instant::now().duration_since(start_time),
            responses: vec![OutputResponse {
                value: value,
            }],
        })
    }
}
