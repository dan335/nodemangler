use crate::color::Color;
use crate::input::{Input, InputSettings};
use crate::node_settings::NodeSettings;
use crate::operations::{OperationResponse, OperationError, OutputResponse};
use crate::output::Output;
use crate::value::{Value, ValueType};
use serde::{Deserialize, Serialize};
use std::time::Instant;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpColorInputXyz {}

impl OpColorInputXyz {
    pub fn settings() -> NodeSettings {
        NodeSettings {
            name: "xyz".to_string(),
            description: "Creates a color using the XYZ color space.".to_string(),
        }
    }

    pub fn create_inputs() -> Vec<Input> {
        vec![
            Input::new("x".to_string(), Value::Decimal(0.5), Some(InputSettings::Slider { range: (0.0, 1.0), step_by: Some(0.01), clamp_to_range: false }), None),
            Input::new("y".to_string(), Value::Decimal(0.0), Some(InputSettings::Slider { range: (-0.5, 0.5), step_by: Some(0.01), clamp_to_range: false }), None),
            Input::new("z".to_string(), Value::Decimal(0.0), Some(InputSettings::Slider { range: (-0.5, 0.5), step_by: Some(0.01), clamp_to_range: false }), None),
            Input::new("alpha".to_string(), Value::Decimal(1.0), Some(InputSettings::Slider { range: (0.0, 1.0), step_by: Some(0.01), clamp_to_range: true }), None),
        ]
    }

    pub fn create_outputs() -> Vec<Output> {
        vec![
            Output::new("output".to_string(), Value::Color(Color::default()), None)
        ]
    }

    pub async fn run(inputs: &mut Vec<Input>) -> Result<OperationResponse, OperationError> {
        let start_time = Instant::now();
        let mut input_errors: Vec<(usize, String)> = vec![];

        // convert inputs
        let x_converted = inputs[0].value.try_convert_to(ValueType::Decimal);
        let y_converted = inputs[1].value.try_convert_to(ValueType::Decimal);
        let z_converted = inputs[2].value.try_convert_to(ValueType::Decimal);
        let alpha_converted = inputs[3].value.try_convert_to(ValueType::Decimal);

        // gather errors
        if x_converted.is_err() { input_errors.push((0, x_converted.as_ref().err().unwrap().message.clone())); }
        if y_converted.is_err() { input_errors.push((0, y_converted.as_ref().err().unwrap().message.clone())); }
        if z_converted.is_err() { input_errors.push((0, z_converted.as_ref().err().unwrap().message.clone())); }
        if alpha_converted.is_err() { input_errors.push((0, alpha_converted.as_ref().err().unwrap().message.clone())); }

        // return if error
        if input_errors.len() > 0 { return Err(OperationError { input_errors, node_error: None }); }

        // get values
        let Ok(Value::Decimal(x)) = x_converted else { return Err(OperationError { input_errors, node_error: Some("Error converting.".to_string()) }); };
        let Ok(Value::Decimal(y)) = y_converted else { return Err(OperationError { input_errors, node_error: Some("Error converting.".to_string()) }); };
        let Ok(Value::Decimal(z)) = z_converted else { return Err(OperationError { input_errors, node_error: Some("Error converting.".to_string()) }); };
        let Ok(Value::Decimal(alpha)) = alpha_converted else { return Err(OperationError { input_errors, node_error: Some("Error converting.".to_string()) }); };

        // run node
        let color = Color::from_xyz(x, y, z, alpha);

        Ok(OperationResponse {
            time: Instant::now().duration_since(start_time),
            responses: vec![OutputResponse {
                value: Value::Color(color),
            }],
        })
    }
}
