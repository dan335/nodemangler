use crate::color::Color;
use crate::get_id;
use crate::input::{Input, InputSettings};
use crate::node_settings::NodeSettings;
use crate::operations::{OperationResponse, OperationError, OutputResponse};
use crate::output::Output;
use crate::value::{Value, ValueType};
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
            Input::new("b".to_string(), Value::Decimal(0.0), Some(InputSettings::DragValue { speed:None, clamp:None }), None),
        ]
    }

    pub fn create_outputs() -> Vec<Output> {
        vec![
            Output::new("output".to_string(), Value::Decimal(0.0), None)
        ]
    }

    pub async fn run(inputs: &mut Vec<Input>) -> Result<OperationResponse, OperationError> {
        let start_time = Instant::now();
        let input_errors: Vec<(usize, String)> = vec![];

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
                                *pixel = image::Rgba([pixel.0[0] + 1.0, pixel.0[1] + 1.0, pixel.0[2] + 1.0, pixel.0[3] + 1.0]);
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
                    Value::DynamicImage { data: image_b, change_id: _ } => {
                        for (_x, _y, pixel) in image_b.to_rgba32f().enumerate_pixels_mut() {
                            *pixel = image::Rgba([pixel.0[0] + *a as f32, pixel.0[1] + *a as f32, pixel.0[2] + *a as f32, pixel.0[3] + *a as f32]);
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
                    Value::DynamicImage { data: image_b, change_id: _ } => {
                        for (_x, _y, pixel) in image_b.to_rgba32f().enumerate_pixels_mut() {
                            *pixel = image::Rgba([pixel.0[0] + *a, pixel.0[1] + *a, pixel.0[2] + *a, pixel.0[3] + *a]);
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
                            (1, "Unsupported type for 'b' in add operation.".to_string())
                        ],
                        node_error: None
                    }); }
                }
            },
            Value::DynamicImage { data: image_a, change_id: _ } => {
                match &inputs[1].value {
                    Value::Bool(b) => {
                        for (_x, _y, pixel) in image_a.to_rgba32f().enumerate_pixels_mut() {
                            if *b {
                                *pixel = image::Rgba([pixel.0[0] + 1.0, pixel.0[1] + 1.0, pixel.0[2] + 1.0, pixel.0[3] + 1.0]);
                            }
                        }

                        Value::DynamicImage { data: image_a.clone(), change_id: get_id() }
                    },
                    Value::Integer(b) => {
                        for (_x, _y, pixel) in image_a.to_rgba32f().enumerate_pixels_mut() {
                            *pixel = image::Rgba([pixel.0[0] + *b as f32, pixel.0[1] + *b as f32, pixel.0[2] + *b as f32, pixel.0[3] + *b as f32]);
                        }

                        Value::DynamicImage { data: image_a.clone(), change_id: get_id() }
                    },
                    Value::Decimal(_b) => todo!(),
                    Value::DynamicImage { data: _image_b, change_id: _change_id } => todo!(),
                    Value::Color(_b) => todo!(),
                    _ => {return Err(OperationError {
                        input_errors: vec![
                            (1, "Unsupported type for 'b' in add operation.".to_string())
                        ],
                        node_error: None
                    }); }
                }
            },
            Value::String(_a) => {
                match &inputs[1].value {
                    Value::Bool(_b) => todo!(),
                    Value::Integer(_b) => todo!(),
                    Value::Decimal(_b) => todo!(),
                    Value::String(_b) => todo!(),
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
                    Value::String(_b) => todo!(),
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
                value: value,
            }],
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    macro_rules! assert_value {
        ($val:expr, Integer($expected:expr)) => {
            match &$val { Value::Integer(v) => assert_eq!(*v, $expected), other => panic!("Expected Integer({}), got {:?}", $expected, other) }
        };
        ($val:expr, Decimal($expected:expr)) => {
            match &$val { Value::Decimal(v) => assert!((*v - $expected).abs() < 1e-6, "Expected Decimal({}), got Decimal({})", $expected, v), other => panic!("Expected Decimal({}), got {:?}", $expected, other) }
        };
        ($val:expr, Bool($expected:expr)) => {
            match &$val { Value::Bool(v) => assert_eq!(*v, $expected), other => panic!("Expected Bool({}), got {:?}", $expected, other) }
        };
        ($val:expr, String($expected:expr)) => {
            match &$val { Value::String(v) => assert_eq!(v, $expected), other => panic!("Expected String, got {:?}", other) }
        };
    }

    fn make_inputs(a: Value, b: Value) -> Vec<Input> {
        vec![
            Input::new("a".to_string(), a, None, None),
            Input::new("b".to_string(), b, None, None),
        ]
    }

    #[tokio::test]
    async fn test_add_decimal_decimal() {
        let mut inputs = make_inputs(
            Value::Decimal(5.0),
            Value::Decimal(10.0),
        );
        let result = OpNumberMathAdd::run(&mut inputs).await.unwrap();
        assert_value!(result.responses[0].value, Decimal(15.0));
    }

    #[tokio::test]
    async fn test_add_integer_integer() {
        let mut inputs = make_inputs(
            Value::Integer(5),
            Value::Integer(10),
        );
        let result = OpNumberMathAdd::run(&mut inputs).await.unwrap();
        assert_value!(result.responses[0].value, Integer(15));
    }

    #[tokio::test]
    async fn test_add_integer_decimal() {
        let mut inputs = make_inputs(
            Value::Integer(5),
            Value::Decimal(2.5),
        );
        let result = OpNumberMathAdd::run(&mut inputs).await.unwrap();
        assert_value!(result.responses[0].value, Decimal(7.5));
    }

    #[tokio::test]
    async fn test_add_decimal_integer() {
        let mut inputs = make_inputs(
            Value::Decimal(2.5),
            Value::Integer(5),
        );
        let result = OpNumberMathAdd::run(&mut inputs).await.unwrap();
        assert_value!(result.responses[0].value, Decimal(7.5));
    }

    #[tokio::test]
    async fn test_add_bool_true_integer() {
        let mut inputs = make_inputs(
            Value::Bool(true),
            Value::Integer(5),
        );
        let result = OpNumberMathAdd::run(&mut inputs).await.unwrap();
        assert_value!(result.responses[0].value, Integer(6));
    }

    #[tokio::test]
    async fn test_add_bool_false_integer() {
        let mut inputs = make_inputs(
            Value::Bool(false),
            Value::Integer(5),
        );
        let result = OpNumberMathAdd::run(&mut inputs).await.unwrap();
        assert_value!(result.responses[0].value, Integer(5));
    }

    #[tokio::test]
    async fn test_add_bool_bool() {
        let mut inputs = make_inputs(
            Value::Bool(true),
            Value::Bool(false),
        );
        let result = OpNumberMathAdd::run(&mut inputs).await.unwrap();
        assert_value!(result.responses[0].value, Bool(true));
    }

    #[tokio::test]
    async fn test_add_bool_decimal() {
        let mut inputs = make_inputs(
            Value::Bool(true),
            Value::Decimal(5.5),
        );
        let result = OpNumberMathAdd::run(&mut inputs).await.unwrap();
        assert_value!(result.responses[0].value, Decimal(6.5));
    }

    #[tokio::test]
    async fn test_add_integer_bool_true() {
        let mut inputs = make_inputs(
            Value::Integer(10),
            Value::Bool(true),
        );
        let result = OpNumberMathAdd::run(&mut inputs).await.unwrap();
        assert_value!(result.responses[0].value, Integer(11));
    }

    #[tokio::test]
    async fn test_add_decimal_bool_true() {
        let mut inputs = make_inputs(
            Value::Decimal(10.0),
            Value::Bool(true),
        );
        let result = OpNumberMathAdd::run(&mut inputs).await.unwrap();
        assert_value!(result.responses[0].value, Decimal(11.0));
    }

    #[tokio::test]
    async fn test_add_decimal_zero() {
        let mut inputs = make_inputs(
            Value::Decimal(0.0),
            Value::Decimal(0.0),
        );
        let result = OpNumberMathAdd::run(&mut inputs).await.unwrap();
        assert_value!(result.responses[0].value, Decimal(0.0));
    }

    #[tokio::test]
    async fn test_add_negative_numbers() {
        let mut inputs = make_inputs(
            Value::Integer(-5),
            Value::Integer(-10),
        );
        let result = OpNumberMathAdd::run(&mut inputs).await.unwrap();
        assert_value!(result.responses[0].value, Integer(-15));
    }

    #[tokio::test]
    async fn test_add_string_concat() {
        let mut inputs = make_inputs(
            Value::Bool(true),
            Value::String("hello".to_string()),
        );
        let result = OpNumberMathAdd::run(&mut inputs).await.unwrap();
        assert_value!(result.responses[0].value, String("truehello"));
    }

    #[tokio::test]
    async fn test_add_integer_string_concat() {
        let mut inputs = make_inputs(
            Value::Integer(42),
            Value::String("hello".to_string()),
        );
        let result = OpNumberMathAdd::run(&mut inputs).await.unwrap();
        assert_value!(result.responses[0].value, String("42hello"));
    }

    #[tokio::test]
    async fn test_add_settings() {
        let settings = OpNumberMathAdd::settings();
        assert_eq!(settings.name, "add");
    }

    #[tokio::test]
    async fn test_add_create_inputs_count() {
        let inputs = OpNumberMathAdd::create_inputs();
        assert_eq!(inputs.len(), 2);
    }

    #[tokio::test]
    async fn test_add_create_outputs_count() {
        let outputs = OpNumberMathAdd::create_outputs();
        assert_eq!(outputs.len(), 1);
    }
}
