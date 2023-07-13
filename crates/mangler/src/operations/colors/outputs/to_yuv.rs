use crate::color::Color;
use crate::input::{Input, InputSettings};
use crate::node_settings::NodeSettings;
use crate::operations::{OperationResponse, OperationError, OutputResponse};
use crate::output::Output;
use crate::value::{Value, ValueType};
use serde::{Deserialize, Serialize};
use std::time::Instant;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpColorOutputYuv {}

impl OpColorOutputYuv {
    pub fn settings() -> NodeSettings {
        NodeSettings {
            name: "to yuv".to_string(),
            description: "Converts a color to the YUV color space.".to_string(),
        }
    }

    pub fn create_inputs() -> Vec<Input> {
        vec![
            Input::new("input".to_string(), Value::Color(Color::default()), None, None),
        ]
    }

    pub fn create_outputs() -> Vec<Output> {
        vec![
            Output::new("y (luminance)".to_string(), Value::Decimal(0.5), None),
            Output::new("u (chrominance blue)".to_string(), Value::Decimal(0.5), None),
            Output::new("v (chrominance red)".to_string(), Value::Decimal(0.5), None),
            Output::new("alpha".to_string(), Value::Decimal(1.0), None),
        ]
    }

    pub async fn run(inputs: &Vec<Input>) -> Result<OperationResponse, OperationError> {
        let start_time = Instant::now();

        let Ok(Value::Color(color)) = inputs[0].value.try_convert_to(ValueType::Color) else { return Err(OperationError { message: "Unable to convert to integer.".to_string() })};

        let (y, u, v, alpha) = color.to_yuv();

        Ok(OperationResponse {
            time: Instant::now().duration_since(start_time),
            responses: vec![
                OutputResponse {value: Value::Decimal(y)},
                OutputResponse {value: Value::Decimal(u)},
                OutputResponse {value: Value::Decimal(v)},
                OutputResponse {value: Value::Decimal(alpha)},
            ],
        })
    }
}
