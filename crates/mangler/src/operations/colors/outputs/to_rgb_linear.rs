use crate::color::Color;
use crate::input::{Input, InputSettings};
use crate::node_settings::NodeSettings;
use crate::operations::{OperationResponse, OperationError, OutputResponse};
use crate::output::Output;
use crate::value::{Value, ValueType};
use serde::{Deserialize, Serialize};
use std::time::Instant;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpColorOutputRgbLinear {}

impl OpColorOutputRgbLinear {
    pub fn settings() -> NodeSettings {
        NodeSettings {
            name: "to rgb linear".to_string(),
            description: "Converts a color to the RGB linear color space.".to_string(),
        }
    }

    pub fn create_inputs() -> Vec<Input> {
        vec![
            Input::new("input".to_string(), Value::Color(Color::default()), None, None),
        ]
    }

    pub fn create_outputs() -> Vec<Output> {
        vec![
            Output::new("red".to_string(), Value::Decimal(0.5), None),
            Output::new("green".to_string(), Value::Decimal(0.5), None),
            Output::new("blue".to_string(), Value::Decimal(0.5), None),
            Output::new("alpha".to_string(), Value::Decimal(1.0), None),
        ]
    }

    pub async fn run(inputs: &mut Vec<Input>) -> Result<OperationResponse, OperationError> {
        let start_time = Instant::now();
        let mut input_errors: Vec<(usize, String)> = vec![];

        // convert inputs
        let color_converted = inputs[0].value.try_convert_to(ValueType::Color);

        // gather errors
        if color_converted.is_err() { input_errors.push((0, color_converted.as_ref().err().unwrap().message.clone())); }

        // return if error
        if input_errors.len() > 0 { return Err(OperationError { input_errors, node_error: None }); }

        // get values
        let Ok(Value::Color(color)) = color_converted else { return Err(OperationError { input_errors, node_error: Some("Error converting.".to_string()) }); };

        let (r, g, b, a) = color.to_rgb_linear();

        Ok(OperationResponse {
            time: Instant::now().duration_since(start_time),
            responses: vec![
                OutputResponse {value: Value::Decimal(r)},
                OutputResponse {value: Value::Decimal(g)},
                OutputResponse {value: Value::Decimal(b)},
                OutputResponse {value: Value::Decimal(a)},
            ],
        })
    }
}
