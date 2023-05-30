use crate::operations::images::transform::resize::OperationImageTransformResize;
use crate::operations::images::inputs::url::OperationImageInputUrl;
use crate::operations::numbers::math::add::OperationNumberMathAdd;
use crate::operations::numbers::inputs::{integer::OperationNumberInputInteger, decimal::OperationNumberInputDecimal};
use crate::node_settings::NodeSettings;
use crate::operations::sub_graph::OperationSubgraph;
use crate::value::Value;
use core::fmt::Debug;
use serde::{Deserialize, Serialize};
use std::time::Duration;

use crate::{input::Input, output::Output, value::ValueType};

#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum Operation {
    NumberInputInteger,
    NumberInputDecimal,
    NumberMathAdd,
    ImageInputUrl,
    ImageTransformResize,
    Subgraph,
}

impl Operation {
    pub fn settings(&self) -> NodeSettings {
        match self {
            Operation::NumberInputInteger => OperationNumberInputInteger::settings(),
            Operation::NumberInputDecimal => OperationNumberInputDecimal::settings(),
            Operation::NumberMathAdd => OperationNumberMathAdd::settings(),
            Operation::ImageInputUrl => OperationImageInputUrl::settings(),
            Operation::ImageTransformResize => OperationImageTransformResize::settings(),
            Operation::Subgraph => OperationSubgraph::settings(),
        }
    }

    pub fn create_inputs(&self) -> Vec<Input> {
        match self {
            Operation::NumberInputInteger => OperationNumberInputInteger::create_inputs(),
            Operation::NumberInputDecimal => OperationNumberInputDecimal::create_inputs(),
            Operation::NumberMathAdd => OperationNumberMathAdd::create_inputs(),
            Operation::ImageInputUrl => OperationImageInputUrl::create_inputs(),
            Operation::ImageTransformResize => OperationImageTransformResize::create_inputs(),
            Operation::Subgraph => OperationSubgraph::create_inputs(),
        }
    }

    pub fn create_outputs(&self) -> Vec<Output> {
        match self {
            Operation::NumberInputInteger => OperationNumberInputInteger::create_outputs(),
            Operation::NumberInputDecimal => OperationNumberInputDecimal::create_outputs(),
            Operation::NumberMathAdd => OperationNumberMathAdd::create_outputs(),
            Operation::ImageInputUrl => OperationImageInputUrl::create_outputs(),
            Operation::ImageTransformResize => OperationImageTransformResize::create_outputs(),
            Operation::Subgraph => OperationSubgraph::create_outputs(),
        }
    }

    pub async fn run(&self, inputs: &[Input]) -> Result<OperationResponse, OperationError> {
        match self {
            Operation::NumberInputInteger => OperationNumberInputInteger::run(inputs).await,
            Operation::NumberInputDecimal => OperationNumberInputDecimal::run(inputs).await,
            Operation::NumberMathAdd => OperationNumberMathAdd::run(inputs).await,
            Operation::ImageInputUrl => OperationImageInputUrl::run(inputs).await,
            Operation::ImageTransformResize => OperationImageTransformResize::run(inputs).await,
            Operation::Subgraph => OperationSubgraph::run(inputs).await,
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
