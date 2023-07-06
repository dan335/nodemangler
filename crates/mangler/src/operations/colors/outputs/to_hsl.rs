use crate::color::Color;
use crate::input::{Input, InputSettings};
use crate::node_settings::NodeSettings;
use crate::operations::{OperationResponse, OperationError, OutputResponse};
use crate::output::Output;
use crate::value::{Value, ValueType};
use serde::{Deserialize, Serialize};
use std::time::Instant;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpColorOutputHsl {}

impl OpColorOutputHsl {
    pub fn settings() -> NodeSettings {
        NodeSettings {
            name: "to hsl".to_string(),
        }
    }

    pub fn create_inputs() -> Vec<Input> {
        vec![
            Input::new("input".to_string(), Value::Color(Color::default()), None, None),
        ]
    }

    pub fn create_outputs() -> Vec<Output> {
        vec![
            Output::new("hue".to_string(), Value::Decimal(0.5), None),
            Output::new("saturation".to_string(), Value::Decimal(0.5), None),
            Output::new("lightness".to_string(), Value::Decimal(0.5), None),
            Output::new("alpha".to_string(), Value::Decimal(1.0), None),
        ]
    }

    pub async fn run(inputs: &Vec<Input>) -> Result<OperationResponse, OperationError> {
        let start_time = Instant::now();

        let Ok(Value::Color(color)) = inputs[0].value.try_convert_to(ValueType::Color) else { return Err(OperationError { message: "Unable to convert to integer.".to_string() })};

        let (h, s, l, a) = color.to_hsl();

        Ok(OperationResponse {
            time: Instant::now().duration_since(start_time),
            responses: vec![
                OutputResponse {value: Value::Decimal(h)},
                OutputResponse {value: Value::Decimal(s)},
                OutputResponse {value: Value::Decimal(l)},
                OutputResponse {value: Value::Decimal(a)},
            ],
        })
    }
}
