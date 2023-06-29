use crate::operations::images::adjustments::blur::OperationImageAdjustmentBlur;
use crate::operations::images::inputs::clipboard::OperationImageInputClipboard;
use crate::operations::images::inputs::file::OperationImageInputFile;
use crate::operations::images::outputs::file::OperationImageOutputFile;
use crate::operations::images::transform::resize::OperationImageTransformResize;
use crate::operations::images::inputs::url::OperationImageInputUrl;
use crate::operations::images::transform::resize_exact::OperationImageTransformResizeExact;
use crate::operations::images::transform::resize_fill::OperationImageTransformResizeFill;
use crate::operations::numbers::math::add::OperationNumberMathAdd;
use crate::operations::numbers::inputs::{integer::OperationNumberInputInteger, decimal::OperationNumberInputDecimal};
use crate::node_settings::NodeSettings;
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
    NumberMathSubtract,
    NumberMathMultiply,
    NumberMathDivide,
    NumberCastToInteger,
    NumberCastToDecimal,
    ColorInputRgba,
    ImageInputUrl,
    ImageInputClipboard,
    ImageInputFile,
    ImageInputColor,
    ImageOutputClipboard,
    ImageOutputFile,
    ImageTransformResize,
    ImageTransformResizeExact,
    ImageTransformResizeFill,
    ImageAdjustmentBlur,
    ImageAdjustmentContrast,
    IMageAdjustmentGrayscale,
}

impl Operation {
    pub fn list() -> Vec<Operation> {
        vec![
            Operation::NumberInputInteger,
            Operation::NumberInputDecimal,
            Operation::NumberMathAdd,
            Operation::NumberMathSubtract,
            Operation::NumberMathMultiply,
            Operation::NumberMathDivide,
            Operation::NumberCastToInteger,
            Operation::NumberCastToDecimal,
            Operation::ColorInputRgba,
            Operation::ImageInputUrl,
            Operation::ImageInputClipboard,
            Operation::ImageInputFile,
            Operation::ImageInputColor,
            Operation::ImageOutputClipboard,
            Operation::ImageOutputFile,
            Operation::ImageTransformResize,
            Operation::ImageTransformResizeExact,
            Operation::ImageTransformResizeFill,
            Operation::ImageAdjustmentBlur,
            Operation::ImageAdjustmentContrast,
            Operation::IMageAdjustmentGrayscale,
        ]
    }
}

macro_rules! op_settings {
    ($self:ident) => {
        Operation$self::settings()
    };
}






impl Operation {
    pub fn settings(&self) -> NodeSettings {
        op_settings!(self);
        match self {
            Operation::NumberInputInteger => OperationNumberInputInteger::settings(),
            Operation::NumberInputDecimal => OperationNumberInputDecimal::settings(),
            Operation::NumberMathAdd => OperationNumberMathAdd::settings(),
            Operation::NumberMathSubtract => crate::operations::numbers::math::subtract::OperationNumberMathSubtract::settings(),
            Operation::NumberMathMultiply => crate::operations::numbers::math::multiply::OperationNumberMathMultiply::settings(),
            Operation::NumberMathDivide => crate::operations::numbers::math::divide::OperationNumberMathDivide::settings(),
            Operation::NumberCastToInteger => crate::operations::numbers::cast::to_integer::OperationNumberCastToInteger::settings(),
            Operation::NumberCastToDecimal => crate::operations::numbers::cast::to_decimal::OperationNumberCastToDecimal::settings(),
            Operation::ColorInputRgba => crate::operations::colors::inputs::rgba::OperationColorInputRgba::settings(),
            Operation::ImageInputUrl => OperationImageInputUrl::settings(),
            Operation::ImageInputFile => OperationImageInputFile::settings(),
            Operation::ImageInputClipboard => crate::operations::images::inputs::clipboard::OperationImageInputClipboard::settings(),
            Operation::ImageInputColor => crate::operations::images::inputs::color::OperationImageInputColor::settings(),
            Operation::ImageOutputFile => crate::operations::images::outputs::file::OperationImageOutputFile::settings(),
            Operation::ImageOutputClipboard => crate::operations::images::outputs::clipboard::OperationImageOutputClipboard::settings(),
            Operation::ImageTransformResize => OperationImageTransformResize::settings(),
            Operation::ImageTransformResizeExact => OperationImageTransformResizeExact::settings(),
            Operation::ImageTransformResizeFill => OperationImageTransformResizeFill::settings(),
            Operation::ImageAdjustmentBlur => OperationImageAdjustmentBlur::settings(),
            Operation::ImageAdjustmentContrast => crate::operations::images::adjustments::contrast::OperationImageAdjustmentContrast::settings(),
            Operation::IMageAdjustmentGrayscale => crate::operations::images::adjustments::grayscale::OperationImageAdjustmentGrayscale::settings(),
        }
    }

    pub fn create_inputs(&self) -> Vec<Input> {
        match self {
            Operation::NumberInputInteger => OperationNumberInputInteger::create_inputs(),
            Operation::NumberInputDecimal => OperationNumberInputDecimal::create_inputs(),
            Operation::NumberMathAdd => OperationNumberMathAdd::create_inputs(),
            Operation::NumberMathSubtract => crate::operations::numbers::math::subtract::OperationNumberMathSubtract::create_inputs(),
            Operation::NumberMathMultiply => crate::operations::numbers::math::multiply::OperationNumberMathMultiply::create_inputs(),
            Operation::NumberMathDivide => crate::operations::numbers::math::divide::OperationNumberMathDivide::create_inputs(),
            Operation::NumberCastToInteger => crate::operations::numbers::cast::to_integer::OperationNumberCastToInteger::create_inputs(),
            Operation::NumberCastToDecimal => crate::operations::numbers::cast::to_decimal::OperationNumberCastToDecimal::create_inputs(),
            Operation::ColorInputRgba => crate::operations::colors::inputs::rgba::OperationColorInputRgba::create_inputs(),
            Operation::ImageInputUrl => OperationImageInputUrl::create_inputs(),
            Operation::ImageInputFile => OperationImageInputFile::create_inputs(),
            Operation::ImageInputClipboard => OperationImageInputClipboard::create_inputs(),
            Operation::ImageInputColor => crate::operations::images::inputs::color::OperationImageInputColor::create_inputs(),
            Operation::ImageOutputFile => OperationImageOutputFile::create_inputs(),
            Operation::ImageOutputClipboard => crate::operations::images::outputs::clipboard::OperationImageOutputClipboard::create_inputs(),
            Operation::ImageTransformResize => OperationImageTransformResize::create_inputs(),
            Operation::ImageTransformResizeExact => OperationImageTransformResizeExact::create_inputs(),
            Operation::ImageTransformResizeFill => OperationImageTransformResizeFill::create_inputs(),
            Operation::ImageAdjustmentBlur => OperationImageAdjustmentBlur::create_inputs(),
            Operation::ImageAdjustmentContrast => crate::operations::images::adjustments::contrast::OperationImageAdjustmentContrast::create_inputs(),
            Operation::IMageAdjustmentGrayscale => crate::operations::images::adjustments::grayscale::OperationImageAdjustmentGrayscale::create_inputs(),
        }
    }

    pub fn create_outputs(&self) -> Vec<Output> {
        match self {
            Operation::NumberInputInteger => OperationNumberInputInteger::create_outputs(),
            Operation::NumberInputDecimal => OperationNumberInputDecimal::create_outputs(),
            Operation::NumberMathAdd => OperationNumberMathAdd::create_outputs(),
            Operation::NumberMathSubtract => crate::operations::numbers::math::subtract::OperationNumberMathSubtract::create_outputs(),
            Operation::NumberMathMultiply => crate::operations::numbers::math::multiply::OperationNumberMathMultiply::create_outputs(),
            Operation::NumberMathDivide => crate::operations::numbers::math::divide::OperationNumberMathDivide::create_outputs(),
            Operation::NumberCastToInteger => crate::operations::numbers::cast::to_integer::OperationNumberCastToInteger::create_outputs(),
            Operation::NumberCastToDecimal => crate::operations::numbers::cast::to_decimal::OperationNumberCastToDecimal::create_outputs(),
            Operation::ColorInputRgba => crate::operations::colors::inputs::rgba::OperationColorInputRgba::create_outputs(),
            Operation::ImageInputUrl => OperationImageInputUrl::create_outputs(),
            Operation::ImageInputFile => OperationImageInputFile::create_outputs(),
            Operation::ImageInputClipboard => OperationImageInputClipboard::create_outputs(),
            Operation::ImageInputColor => crate::operations::images::inputs::color::OperationImageInputColor::create_outputs(),
            Operation::ImageOutputFile => OperationImageOutputFile::create_outputs(),
            Operation::ImageOutputClipboard => crate::operations::images::outputs::clipboard::OperationImageOutputClipboard::create_outputs(),
            Operation::ImageTransformResize => OperationImageTransformResize::create_outputs(),
            Operation::ImageTransformResizeExact => OperationImageTransformResizeExact::create_outputs(),
            Operation::ImageTransformResizeFill => OperationImageTransformResizeFill::create_outputs(),
            Operation::ImageAdjustmentBlur => OperationImageAdjustmentBlur::create_outputs(),
            Operation::ImageAdjustmentContrast => crate::operations::images::adjustments::contrast::OperationImageAdjustmentContrast::create_outputs(),
            Operation::IMageAdjustmentGrayscale => crate::operations::images::adjustments::grayscale::OperationImageAdjustmentGrayscale::create_outputs(),
        }
    }

    pub async fn run(&self, inputs: &Vec<Input>) -> Result<OperationResponse, OperationError> {
        match self {
            Operation::NumberInputInteger => OperationNumberInputInteger::run(inputs).await,
            Operation::NumberInputDecimal => OperationNumberInputDecimal::run(inputs).await,
            Operation::NumberMathAdd => OperationNumberMathAdd::run(inputs).await,
            Operation::NumberMathSubtract => crate::operations::numbers::math::subtract::OperationNumberMathSubtract::run(inputs).await,
            Operation::NumberMathMultiply => crate::operations::numbers::math::multiply::OperationNumberMathMultiply::run(inputs).await,
            Operation::NumberMathDivide => crate::operations::numbers::math::divide::OperationNumberMathDivide::run(inputs).await,
            Operation::NumberCastToInteger => crate::operations::numbers::cast::to_integer::OperationNumberCastToInteger::run(inputs).await,
            Operation::NumberCastToDecimal => crate::operations::numbers::cast::to_decimal::OperationNumberCastToDecimal::run(inputs).await,
            Operation::ColorInputRgba => crate::operations::colors::inputs::rgba::OperationColorInputRgba::run(inputs).await,
            Operation::ImageInputUrl => OperationImageInputUrl::run(inputs).await,
            Operation::ImageInputFile => OperationImageInputFile::run(inputs).await,
            Operation::ImageInputClipboard => OperationImageInputClipboard::run(inputs).await,
            Operation::ImageInputColor => crate::operations::images::inputs::color::OperationImageInputColor::run(inputs).await,
            Operation::ImageOutputFile => OperationImageOutputFile::run(inputs).await,
            Operation::ImageOutputClipboard => crate::operations::images::outputs::clipboard::OperationImageOutputClipboard::run(inputs).await,
            Operation::ImageTransformResize => OperationImageTransformResize::run(inputs).await,
            Operation::ImageTransformResizeExact => OperationImageTransformResizeExact::run(inputs).await,
            Operation::ImageTransformResizeFill => OperationImageTransformResizeFill::run(inputs).await,
            Operation::ImageAdjustmentBlur => OperationImageAdjustmentBlur::run(inputs).await,
            Operation::ImageAdjustmentContrast => crate::operations::images::adjustments::contrast::OperationImageAdjustmentContrast::run(inputs).await,
            Operation::IMageAdjustmentGrayscale => crate::operations::images::adjustments::grayscale::OperationImageAdjustmentGrayscale::run(inputs).await,
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
