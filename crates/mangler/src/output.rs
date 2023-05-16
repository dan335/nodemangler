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
                    let mut p = nodes.get_mut(node_id);
                    if let Some(node) = p {
                        if &node.inputs.len() > input_index {
                            match (&self.value, &node.inputs[*input_index].value) {
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
                                    node.inputs[*input_index].value = Value::Integer(*o as usize);
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
                            }
                        }
                    }
                }
            }
        }
    }
}
