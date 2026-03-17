use crate::input::{Input, InputSettings};
use crate::node_settings::NodeSettings;
use crate::operations::{OperationResponse, OperationError, OutputResponse};
use crate::output::Output;
use crate::value::{Value, ValueType};
use serde::{Deserialize, Serialize};
use std::time::Instant;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpNumberMathNthRt {}

impl OpNumberMathNthRt {
    pub fn settings() -> NodeSettings {
        NodeSettings {
            name: "nth root".to_string(),
            description: "Returns the nth root of a number.".to_string(),
        }
    }

    pub fn create_inputs() -> Vec<Input> {
        vec![
            Input::new("a".to_string(), Value::Decimal(1.0), Some(InputSettings::DragValue { speed:None, clamp:None }), None),
            Input::new("n".to_string(), Value::Decimal(1.0), Some(InputSettings::DragValue { speed:None, clamp:None }), None)
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

        let Ok(Value::Decimal(n)) = inputs[1].value.try_convert_to(ValueType::Decimal) else { return Err(OperationError { input_errors: vec![(1, "Unable to convert 'n' to Decimal.".to_string())], node_error: None })};

        if n == 0.0 {
            return Err(OperationError {
                input_errors: vec![(1, "Root degree cannot be zero.".to_string())], node_error: None,
            });
        }

        let num = match &inputs[0].value {
            Value::Integer(a) => Some(*a as f32),
            Value::Decimal(a) => Some(a.clone()),
            _ => None,
        };

        if let Some(mut num) = num {
            num = num.max(0.0);

            let nth_root = num.powf(1.0 / n);

            Ok(OperationResponse {
                time: Instant::now().duration_since(start_time),
                responses: vec![OutputResponse {
                    value: Value::Decimal(nth_root),
                }],
            })
        } else {
            return Err(OperationError {
                input_errors: vec![(0, "Unable to convert to a number.".to_string())],
                node_error: None,
            });
        }

        
    }
}
