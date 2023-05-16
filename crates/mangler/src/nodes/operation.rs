use crate::nodes::*;
use core::fmt::Debug;
use std::time::Duration;

use crate::{
    input::Input,
    output::Output,
    value::{Value, ValueType},
};

#[derive(Debug, Clone)]
pub enum Operation {
    Add(add::Add),
    Subtract(subtract::Subtract),
    Float(float::Float),
    Integer(integer::Integer),
}

impl Operation {
    pub fn run(&mut self, inputs: &Vec<Input>, outputs: &mut Vec<Output>) -> Duration {
        match self {
            Operation::Float(operation) => operation.run(inputs, outputs),
            Operation::Integer(operation) => operation.run(inputs, outputs),
            Operation::Add(operation) => operation.run(inputs, outputs),
            Operation::Subtract(operation) => operation.run(inputs, outputs),
        }
    }
}

#[derive(Debug, Clone)]
pub struct ConnectionSettings {
    pub name: String,
    pub default_value: Value,
    pub valid_types: Vec<ValueType>,
    pub ui_type: Option<UiType>, // for output connections it's none
}

#[derive(Debug, Clone)]
pub enum UiType {
    DragValue,
    Checkbox,
    Slider,
    TextEdit,
}
