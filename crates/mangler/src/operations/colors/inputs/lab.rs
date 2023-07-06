use crate::color::Color;
use crate::input::{Input, InputSettings};
use crate::node_settings::NodeSettings;
use crate::operations::{OperationResponse, OperationError, OutputResponse};
use crate::output::Output;
use crate::value::{Value, ValueType};
use serde::{Deserialize, Serialize};
use std::time::Instant;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpColorInputLab {}

impl OpColorInputLab {
    pub fn settings() -> NodeSettings {
        NodeSettings {
            name: "lab".to_string(),
        }
    }

    pub fn create_inputs() -> Vec<Input> {
        vec![
            Input::new("lightness".to_string(), Value::Decimal(50.0), Some(InputSettings::Slider { range: (0.0, 100.0), step_by: Some(1.0), clamp_to_range: false }), None),
            Input::new("green - red".to_string(), Value::Decimal(0.0), Some(InputSettings::Slider { range: (-128.0, 127.0), step_by: Some(1.0), clamp_to_range: false }), None),
            Input::new("blue - yellow".to_string(), Value::Decimal(0.0), Some(InputSettings::Slider { range: (-128.0, 127.0), step_by: Some(1.0), clamp_to_range: false }), None),
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

        let Ok(Value::Decimal(l)) = inputs[0].value.try_convert_to(ValueType::Decimal) else { return Err(OperationError { message: "Unable to convert to integer.".to_string() })};
        let Ok(Value::Decimal(a)) = inputs[1].value.try_convert_to(ValueType::Decimal) else { return Err(OperationError { message: "Unable to convert to integer.".to_string() })};
        let Ok(Value::Decimal(b)) = inputs[2].value.try_convert_to(ValueType::Decimal) else { return Err(OperationError { message: "Unable to convert to integer.".to_string() })};
        let Ok(Value::Decimal(alpha)) = inputs[3].value.try_convert_to(ValueType::Decimal) else { return Err(OperationError { message: "Unable to convert to integer.".to_string() })};

        let color = Color::from_lab(l, a, b, alpha);

        Ok(OperationResponse {
            time: Instant::now().duration_since(start_time),
            responses: vec![OutputResponse {
                value: Value::Color(color),
            }],
        })
    }
}
