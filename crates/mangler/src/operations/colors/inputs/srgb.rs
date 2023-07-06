use crate::color::Color;
use crate::input::{Input, InputSettings};
use crate::node_settings::NodeSettings;
use crate::operations::{OperationResponse, OperationError, OutputResponse};
use crate::output::Output;
use crate::value::{Value, ValueType};
use serde::{Deserialize, Serialize};
use std::time::Instant;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpColorInputRgba {}

impl OpColorInputRgba {
    pub fn settings() -> NodeSettings {
        NodeSettings {
            name: "rgb".to_string(),
        }
    }

    pub fn create_inputs() -> Vec<Input> {
        vec![
            Input::new("red".to_string(), Value::Decimal(0.5), Some(InputSettings::Slider { range: (0.0, 1.0), step_by: Some(0.01), clamp_to_range: false }), None),
            Input::new("green".to_string(), Value::Decimal(0.5), Some(InputSettings::Slider { range: (0.0, 1.0), step_by: Some(0.01), clamp_to_range: false }), None),
            Input::new("blue".to_string(), Value::Decimal(0.5), Some(InputSettings::Slider { range: (0.0, 1.0), step_by: Some(0.01), clamp_to_range: false }), None),
            Input::new("alpha".to_string(), Value::Decimal(1.0), Some(InputSettings::Slider { range: (0.0, 1.0), step_by: Some(0.01), clamp_to_range: true }), None),
        ]
    }

    pub fn create_outputs() -> Vec<Output> {
        vec![
            Output::new("output".to_string(), Value::Color(Color::default()), None)
        ]
    }

    pub async fn run(inputs: &Vec<Input>) -> Result<OperationResponse, OperationError> {
        let start_time = Instant::now();

        let Ok(Value::Decimal(red)) = inputs[0].value.try_convert_to(ValueType::Decimal) else { return Err(OperationError { message: "Unable to convert to integer.".to_string() })};
        let Ok(Value::Decimal(green)) = inputs[1].value.try_convert_to(ValueType::Decimal) else { return Err(OperationError { message: "Unable to convert to integer.".to_string() })};
        let Ok(Value::Decimal(blue)) = inputs[2].value.try_convert_to(ValueType::Decimal) else { return Err(OperationError { message: "Unable to convert to integer.".to_string() })};
        let Ok(Value::Decimal(alpha)) = inputs[3].value.try_convert_to(ValueType::Decimal) else { return Err(OperationError { message: "Unable to convert to integer.".to_string() })};

        let color = Color::from_srgb_float(red, green, blue, alpha);

        Ok(OperationResponse {
            time: Instant::now().duration_since(start_time),
            responses: vec![OutputResponse {
                value: Value::Color(color),
            }],
        })
    }
}
