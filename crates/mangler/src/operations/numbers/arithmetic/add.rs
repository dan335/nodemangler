use crate::color::Color;
use crate::get_id;
use crate::input::{Input, InputSettings};
use crate::node_settings::NodeSettings;
use crate::operations::{OperationResponse, OperationError, OutputResponse};
use crate::output::Output;
use crate::value::Value;
use serde::{Deserialize, Serialize};
use std::time::Instant;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpNumberMathAdd {}

impl OpNumberMathAdd {
    pub fn settings() -> NodeSettings {
        NodeSettings {
            name: "add".to_string(),
            description: "Adds two numbers together.".to_string(),
        }
    }

    pub fn create_inputs() -> Vec<Input> {
        vec![
            Input::new("a".to_string(), Value::Decimal(0.0), Some(InputSettings::DragValue { speed:None, clamp:None }), None),
            Input::new("b".to_string(), Value::Decimal(0.0), Some(InputSettings::DragValue { speed:None, clamp:None }), None)
        ]
    }

    pub fn create_outputs() -> Vec<Output> {
        vec![
            Output::new("output".to_string(), Value::Decimal(0.0), None)
        ]
    }

    pub async fn run(inputs: &mut Vec<Input>) -> Result<OperationResponse, OperationError> {
        let start_time = Instant::now();
        let mut input_errors: Vec<(usize, String)> = vec![];

        // convert inputs
        // gather errors

        // return if error
        if input_errors.len() > 0 { return Err(OperationError { input_errors, node_error: None }); }

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
                    Value::DynamicImage { data: image_b, change_id: _ } => {
                        for (_x, _y, pixel) in image_b.to_rgba32f().enumerate_pixels_mut() {
                            if *a {
                                *pixel = image::Rgba([pixel.0[0] + 1.0, pixel.0[1] + 1.0, pixel.0[3] + 1.0, pixel.0[4] + 1.0]);
                            }
                        }

                        Value::DynamicImage { data: image_b.clone(), change_id: get_id() }
                    },
                    Value::String(b) => {
                        Value::String(format!("{}{}", a.to_string(), b))
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
                            (1, "Error converting.".to_string())
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
                    Value::DynamicImage { data: image_b, change_id: _ } => {
                        for (_x, _y, pixel) in image_b.to_rgba32f().enumerate_pixels_mut() {
                            *pixel = image::Rgba([pixel.0[0] + *a as f32, pixel.0[1] + *a as f32, pixel.0[3] + *a as f32, pixel.0[4] + *a as f32]);
                        }

                        Value::DynamicImage { data: image_b.clone(), change_id: get_id() }
                    },
                    Value::String(b) => {
                        Value::String(format!("{}{}", a.to_string(), b))
                    },
                    Value::Color(b) => {
                        let rgba = b.to_srgb_float();
                        Value::Color(Color::from_srgb_float(rgba.0 + *a as f32, rgba.1 + *a as f32, rgba.2 + *a as f32, rgba.3 + *a as f32))
                    },
                    _ => {return Err(OperationError {
                        input_errors: vec![
                            (1, "Error converting.".to_string())
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
                    Value::DynamicImage { data: image_b, change_id: _ } => {
                        for (_x, _y, pixel) in image_b.to_rgba32f().enumerate_pixels_mut() {
                            *pixel = image::Rgba([pixel.0[0] + *a, pixel.0[1] + *a, pixel.0[3] + *a, pixel.0[4] + *a]);
                        }

                        Value::DynamicImage { data: image_b.clone(), change_id: get_id() }
                    },
                    Value::String(b) => {
                        Value::String(format!("{}{}", a.to_string(), b))
                    },
                    Value::Color(b) => {
                        let rgba = b.to_srgb_float();
                        Value::Color(Color::from_srgb_float(rgba.0 + *a, rgba.1 + *a, rgba.2 + *a, rgba.3 + *a))
                    },
                    _ => {return Err(OperationError {
                        input_errors: vec![
                            (1, "Error converting.".to_string())
                        ],
                        node_error: None
                    }); }
                }
            },
            Value::DynamicImage { data: image_a, change_id } => {
                match &inputs[1].value {
                    Value::Bool(b) => {
                        for (_x, _y, pixel) in image_a.to_rgba32f().enumerate_pixels_mut() {
                            if *a {
                                *pixel = image::Rgba([pixel.0[0] + 1.0, pixel.0[1] + 1.0, pixel.0[3] + 1.0, pixel.0[4] + 1.0]);
                            }
                        }

                        Value::DynamicImage { data: image_a.clone(), change_id: get_id() }
                    },
                    Value::Integer(b) => todo!(),
                    Value::Decimal(b) => todo!(),
                    Value::DynamicImage { data: image_b, change_id } => todo!(),
                    Value::Color(b) => todo!(),
                    _ => {return Err(OperationError {
                        input_errors: vec![
                            (1, "Error converting.".to_string())
                        ],
                        node_error: None
                    }); }
                }
            },
            Value::String(a) => {
                match &inputs[1].value {
                    Value::Bool(b) => todo!(),
                    Value::Integer(b) => todo!(),
                    Value::Decimal(b) => todo!(),
                    Value::String(b) => todo!(),
                    Value::Color(b) => todo!(),
                    Value::Path(b) => todo!(),
                    _ => {return Err(OperationError {
                        input_errors: vec![
                            (1, "Error converting.".to_string())
                        ],
                        node_error: None
                    }); }
                }
            },
            Value::Path(a) => {
                match &inputs[1].value {
                    Value::Bool(b) => todo!(),
                    Value::Integer(b) => todo!(),
                    Value::Decimal(b) => todo!(),
                    Value::String(b) => todo!(),
                    Value::Path(b) => todo!(),
                    _ => {return Err(OperationError {
                        input_errors: vec![
                            (1, "Error converting.".to_string())
                        ],
                        node_error: None
                    }); }
                }
            },
            _ => {return Err(OperationError {
                input_errors: vec![
                    (0, "Error converting.".to_string())
                ],
                node_error: None
            }); }
        };

        Ok(OperationResponse {
            time: Instant::now().duration_since(start_time),
            responses: vec![OutputResponse {
                value: value,
            }],
        })
    }
}
