//! Addition operation for the node graph.
//!
//! Performs polymorphic addition across value types: numbers are summed,
//! booleans act as 0 or 1, scalars prepend to text, and colors/images have
//! scalars, colors, or matching images added per channel.

use crate::color::Color;
use crate::float_image::FloatImage;
use crate::get_id;
use crate::input::{Input, InputSettings};
use crate::node_settings::NodeSettings;
use crate::operations::{OperationResponse, OperationError, OutputResponse};
use crate::output::Output;
use crate::value::Value;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use std::time::Instant;

/// Builds the `OperationError` shape used for a single bad input.
fn unsupported(input_index: usize, message: &str) -> OperationError {
    OperationError {
        input_errors: vec![(input_index, message.to_string())],
        node_error: None,
    }
}

/// Adds `amount` to every channel of every pixel, returning a new image value
/// with a fresh change id.
fn add_scalar_to_image(image: &FloatImage, amount: f32) -> Value {
    let mut result = image.clone();
    for pixel in result.pixels_mut() {
        for c in pixel.iter_mut() {
            *c += amount;
        }
    }
    Value::Image { data: Arc::new(result), change_id: get_id() }
}

/// Adds two images per pixel, per channel. Dimensions and channel counts must
/// match; otherwise an `OperationError` describing the mismatch is returned.
fn add_images(a: &FloatImage, b: &FloatImage) -> Result<Value, OperationError> {
    if a.dimensions() != b.dimensions() {
        return Err(unsupported(1, &format!(
            "Image dimensions must match to add images: 'a' is {}x{} but 'b' is {}x{}.",
            a.width(), a.height(), b.width(), b.height()
        )));
    }
    if a.channels() != b.channels() {
        return Err(unsupported(1, &format!(
            "Image channel counts must match to add images: 'a' has {} channel(s) but 'b' has {} channel(s).",
            a.channels(), b.channels()
        )));
    }
    let mut result = a.clone();
    for (dst, src) in result.as_raw_mut().iter_mut().zip(b.as_raw()) {
        *dst += *src;
    }
    Ok(Value::Image { data: Arc::new(result), change_id: get_id() })
}

/// Adds a color's sRGBA components to every pixel of an image. Channel layouts
/// follow the repo convention: 1 = gray, 2 = gray+alpha, 3 = RGB, 4 = RGBA;
/// grayscale channels receive the color's RGB average.
fn add_color_to_image(image: &FloatImage, color: &Color) -> Value {
    let (r, g, b, a) = color.to_srgb_float();
    let luma = (r + g + b) / 3.0;
    let mut result = image.clone();
    let addend: [f32; 4] = match result.channels() {
        1 => [luma, 0.0, 0.0, 0.0],
        2 => [luma, a, 0.0, 0.0],
        3 => [r, g, b, 0.0],
        _ => [r, g, b, a],
    };
    for pixel in result.pixels_mut() {
        for (c, add) in pixel.iter_mut().zip(addend.iter()) {
            *c += *add;
        }
    }
    Value::Image { data: Arc::new(result), change_id: get_id() }
}

/// Node operation that adds two values together.
///
/// Supports many type combinations: numeric (integer/decimal), boolean (treated
/// as 0 or 1), color (per-channel addition), and image (per-pixel addition of
/// scalars, colors, or a matching image). Mixed numeric types promote to
/// decimal; a scalar 'a' concatenates in front of a text 'b'.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpNumberMathAdd {}

impl OpNumberMathAdd {
    /// Returns the node metadata (name and description).
    pub fn settings() -> NodeSettings {
        NodeSettings {
            name: "add".to_string(),
            description: "Adds two numbers together.".to_string(),
            help: "Polymorphic addition that dispatches on the types of both inputs. Numbers are summed and mixed integer/decimal types promote to decimal. Booleans act as 0 or 1, so true + true is the integer 2.\n\nA number or boolean 'a' concatenates in front of a text 'b' (e.g. 42 + \"px\" becomes \"42px\"); text is not supported as input 'a'. Colors add the scalar to every sRGBA channel. Images accept a scalar (added to every pixel channel), a color (added per channel), or another image with matching dimensions and channel counts (added per pixel).".to_string(),
        }
    }

    /// Creates the default input list: two decimal drag-value inputs (a and b).
    pub fn create_inputs() -> Vec<Input> {
        vec![
            Input::new("a".to_string(), Value::Decimal(0.0), Some(InputSettings::DragValue { speed:None, clamp:None }), None)
                .with_description("First operand; accepts numbers, booleans, or images."),
            Input::new("b".to_string(), Value::Decimal(0.0), Some(InputSettings::DragValue { speed:None, clamp:None }), None)
                .with_description("Second operand added to a; type combines with a to choose the result type."),
        ]
    }

    /// Creates the default output list: a single decimal output.
    pub fn create_outputs() -> Vec<Output> {
        vec![
            Output::new("output".to_string(), Value::Decimal(0.0), None)
                .with_description("Sum of a and b, promoted or concatenated based on input types.")
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
                        Value::Integer(*a as i32 + *b as i32)
                    },
                    Value::Integer(b) => {
                        Value::Integer(*b + *a as i32)
                    },
                    Value::Decimal(b) => {
                        Value::Decimal(*b + if *a { 1.0 } else { 0.0 })
                    },
                    Value::Image { data: image_b, change_id: _ } => {
                        add_scalar_to_image(image_b, if *a { 1.0 } else { 0.0 })
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
                    _ => { return Err(unsupported(1, "Unsupported type for 'b' in add operation.")); }
                }
            },
            Value::Integer(a) => {
                match &inputs[1].value {
                    Value::Bool(b) => {
                        Value::Integer(*a + *b as i32)
                    },
                    Value::Integer(b) => {
                        Value::Integer(*a + *b)
                    },
                    Value::Decimal(b) => {
                        Value::Decimal(*a as f32 + *b)
                    },
                    Value::Image { data: image_b, change_id: _ } => {
                        add_scalar_to_image(image_b, *a as f32)
                    },
                    Value::Text(b) => {
                        Value::Text(format!("{}{}", a, b))
                    },
                    Value::Color(b) => {
                        let rgba = b.to_srgb_float();
                        Value::Color(Color::from_srgb_float(rgba.0 + *a as f32, rgba.1 + *a as f32, rgba.2 + *a as f32, rgba.3 + *a as f32))
                    },
                    _ => { return Err(unsupported(1, "Unsupported type for 'b' in add operation.")); }
                }
            },
            Value::Decimal(a) => {
                match &inputs[1].value {
                    Value::Bool(b) => {
                        Value::Decimal(*a + if *b { 1.0 } else { 0.0 })
                    },
                    Value::Integer(b) => {
                        Value::Decimal(*a + *b as f32)
                    },
                    Value::Decimal(b) => {
                        Value::Decimal(*a + *b)
                    },
                    Value::Image { data: image_b, change_id: _ } => {
                        add_scalar_to_image(image_b, *a)
                    },
                    Value::Text(b) => {
                        Value::Text(format!("{}{}", a, b))
                    },
                    Value::Color(b) => {
                        let rgba = b.to_srgb_float();
                        Value::Color(Color::from_srgb_float(rgba.0 + *a, rgba.1 + *a, rgba.2 + *a, rgba.3 + *a))
                    },
                    _ => { return Err(unsupported(1, "Unsupported type for 'b' in add operation.")); }
                }
            },
            Value::Image { data: image_a, change_id: _ } => {
                match &inputs[1].value {
                    Value::Bool(b) => {
                        add_scalar_to_image(image_a, if *b { 1.0 } else { 0.0 })
                    },
                    Value::Integer(b) => {
                        add_scalar_to_image(image_a, *b as f32)
                    },
                    Value::Decimal(b) => {
                        add_scalar_to_image(image_a, *b)
                    },
                    Value::Image { data: image_b, change_id: _ } => {
                        add_images(image_a, image_b)?
                    },
                    Value::Color(b) => {
                        add_color_to_image(image_a, b)
                    },
                    _ => { return Err(unsupported(1, "Unsupported type for 'b' in add operation.")); }
                }
            },
            Value::Text(_) => {
                return Err(unsupported(0, "Text is not supported for 'a' in add operation. Connect the text to input 'b' to concatenate it after a number or boolean."));
            },
            Value::Path(_) => {
                return Err(unsupported(0, "Path values cannot be added; unsupported type for 'a' in add operation."));
            },
            _ => { return Err(unsupported(0, "Unsupported type for 'a' in add operation.")); }
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
