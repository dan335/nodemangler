use crate::{value::ValueType};
use serde::{Deserialize, Serialize};
use crate::value::Value;
use core::fmt::Debug;
use std::time::Duration;

pub mod numbers;
pub mod images;
pub mod colors;

#[derive(Debug, Clone)]
pub struct ConnectionSettings {
    pub name: String,
    pub default_value: Value,
    pub valid_types: Vec<ValueType>,
    pub ui_type: Option<UiType>, // for output connections it's none
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum UiType {
    DragValue,
    Checkbox,
    Slider,
    TextEdit,
    ComboBox,
    UiButton,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OperationResponse {
    pub responses: Vec<OutputResponse>,
    pub time: Duration,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OutputResponse {
    pub value: Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OperationError {
    pub message: String,
}

#[macro_export]
macro_rules! operations {
    ( $($variant:ident($inner:ty)),* $(,)?) => {
        #[derive(Debug, Serialize, Deserialize, Clone)]
        pub enum Operation {
            $($variant,)*
        }

        impl Operation {
            pub fn settings(&self) -> NodeSettings {
                match self {
                    $(Operation::$variant => <$inner>::settings(),)*
                }
            }

            pub fn create_inputs(&self) -> Vec<Input> {
                match self {
                    $(Operation::$variant => <$inner>::create_inputs(),)*
                }
            }

            pub fn create_outputs(&self) -> Vec<Output> {
                match self {
                    $(Operation::$variant => <$inner>::create_outputs(),)*
                }
            }

            pub async fn run(&self, inputs: &Vec<Input>) -> Result<crate::operations::OperationResponse, crate::operations::OperationError> {
                match self {
                    $(Operation::$variant => <$inner>::run(inputs).await,)*
                }
            }
        }
    };
}

