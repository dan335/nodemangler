use crate::color::Color;
use crate::input::Input;
use crate::node_settings::NodeSettings;
use crate::operations::{OperationResponse, OperationError, OutputResponse, convert_input};
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

    pub async fn run(inputs: &mut Vec<Input>) -> Result<OperationResponse, OperationError> {
        let start_time = Instant::now();
        let mut input_errors: Vec<(usize, String)> = vec![];

        // convert inputs
        let color_converted = convert_input(inputs, 0, ValueType::Color, &mut input_errors);


        // return if error
        if input_errors.len() > 0 { return Err(OperationError { input_errors, node_error: None }); }

        // get values
        let Value::Color(color) = color_converted.unwrap() else { unreachable!() };

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
