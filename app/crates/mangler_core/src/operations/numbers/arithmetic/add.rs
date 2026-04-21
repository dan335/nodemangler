//! Addition operation for the node graph.
//!
//! Performs polymorphic addition across value types: numbers are summed,
//! booleans act as 0/1, strings are concatenated, and colors/images have
//! the scalar added to each channel.

use crate::color::Color;
use crate::get_id;
use crate::input::{Input, InputSettings};
use crate::node_settings::NodeSettings;
use crate::operations::{OperationResponse, OperationError, OutputResponse};
use crate::output::Output;
use crate::value::Value;
use serde::{Deserialize, Serialize};
use std::time::Instant;

/// Node operation that adds two values together.
///
/// Supports many type combinations: numeric (integer/decimal), boolean (treated
/// as 0 or 1), string (concatenation), color (per-channel addition), and image
/// (per-pixel scalar addition). Mixed numeric types promote to decimal.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpNumberMathAdd {}

impl OpNumberMathAdd {
    /// Returns the node metadata (name and description).
    pub fn settings() -> NodeSettings {
        NodeSettings {
            name: "add".to_string(),
            description: "Adds two numbers together.".to_string(),
        }
    }

    /// Creates the default input list: two decimal drag-value inputs (a and b).
    pub fn create_inputs() -> Vec<Input> {
        vec![
            Input::new("a".to_string(), Value::Decimal(0.0), Some(InputSettings::DragValue { speed:None, clamp:None }), None),
            Input::new("b".to_string(), Value::Decimal(0.0), Some(InputSettings::DragValue { speed:None, clamp:None }), None),
        ]
    }

    /// Creates the default output list: a single decimal output.
    pub fn create_outputs() -> Vec<Output> {
        vec![
            Output::new("output".to_string(), Value::Decimal(0.0), None)
        ]
    }

    /// Executes the addition. Dispatches on the type combination of inputs a and b.
    pub async fn run(inputs: &mut [Input]) -> Result<OperationResponse, OperationError> {
        let start_time = Instant::now();
        let input_errors: Vec<(usize, String)> = vec![];

        // convert inputs
        // gather errors

        // return if error
        if !input_errors.is_empty() { return Err(OperationError { input_errors, node_error: None }); }

        // get values
        // run node

        let value = match &inputs[0].value {
            Value::Bool(a) => {
                match &inputs[1].value {
                    Value::Bool(b) => {
                        Value::Bool(*a || *b)
                    },
                    Value::Integer(b) => {
                        if *a {
                            Value::Integer(*b + 1)
                        } else {
                            Value::Integer(*b)
                        }
                    },
                    Value::Decimal(b) => {
                        if *a {
                            Value::Decimal(*b + 1.0)
                        } else {
                            Value::Decimal(*b)
                        }
                    },
                    Value::Image { data: image_b, change_id: _ } => {
                        // Add 1.0 to all channels if boolean is true
                        let mut result = (**image_b).clone();
                        if *a {
                            for pixel in result.pixels_mut() {
                                for c in 0..pixel.len() { pixel[c] += 1.0; }
                            }
                        }
                        Value::Image { data: std::sync::Arc::new(result), change_id: get_id() }
                    },
                    Value::Text(b) => {
                        Value::Text(format!("{}{}", a, b))
                    },
                    Value::Color(b) => {
                        let rgba = b.to_srgb_float();
                        if *a {
                            Value::Color(Color::from_srgb_float(rgba.0 + 1.0, rgba.1 + 1.0, rgba.2 + 1.0, rgba.3 + 1.0))
                        } else {
                            Value::Color(*b)
                        }
                    },
                    _ => {return Err(OperationError {
                        input_errors: vec![
                            (1, "Unsupported type for 'b' in add operation.".to_string())
                        ],
                        node_error: None
                    }); }
                }
            },
            Value::Integer(a) => {
                match &inputs[1].value {
                    Value::Bool(b) => {
                        if *b {
                            Value::Integer(*a + 1)
                        } else {
                            Value::Integer(*a)
                        }
                    },
                    Value::Integer(b) => {
                        Value::Integer(*a + *b)
                    },
                    Value::Decimal(b) => {
                        Value::Decimal(*a as f32 + *b)
                    },
                    Value::Image { data: image_b, change_id: _ } => {
                        // Add integer value to all channels
                        let mut result = (**image_b).clone();
                        let val = *a as f32;
                        for pixel in result.pixels_mut() {
                            for c in 0..pixel.len() { pixel[c] += val; }
                        }
                        Value::Image { data: std::sync::Arc::new(result), change_id: get_id() }
                    },
                    Value::Text(b) => {
                        Value::Text(format!("{}{}", a, b))
                    },
                    Value::Color(b) => {
                        let rgba = b.to_srgb_float();
                        Value::Color(Color::from_srgb_float(rgba.0 + *a as f32, rgba.1 + *a as f32, rgba.2 + *a as f32, rgba.3 + *a as f32))
                    },
                    _ => {return Err(OperationError {
                        input_errors: vec![
                            (1, "Unsupported type for 'b' in add operation.".to_string())
                        ],
                        node_error: None
                    }); }
                }
            },
            Value::Decimal(a) => {
                match &inputs[1].value {
                    Value::Bool(b) => {
                        if *b {
                            Value::Decimal(*a + 1.0)
                        } else {
                            Value::Decimal(*a)
                        }
                    },
                    Value::Integer(b) => {
                        Value::Decimal(*a + *b as f32)
                    },
                    Value::Decimal(b) => {
                        Value::Decimal(*a + *b)
                    },
                    Value::Image { data: image_b, change_id: _ } => {
                        // Add decimal value to all channels
                        let mut result = (**image_b).clone();
                        for pixel in result.pixels_mut() {
                            for c in 0..pixel.len() { pixel[c] += *a; }
                        }
                        Value::Image { data: std::sync::Arc::new(result), change_id: get_id() }
                    },
                    Value::Text(b) => {
                        Value::Text(format!("{}{}", a, b))
                    },
                    Value::Color(b) => {
                        let rgba = b.to_srgb_float();
                        Value::Color(Color::from_srgb_float(rgba.0 + *a, rgba.1 + *a, rgba.2 + *a, rgba.3 + *a))
                    },
                    _ => {return Err(OperationError {
                        input_errors: vec![
                            (1, "Unsupported type for 'b' in add operation.".to_string())
                        ],
                        node_error: None
                    }); }
                }
            },
            Value::Image { data: image_a, change_id: _ } => {
                match &inputs[1].value {
                    Value::Bool(b) => {
                        // Add 1.0 to all channels if boolean is true
                        let mut result = (**image_a).clone();
                        if *b {
                            for pixel in result.pixels_mut() {
                                for c in 0..pixel.len() { pixel[c] += 1.0; }
                            }
                        }
                        Value::Image { data: std::sync::Arc::new(result), change_id: get_id() }
                    },
                    Value::Integer(b) => {
                        // Add integer value to all channels
                        let mut result = (**image_a).clone();
                        let val = *b as f32;
                        for pixel in result.pixels_mut() {
                            for c in 0..pixel.len() { pixel[c] += val; }
                        }
                        Value::Image { data: std::sync::Arc::new(result), change_id: get_id() }
                    },
                    Value::Decimal(_b) => todo!(),
                    Value::Image { data: _image_b, change_id: _change_id } => todo!(),
                    Value::Color(_b) => todo!(),
                    _ => {return Err(OperationError {
                        input_errors: vec![
                            (1, "Unsupported type for 'b' in add operation.".to_string())
                        ],
                        node_error: None
                    }); }
                }
            },
            Value::Text(_a) => {
                match &inputs[1].value {
                    Value::Bool(_b) => todo!(),
                    Value::Integer(_b) => todo!(),
                    Value::Decimal(_b) => todo!(),
                    Value::Text(_b) => todo!(),
                    Value::Color(_b) => todo!(),
                    Value::Path(_b) => todo!(),
                    _ => {return Err(OperationError {
                        input_errors: vec![
                            (1, "Unsupported type for 'b' in add operation.".to_string())
                        ],
                        node_error: None
                    }); }
                }
            },
            Value::Path(_a) => {
                match &inputs[1].value {
                    Value::Bool(_b) => todo!(),
                    Value::Integer(_b) => todo!(),
                    Value::Decimal(_b) => todo!(),
                    Value::Text(_b) => todo!(),
                    Value::Path(_b) => todo!(),
                    _ => {return Err(OperationError {
                        input_errors: vec![
                            (1, "Unsupported type for 'b' in add operation.".to_string())
                        ],
                        node_error: None
                    }); }
                }
            },
            _ => {return Err(OperationError {
                input_errors: vec![
                    (0, "Unsupported type for 'a' in add operation.".to_string())
                ],
                node_error: None
            }); }
        };

        Ok(OperationResponse { 
            time: Instant::now().duration_since(start_time),
            responses: vec![OutputResponse {
                value,
            }],
        })
    }
}

#[cfg(test)]
#[path = "add_tests.rs"]
mod tests;
