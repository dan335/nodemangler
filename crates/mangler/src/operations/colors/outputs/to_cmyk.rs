use crate::color::Color;
use crate::input::{Input, InputSettings};
use crate::node_settings::NodeSettings;
use crate::operations::{OperationResponse, OperationError, OutputResponse};
use crate::output::Output;
use crate::value::{Value, ValueType};
use serde::{Deserialize, Serialize};
use std::time::Instant;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpColorOutputCmyk {}

impl OpColorOutputCmyk {
    pub fn settings() -> NodeSettings {
        NodeSettings {
            name: "to cmyk".to_string(),
        }
    }

    pub fn create_inputs() -> Vec<Input> {
        vec![
            Input::new("input".to_string(), Value::Color(Color::default()), None, None),
        ]
    }

    pub fn create_outputs() -> Vec<Output> {
        vec![
            Output::new("cyan".to_string(), Value::Decimal(0.5), None),
            Output::new("magenta".to_string(), Value::Decimal(0.5), None),
            Output::new("yellow".to_string(), Value::Decimal(0.5), None),
            Output::new("key (black)".to_string(), Value::Decimal(0.5), None),
            Output::new("alpha".to_string(), Value::Decimal(1.0), None),
        ]
    }

    pub async fn run(inputs: &Vec<Input>) -> Result<OperationResponse, OperationError> {
        let start_time = Instant::now();

        let Ok(Value::Color(color)) = inputs[0].value.try_convert_to(ValueType::Color) else { return Err(OperationError { message: "Unable to convert to integer.".to_string() })};

        let (c, m, y, k, a) = color.to_cmyk();

        Ok(OperationResponse {
            time: Instant::now().duration_since(start_time),
            responses: vec![
                OutputResponse {value: Value::Decimal(c)},
                OutputResponse {value: Value::Decimal(m)},
                OutputResponse {value: Value::Decimal(y)},
                OutputResponse {value: Value::Decimal(k)},
                OutputResponse {value: Value::Decimal(a)},
            ],
        })
    }
}
