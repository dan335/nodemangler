use crate::nodes::*;
use core::fmt::Debug;
use std::{time::Duration, thread::{self, JoinHandle}};

use crate::{
    input::Input,
    output::Output,
    value::{Value, ValueType},
};

use super::{image_from_url::image_from_url, float::new_float, integer::new_integer, subtract::subtract, add::add, image_resize::image_resize};

#[derive(Debug, Clone)]
pub enum Operation {
    Add,
    Subtract,
    Float,
    Integer,
    ImageFromUrl,
    ImageResize,
}

impl Operation {
    pub fn run(&mut self, inputs: &Vec<Input>, outputs: &mut Vec<Output>) -> JoinHandle<OperationResponse> {
        let handle = thread::spawn(|| {
            match self {
                Operation::Float => new_float(inputs, outputs),
                Operation::Integer => new_integer(inputs, outputs),
                Operation::Add => add(inputs, outputs),
                Operation::Subtract => subtract(inputs, outputs),
                Operation::ImageFromUrl => image_from_url(inputs, outputs),
                Operation::ImageResize => image_resize(inputs, outputs),
            }
        });
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
    ComboBox,
}


pub struct OperationResponse {
    pub output_values: Vec<Value>,
    pub time: Duration,
}

impl OperationResponse {
    pub fn new() -> OperationResponse {
        OperationResponse { output_values: Vec::new(), time: Duration::default() }
    }
}
