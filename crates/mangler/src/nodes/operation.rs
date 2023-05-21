use image::{ImageBuffer, Rgba, DynamicImage};
use tokio::sync::mpsc::Sender;

use crate::{NodeOutputChangedMessage, value::Value};
use core::fmt::Debug;
use std::time::Duration;

use crate::{
    input::Input,
    output::Output,
    value::ValueType,
};

pub const THUMBNAIL_SIZE: [u32; 2] = [128, 128];

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
    pub async fn run(&self, node_id: &String, inputs: &Vec<Input>, outputs: &mut Vec<Output>, tx_output: Sender<NodeOutputChangedMessage>) -> Duration {
        let node_output_messages = match self {
            Operation::Float => new_float(node_id, inputs).await,
            Operation::Integer => new_integer(node_id, inputs).await,
            Operation::Add => add(node_id, inputs).await,
            Operation::Subtract => subtract(node_id, inputs).await,
            Operation::ImageFromUrl => image_from_url(node_id, inputs).await,
            Operation::ImageResize => image_resize(node_id, inputs).await,
        };
        
        let time = node_output_messages[0].time;

        for (index, mut node_output_message) in node_output_messages.into_iter().enumerate() {

            node_output_message.thumbnail = Self::create_thumbnail(&node_output_message.value);

            match tx_output.try_send(node_output_message.clone()) {
                Ok(_) => {
                    outputs[index].value = node_output_message.value;
                },
                Err(err) => {
                    println!("Error: {:?}", err);
                },
            }
        }

        time
    }

    pub fn create_thumbnail(value: &Value) -> Option<ImageBuffer<Rgba<u8>, Vec<u8>>> {
        match value {
            Value::ImageRgba32F(value) => {
                Some(DynamicImage::ImageRgba32F(value.clone()).resize(THUMBNAIL_SIZE[0], THUMBNAIL_SIZE[1], image::imageops::FilterType::Triangle).to_rgba8())
            },
            Value::ImageRgba8(value) => {
                Some(DynamicImage::ImageRgba8(value.clone()).resize(THUMBNAIL_SIZE[0], THUMBNAIL_SIZE[1], image::imageops::FilterType::Triangle).to_rgba8())
            },
            Value::ImageGray8(value) => {
                Some(DynamicImage::ImageLuma8(value.clone()).resize(THUMBNAIL_SIZE[0], THUMBNAIL_SIZE[1], image::imageops::FilterType::Triangle).to_rgba8())
            },
            Value::ImageRgb32F(value) => {
                Some(DynamicImage::ImageRgb32F(value.clone()).resize(THUMBNAIL_SIZE[0], THUMBNAIL_SIZE[1], image::imageops::FilterType::Triangle).to_rgba8())
            },
            Value::ImageRgba16(value) => {
                Some(DynamicImage::ImageRgba16(value.clone()).resize(THUMBNAIL_SIZE[0], THUMBNAIL_SIZE[1], image::imageops::FilterType::Triangle).to_rgba8())
            },
            Value::ImageRgb16(value) => {
                Some(DynamicImage::ImageRgb16(value.clone()).resize(THUMBNAIL_SIZE[0], THUMBNAIL_SIZE[1], image::imageops::FilterType::Triangle).to_rgba8())
            },
            Value::ImageGrayA16(value) => {
                Some(DynamicImage::ImageLumaA16(value.clone()).resize(THUMBNAIL_SIZE[0], THUMBNAIL_SIZE[1], image::imageops::FilterType::Triangle).to_rgba8())
            },
            Value::ImageGray16(value) => {
                Some(DynamicImage::ImageLuma16(value.clone()).resize(THUMBNAIL_SIZE[0], THUMBNAIL_SIZE[1], image::imageops::FilterType::Triangle).to_rgba8())
            },
            Value::ImageRgb8(value) => {
                Some(DynamicImage::ImageRgb8(value.clone()).resize(THUMBNAIL_SIZE[0], THUMBNAIL_SIZE[1], image::imageops::FilterType::Triangle).to_rgba8())
            },
            Value::ImageGrayA8(value) => {
                Some(DynamicImage::ImageLumaA8(value.clone()).resize(THUMBNAIL_SIZE[0], THUMBNAIL_SIZE[1], image::imageops::FilterType::Triangle).to_rgba8())
            },
            Value::Bool(_) |
            Value::Integer(_) |
            Value::Decimal(_) |
            Value::String(_) |
            Value::FilterType(_) |
            Value::ImageFormat(_) => {
                None
            },
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
    ComboBox,
}


// pub struct OperationResponse {
//     pub output_values: Vec<NodeOutputChangedMessage>,
// }

// impl OperationResponse {
//     pub fn new() -> OperationResponse {
//         OperationResponse { output_values: Vec::new() }
//     }
// }
