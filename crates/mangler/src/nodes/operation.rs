use core::fmt::Debug;
use std::time::Duration;

use crate::{
    input::Input,
    output::Output,
    value::{Value, ValueType},
};

pub trait Operation {
    fn run(&mut self, inputs: &Vec<Input>, outputs: &mut Vec<Output>) -> Duration;
    //fn clone(&self) -> Self;
}

impl Debug for dyn Operation {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("<Operation>")
    }
}

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
