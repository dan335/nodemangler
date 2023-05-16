use image::{DynamicImage, RgbaImage};

use crate::nodes::node::Node;
use crate::value::Value;
use std::collections::HashMap;

#[derive(Debug, Clone)]
pub struct Output {
    pub name: String,
    pub value: Value,
    pub connection: Option<Vec<(String, usize)>>, // id of input node, index of input
}

impl Output {
    pub fn new(name: String, value: Value) -> Output {
        Output {
            name,
            value,
            connection: None,
        }
    }

    pub fn pass_value_to_connections(&self, nodes: &mut HashMap<String, Node>) {
        if let Some(connections) = &self.connection {
            for (node_id, input_index) in connections.iter() {
                if nodes.contains_key(node_id) {
                    let p = nodes.get_mut(node_id);
                    if let Some(node) = p {
                        if &node.inputs.len() > input_index {
                            match (&self.value, &node.inputs[*input_index].value) {

                                // math
                                (Value::Integer(o), Value::Integer(_)) => {
                                    node.inputs[*input_index].value = Value::Integer(*o);
                                }
                                (Value::Integer(o), Value::Decimal(_)) => {
                                    node.inputs[*input_index].value = Value::Decimal(*o as f32);
                                }
                                (Value::Integer(o), Value::String(_)) => {
                                    node.inputs[*input_index].value = Value::String(o.to_string());
                                }
                                (Value::Decimal(o), Value::Integer(_)) => {
                                    node.inputs[*input_index].value = Value::Integer(*o as i32);
                                }
                                (Value::Decimal(o), Value::Decimal(_)) => {
                                    node.inputs[*input_index].value = Value::Decimal(*o);
                                }
                                (Value::Decimal(o), Value::String(_)) => {
                                    node.inputs[*input_index].value = Value::String(o.to_string());
                                }
                                (Value::String(o), Value::Integer(_)) => {
                                    node.inputs[*input_index].value =
                                        Value::Integer(o.parse().unwrap_or(0));
                                }
                                (Value::String(o), Value::Decimal(_)) => {
                                    node.inputs[*input_index].value =
                                        Value::Decimal(o.parse().unwrap_or(0.0));
                                }
                                (Value::String(o), Value::String(_)) => {
                                    node.inputs[*input_index].value = Value::String(o.clone());
                                }
                                 
                                // image
                                (Value::ImageRgba32F(o), Value::ImageRgba32F(_)) =>  {
                                    node.inputs[*input_index].value = Value::ImageRgba32F(o.clone());
                                },
                                (Value::ImageRgba32F(o), Value::ImageRgba8(_)) => {
                                    let converted = DynamicImage::ImageRgba32F(o.clone()).to_rgba8();
                                    node.inputs[*input_index].value = Value::ImageRgba8(converted);
                                },
                                (Value::ImageRgba32F(o), Value::ImageGray8(_)) => {
                                    let converted = DynamicImage::ImageRgba32F(o.clone()).to_luma8();
                                    node.inputs[*input_index].value = Value::ImageGray8(converted);
                                },
                            
                                (Value::ImageRgba8(o), Value::ImageRgba32F(_)) => {
                                    let converted = DynamicImage::ImageRgba8(o.clone()).to_rgba32f();
                                    node.inputs[*input_index].value = Value::ImageRgba32F(converted);
                                },
                                (Value::ImageRgba8(o), Value::ImageRgba8(_)) => {
                                    node.inputs[*input_index].value = Value::ImageRgba8(o.clone());
                                },
                                (Value::ImageRgba8(o), Value::ImageGray8(_)) => {
                                    let converted = DynamicImage::ImageRgba8(o.clone()).to_luma8();
                                    node.inputs[*input_index].value = Value::ImageGray8(converted);
                                },
                                
                                (Value::ImageGray8(o), Value::ImageRgba32F(_)) => {
                                    let converted = DynamicImage::ImageLuma8(o.clone()).to_rgba32f();
                                    node.inputs[*input_index].value = Value::ImageRgba32F(converted);
                                },
                                (Value::ImageGray8(o), Value::ImageRgba8(_)) => {
                                    let converted = DynamicImage::ImageLuma8(o.clone()).to_rgba8();
                                    node.inputs[*input_index].value = Value::ImageRgba8(converted);
                                },
                                (Value::ImageGray8(o), Value::ImageGray8(_)) => {
                                    node.inputs[*input_index].value = Value::ImageGray8(o.clone());
                                },

                                _ => panic!("Unable to conver formats in output.rs.")
                            }
                        }
                    }
                }
            }
        }
    }
}
