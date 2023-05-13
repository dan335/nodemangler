use std::time::Duration;
use core::fmt::Debug;

use crate::{value::{Value, ValueType}, output::Output, input::Input};


pub trait Operation {
    fn run(&mut self, inputs: &Vec<Input>, outputs: &mut Vec<Output>) -> Duration;
}

impl Debug for Box<dyn Operation> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "Series{{}}")
    }
}

impl Clone for Box<dyn Operation> {
    fn clone(&self) -> Self {
        self.clone()
    }
}


#[derive(Debug, Clone)]
pub struct ConnectionSettings {
    pub name: String,
    pub default_value: Value,
    pub valid_types: Vec<ValueType>,
    pub ui_type: Option<UiType>,    // for output connections it's none
}

#[derive(Debug, Clone)]
pub enum UiType {
    DragValue,
    Checkbox,
    Slider,
    TextEdit,
}