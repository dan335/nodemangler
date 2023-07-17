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
            description: "Creates a color using the LAB color space.".to_string(),
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

    pub async fn run(inputs: &mut Vec<Input>) -> Result<OperationResponse, OperationError> {
        let start_time = Instant::now();
        let mut input_errors: Vec<(usize, String)> = vec![];

        // convert inputs
        let l_converted = inputs[0].value.try_convert_to(ValueType::Decimal);
        let a_converted = inputs[1].value.try_convert_to(ValueType::Decimal);
        let b_converted = inputs[2].value.try_convert_to(ValueType::Decimal);
        let alpha_converted = inputs[3].value.try_convert_to(ValueType::Decimal);

        // gather errors
        if l_converted.is_err() { input_errors.push((0, l_converted.as_ref().err().unwrap().message.clone())); }
        if a_converted.is_err() { input_errors.push((0, a_converted.as_ref().err().unwrap().message.clone())); }
        if b_converted.is_err() { input_errors.push((0, b_converted.as_ref().err().unwrap().message.clone())); }
        if alpha_converted.is_err() { input_errors.push((0, alpha_converted.as_ref().err().unwrap().message.clone())); }

        // return if error
        if input_errors.len() > 0 { return Err(OperationError { input_errors, node_error: None }); }

        // get values
        let Ok(Value::Decimal(l)) = l_converted else { return Err(OperationError { input_errors, node_error: Some("Error converting.".to_string()) }); };
        let Ok(Value::Decimal(a)) = a_converted else { return Err(OperationError { input_errors, node_error: Some("Error converting.".to_string()) }); };
        let Ok(Value::Decimal(b)) = b_converted else { return Err(OperationError { input_errors, node_error: Some("Error converting.".to_string()) }); };
        let Ok(Value::Decimal(alpha)) = alpha_converted else { return Err(OperationError { input_errors, node_error: Some("Error converting.".to_string()) }); };        
        
        // run node
        let color = Color::from_lab(l, a, b, alpha);

        Ok(OperationResponse {
            time: Instant::now().duration_since(start_time),
            responses: vec![OutputResponse {
                value: Value::Color(color),
            }],
        })
    }
}
