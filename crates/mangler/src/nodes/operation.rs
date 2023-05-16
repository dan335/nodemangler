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
    //fn run(&mut self, inputs: &Vec<Input>, outputs: &mut Vec<Output>) -> Duration;
    //fn clone(&self) -> Self;
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

// impl Debug for dyn Operation {
//     fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> std::fmt::Result {
//         f.write_str("<Operation>")
//     }
// }

// impl Clone for Box<dyn Operation> {
//     fn clone(&self) -> Self {
//         Self {}
//     }
// }

// impl Debug for Box<dyn Operation> {
//     fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
//         write!(f, "Series{{}}")
//     }
// }

// impl<T: Clone> Operation for T {

// }

// impl Clone for Box<dyn Operation> {
//     fn clone(&self) -> Self {
//         self.clone()
//     }
// }

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
