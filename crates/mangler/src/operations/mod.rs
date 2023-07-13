use crate::{value::ValueType};
use image::DynamicImage;
use serde::{Deserialize, Serialize};
use crate::value::Value;
use core::fmt::Debug;
use std::time::Duration;
use crate::{node_settings::NodeSettings, operations};
use crate::{input::Input, output::Output};

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

#[derive(Clone)]
pub enum OperationListItem {
    Category {
        name: String,
        operation_list_items: Vec<OperationListItem>,
    },
    Operation {
        operation: Operation,
    },
    Subgraph
}

pub fn default_image() -> DynamicImage {
    DynamicImage::ImageRgba8(image::RgbaImage::new(1, 1))
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

operations! {
    // numbers
    OpNumberInputInteger(crate::operations::numbers::inputs::integer::OpNumberInputInteger),
    OpNumberInputDecimal(crate::operations::numbers::inputs::decimal::OpNumberInputDecimal),

    OpNumberCastToInteger(crate::operations::numbers::cast::to_integer::OpNumberCastToInteger),
    OpNumberCastToDecimal(crate::operations::numbers::cast::to_decimal::OpNumberCastToDecimal),

    OpNumberMathAdd(crate::operations::numbers::arithmetic::add::OpNumberMathAdd),
    OpNumberMathSubtract(crate::operations::numbers::arithmetic::subtract::OpNumberMathSubtract),
    OpNumberMathMultiply(crate::operations::numbers::arithmetic::multiply::OpNumberMathMultiply),
    OpNumberMathDivide(crate::operations::numbers::arithmetic::divide::OpNumberMathDivide),
    OpNumberMathDecrement(crate::operations::numbers::arithmetic::decrement::OpNumberMathDecrement),
    OpNumberMathIncrement(crate::operations::numbers::arithmetic::increment::OpNumberMathIncrement),
    OpNumberMathMax(crate::operations::numbers::arithmetic::max::OpNumberMathMax),
    OpNumberMathMin(crate::operations::numbers::arithmetic::min::OpNumberMathMin),
    OpNumberMathClamp(crate::operations::numbers::arithmetic::clamp::OpNumberMathClamp),
    OpNumberMathModulus(crate::operations::numbers::arithmetic::modulus::OpNumberMathModulus),
    OpNumberMathRound(crate::operations::numbers::arithmetic::round::OpNumberMathRound),
    OpNumberMathSign(crate::operations::numbers::arithmetic::sign::OpNumberMathSign),

    // random
    OpNumberRandomDecimal(crate::operations::numbers::random::random_decimal::OpNumberRandomDecimal),
    OpNumberRandomInteger(crate::operations::numbers::random::random_integer::OpNumberRandomInteger),

    // algebra
    OpNumberMathAbs(crate::operations::numbers::algebra::abs::OpNumberMathAbs),
    OpNumberMathSqrt(crate::operations::numbers::algebra::sqrt::OpNumberMathSqrt),
    OpNumberMathCbrt(crate::operations::numbers::algebra::cbrt::OpNumberMathCbrt),
    OpNumberMathNthRt(crate::operations::numbers::algebra::nth_root::OpNumberMathNthRt),

    // colors
    OpColorInputCmyk(crate::operations::colors::inputs::cmyk::OpColorInputCmyk),
    OpColorInputHsl(crate::operations::colors::inputs::hsl::OpColorInputHsla),
    OpColorInputHsv(crate::operations::colors::inputs::hsv::OpColorInputHsva),
    OpColorInputLab(crate::operations::colors::inputs::lab::OpColorInputLab),
    OpColorInputLch(crate::operations::colors::inputs::lch::OpColorInputLch),
    OpColorInputRgbLinear(crate::operations::colors::inputs::rgb_linear::OpColorInputRgbaLinear),
    OpColorInputRgb(crate::operations::colors::inputs::srgb::OpColorInputRgba),
    OpColorInputXyz(crate::operations::colors::inputs::xyz::OpColorInputXyz),
    OpColorInputYuv(crate::operations::colors::inputs::yuv::OpColorInputYuv),

    OpColorOutputCmyk(crate::operations::colors::outputs::to_cmyk::OpColorOutputCmyk),
    OpColorOutputHsl(crate::operations::colors::outputs::to_hsl::OpColorOutputHsl),
    OpColorOutputHsv(crate::operations::colors::outputs::to_hsv::OpColorOutputHsv),
    OpColorOutputLab(crate::operations::colors::outputs::to_lab::OpColorOutputLab),
    OpColorOutputLch(crate::operations::colors::outputs::to_lch::OpColorOutputLch),
    OpColorOutputRgbLinear(crate::operations::colors::outputs::to_rgb_linear::OpColorOutputRgbLinear),
    OpColorOutputRgb(crate::operations::colors::outputs::to_srgb::OpColorOutputRgb),
    OpColorOutputXyz(crate::operations::colors::outputs::to_xyz::OpColorOutputXyz),
    OpColorOutputYuv(crate::operations::colors::outputs::to_yuv::OpColorOutputYuv),

    OpColorBlendCmyk(crate::operations::colors::blend::cmyk::OpColorBlendCmyk),
    OpColorBlendHsl(crate::operations::colors::blend::hsl::OpColorBlendHsl),
    OpColorBlendHsv(crate::operations::colors::blend::hsv::OpColorBlendHsv),
    OpColorBlendLab(crate::operations::colors::blend::lab::OpColorBlendLab),
    OpColorBlendLch(crate::operations::colors::blend::lch::OpColorBlendLch),
    OpColorBlendSrgb(crate::operations::colors::blend::srgb::OpColorBlendSrgb),
    OpColorBlendRgbLinear(crate::operations::colors::blend::rgb_linear::OpColorBlendRgbLinear),
    OpColorBlendXyz(crate::operations::colors::blend::xyz::OpColorBlendXyz),
    OpColorBlendYuv(crate::operations::colors::blend::yuv::OpColorBlendYuv),

    OpColorSampleMostCommonColors(crate::operations::colors::sample_image::most_common_colors::OpColorSampleMostCommonColors),

    // image
    OpImageInputUrl(crate::operations::images::inputs::url::OpImageInputUrl),
    OpImageInputClipboard(crate::operations::images::inputs::clipboard::OpImageInputClipboard),
    OpImageInputColor(crate::operations::images::inputs::color::OpImageInputColor),
    OpImageInputFile(crate::operations::images::inputs::file::OpImageInputFile),
    OpImageInputGradient(crate::operations::images::inputs::gradient::OpImageInputGradient),

    OpImageOutputClipboard(crate::operations::images::outputs::clipboard::OpImageOutputClipboard),
    OpImageOutputFile(crate::operations::images::outputs::file::OpImageOutputFile),

    OpImageCombineBlit(crate::operations::images::combine::blit::OpImageCombineBlit),

    OpImageTransformCrop(crate::operations::images::transform::crop::OpImageTransformCrop),
    OpImageTransformResize(crate::operations::images::transform::resize::OpImageTransformResize),
    OpImageTransformResizeExact(crate::operations::images::transform::resize_exact::OpImageTransformResizeExact),
    OpImageTransformResizeFill(crate::operations::images::transform::resize_fill::OpImageTransformResizeFill),
    OpImageTransformFlipHorizontal(crate::operations::images::transform::flip_horizontal::OpImageTransformFlipHorizontal),
    OpImageTransformFlipVertical(crate::operations::images::transform::flip_vertical::OpImageTransformFlipVertical),
    OpImageTransformRotate90(crate::operations::images::transform::rotate_90::OpImageTransformRotate90),
    OpImageTransformRotate180(crate::operations::images::transform::rotate_180::OpImageTransformRotate180),
    OpImageTransformRotate270(crate::operations::images::transform::rotate_270::OpImageTransformRotate270),
    OpImageTransformRotateAroundCenter(crate::operations::images::transform::rotate_around_center::OpImageTransformRotateAroundCenter),

    OpImageAdjustmentBlur(crate::operations::images::adjustments::blur::OpImageAdjustmentBlur),
    OpImageAdjustmentContrast(crate::operations::images::adjustments::contrast::OpImageAdjustmentContrast),
    OpImageAdjustmentGrayscale(crate::operations::images::adjustments::grayscale::OpImageAdjustmentGrayscale),
    OpImageAdjustmentInvert(crate::operations::images::adjustments::invert::OpImageAdjustmentInvert),
    OpImageAdjustmentBrighten(crate::operations::images::adjustments::brighten::OpImageAdjustmentBrighten),
    OpImageAdjustmentHueRotate(crate::operations::images::adjustments::hue_rotate::OpImageAdjustmentHueRotate),
    OpImageAdjustmentUnsharpen(crate::operations::images::adjustments::unsharpen::OpImageAdjustmentUnsharpen),

    OpImageNoisePerlin(crate::operations::images::noise::perlin::OpImageNoisePerlin),
    OpImageNoiseWorleyDistance(crate::operations::images::noise::worley_distance::OpImageNoiseWorleyDistance),
    OpImageNoiseWorleyValue(crate::operations::images::noise::worley_value::OpImageNoiseWorleyValue),
    OpImageNoiseHeterogenousMultifractalNoise(crate::operations::images::noise::heterogenous_multifractal::OpImageNoiseHeterogenousMultifractalNoise),
    OpImageNoiseBillow(crate::operations::images::noise::billow::OpImageNoiseBillow),
    OpImageNoiseCylinders(crate::operations::images::noise::cylinders::OpImageNoiseCylinders),
    OpImageNoiseFbm(crate::operations::images::noise::fbm::OpImageNoiseFbm),
    OpImageNoiseHybridMultifractalNoise(crate::operations::images::noise::hybrid_multifractal::OpImageNoiseHybridMultifractalNoise),
    OpImageNoiseOpenSimplex(crate::operations::images::noise::open_simplex::OpImageNoiseOpenSimplex),
    OpImageNoiseSimplex(crate::operations::images::noise::simplex::OpImageNoiseSimplex),
    OpImageNoiseSuperSimplex(crate::operations::images::noise::super_simplex::OpImageNoiseSuperSimplex),
    OpImageNoisePerlinSurflet(crate::operations::images::noise::perlin_surflet::OpImageNoisePerlinSurflet),
    OpImageNoiseRidgedMultifractalNoise(crate::operations::images::noise::ridged_multifractal::OpImageNoiseRidgedMultifractalNoise),
    OpImageNoiseValue(crate::operations::images::noise::value::OpImageNoiseValue),
}

pub fn operation_list() -> Vec<OperationListItem> {
    vec![
        OperationListItem::Category { name: "numbers".to_string(), operation_list_items: vec![
            OperationListItem::Category { name: "input".to_string(), operation_list_items: vec![
                OperationListItem::Operation { operation: Operation::OpNumberInputDecimal },
                OperationListItem::Operation { operation: Operation::OpNumberInputInteger },
            ]},
            OperationListItem::Category { name: "arithmetic".to_string(), operation_list_items: vec![
                OperationListItem::Operation { operation: Operation::OpNumberMathAdd },
                OperationListItem::Operation { operation: Operation::OpNumberMathSubtract },
                OperationListItem::Operation { operation: Operation::OpNumberMathMultiply },
                OperationListItem::Operation { operation: Operation::OpNumberMathDivide },
                OperationListItem::Operation { operation: Operation::OpNumberMathDecrement },
                OperationListItem::Operation { operation: Operation::OpNumberMathIncrement },
                OperationListItem::Operation { operation: Operation::OpNumberMathMax },
                OperationListItem::Operation { operation: Operation::OpNumberMathMin },
                OperationListItem::Operation { operation: Operation::OpNumberMathClamp },
                OperationListItem::Operation { operation: Operation::OpNumberMathModulus },
                OperationListItem::Operation { operation: Operation::OpNumberMathRound },
                OperationListItem::Operation { operation: Operation::OpNumberMathSign },
            ]},
            OperationListItem::Category { name: "algebraic".to_string(), operation_list_items: vec![
                OperationListItem::Operation { operation: Operation::OpNumberMathAbs },
                OperationListItem::Operation { operation: Operation::OpNumberMathSqrt },
                OperationListItem::Operation { operation: Operation::OpNumberMathCbrt },
                OperationListItem::Operation { operation: Operation::OpNumberMathNthRt },
            ]},
            OperationListItem::Category { name: "random".to_string(), operation_list_items: vec![
                OperationListItem::Operation { operation: Operation::OpNumberRandomDecimal },
                OperationListItem::Operation { operation: Operation::OpNumberRandomInteger },
            ]},
        ]},
        OperationListItem::Category { name: "colors".to_string(), operation_list_items: vec![
            OperationListItem::Category { name: "input".to_string(), operation_list_items: vec![
                OperationListItem::Operation { operation: Operation::OpColorInputCmyk },
                OperationListItem::Operation { operation: Operation::OpColorInputHsl },
                OperationListItem::Operation { operation: Operation::OpColorInputHsv },
                OperationListItem::Operation { operation: Operation::OpColorInputLab },
                OperationListItem::Operation { operation: Operation::OpColorInputLch },
                OperationListItem::Operation { operation: Operation::OpColorInputRgb },
                OperationListItem::Operation { operation: Operation::OpColorInputRgbLinear },
                OperationListItem::Operation { operation: Operation::OpColorInputXyz },
                OperationListItem::Operation { operation: Operation::OpColorInputYuv },
            ]},
            OperationListItem::Category { name: "output".to_string(), operation_list_items: vec![
                OperationListItem::Operation { operation: Operation::OpColorOutputCmyk },
                OperationListItem::Operation { operation: Operation::OpColorOutputHsl },
                OperationListItem::Operation { operation: Operation::OpColorOutputHsv },
                OperationListItem::Operation { operation: Operation::OpColorOutputLab },
                OperationListItem::Operation { operation: Operation::OpColorOutputLch },
                OperationListItem::Operation { operation: Operation::OpColorOutputRgb },
                OperationListItem::Operation { operation: Operation::OpColorOutputRgbLinear },
                OperationListItem::Operation { operation: Operation::OpColorOutputXyz },
                OperationListItem::Operation { operation: Operation::OpColorOutputYuv },
            ]},
            OperationListItem::Category { name: "blend".to_string(), operation_list_items: vec![
                OperationListItem::Operation { operation: Operation::OpColorBlendCmyk },
                OperationListItem::Operation { operation: Operation::OpColorBlendHsl },
                OperationListItem::Operation { operation: Operation::OpColorBlendHsv },
                OperationListItem::Operation { operation: Operation::OpColorBlendLab },
                OperationListItem::Operation { operation: Operation::OpColorBlendLch },
                OperationListItem::Operation { operation: Operation::OpColorBlendRgbLinear },
                OperationListItem::Operation { operation: Operation::OpColorBlendSrgb },
                OperationListItem::Operation { operation: Operation::OpColorBlendXyz },
                OperationListItem::Operation { operation: Operation::OpColorBlendYuv },
            ]},
            OperationListItem::Category { name: "sample image".to_string(), operation_list_items: vec![
                OperationListItem::Operation { operation: Operation::OpColorSampleMostCommonColors },
            ]},
        ]},
        OperationListItem::Category { name: "images".to_string(), operation_list_items: vec![
            OperationListItem::Category { name: "input".to_string(), operation_list_items: vec![
                OperationListItem::Operation { operation: Operation::OpImageInputFile },
                OperationListItem::Operation { operation: Operation::OpImageInputUrl },
                OperationListItem::Operation { operation: Operation::OpImageInputClipboard },
                OperationListItem::Operation { operation: Operation::OpImageInputColor },
                OperationListItem::Operation { operation: Operation::OpImageInputGradient },
            ]},
            OperationListItem::Category { name: "output".to_string(), operation_list_items: vec![
                OperationListItem::Operation { operation: Operation::OpImageOutputFile },
                OperationListItem::Operation { operation: Operation::OpImageOutputClipboard },
            ]},
            OperationListItem::Category { name: "combine".to_string(), operation_list_items: vec![
                OperationListItem::Operation { operation: Operation::OpImageCombineBlit },
            ]},
            OperationListItem::Category { name: "transform".to_string(), operation_list_items: vec![
                OperationListItem::Operation { operation: Operation::OpImageTransformCrop },
                OperationListItem::Operation { operation: Operation::OpImageTransformResize },
                OperationListItem::Operation { operation: Operation::OpImageTransformResizeExact },
                OperationListItem::Operation { operation: Operation::OpImageTransformResizeFill },
                OperationListItem::Operation { operation: Operation::OpImageTransformFlipHorizontal },
                OperationListItem::Operation { operation: Operation::OpImageTransformFlipVertical },
                OperationListItem::Operation { operation: Operation::OpImageTransformRotate90 },
                OperationListItem::Operation { operation: Operation::OpImageTransformRotate180 },
                OperationListItem::Operation { operation: Operation::OpImageTransformRotate270 },
                OperationListItem::Operation { operation: Operation::OpImageTransformRotateAroundCenter },
            ]},
            OperationListItem::Category { name: "adjustments".to_string(), operation_list_items: vec![
                OperationListItem::Operation { operation: Operation::OpImageAdjustmentBlur },
                OperationListItem::Operation { operation: Operation::OpImageAdjustmentContrast },
                OperationListItem::Operation { operation: Operation::OpImageAdjustmentGrayscale },
                OperationListItem::Operation { operation: Operation::OpImageAdjustmentInvert },
                OperationListItem::Operation { operation: Operation::OpImageAdjustmentBrighten },
                OperationListItem::Operation { operation: Operation::OpImageAdjustmentHueRotate },
                OperationListItem::Operation { operation: Operation::OpImageAdjustmentUnsharpen },
            ]},
            OperationListItem::Category { name: "noise".to_string(), operation_list_items: vec![
                OperationListItem::Operation { operation: Operation::OpImageNoiseOpenSimplex },
                OperationListItem::Operation { operation: Operation::OpImageNoiseSimplex },
                OperationListItem::Operation { operation: Operation::OpImageNoiseSuperSimplex },
                OperationListItem::Operation { operation: Operation::OpImageNoisePerlin },
                OperationListItem::Operation { operation: Operation::OpImageNoisePerlinSurflet },
                OperationListItem::Operation { operation: Operation::OpImageNoiseWorleyDistance },
                OperationListItem::Operation { operation: Operation::OpImageNoiseWorleyValue },
                OperationListItem::Operation { operation: Operation::OpImageNoiseBillow },
                OperationListItem::Operation { operation: Operation::OpImageNoiseCylinders },
                OperationListItem::Operation { operation: Operation::OpImageNoiseFbm },
                OperationListItem::Operation { operation: Operation::OpImageNoiseHeterogenousMultifractalNoise },
                OperationListItem::Operation { operation: Operation::OpImageNoiseHybridMultifractalNoise },
                OperationListItem::Operation { operation: Operation::OpImageNoiseRidgedMultifractalNoise },
                OperationListItem::Operation { operation: Operation::OpImageNoiseValue },
            ]},
        ]}, 
        //OperationListItem::Subgraph,
    ]
}




