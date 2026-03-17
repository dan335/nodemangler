use crate::get_id;
use crate::value::ValueType;
use image::RgbaImage;
use crate::input::Input;
use crate::node_settings::NodeSettings;
use crate::operations::{OperationResponse, OperationError, OutputResponse, default_image, convert_input};
use crate::output::Output;
use crate::value::Value;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use std::time::Instant;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpImageChannelSplit {}

impl OpImageChannelSplit {
    pub fn settings() -> NodeSettings {
        NodeSettings {
            name: "channel split".to_string(),
            description: "Splits an image into R, G, B, A channels.".to_string(),
        }
    }

    pub fn create_inputs() -> Vec<Input> {
        vec![
            Input::new("image".to_string(), Value::DynamicImage { data: default_image(), change_id: get_id() }, None, None),
        ]
    }

    pub fn create_outputs() -> Vec<Output> {
        vec![
            Output::new("red".to_string(), Value::DynamicImage { data: default_image(), change_id: get_id() }, None),
            Output::new("green".to_string(), Value::DynamicImage { data: default_image(), change_id: get_id() }, None),
            Output::new("blue".to_string(), Value::DynamicImage { data: default_image(), change_id: get_id() }, None),
            Output::new("alpha".to_string(), Value::DynamicImage { data: default_image(), change_id: get_id() }, None),
        ]
    }

    pub async fn run(inputs: &mut Vec<Input>) -> Result<OperationResponse, OperationError> {
        let start_time = Instant::now();
        let mut input_errors: Vec<(usize, String)> = vec![];

        // convert inputs
        let image_converted = convert_input(inputs, 0, ValueType::DynamicImage, &mut input_errors);

        // return if error
        if input_errors.len() > 0 { return Err(OperationError { input_errors, node_error: None }); }

        // get values
        let Value::DynamicImage{data, change_id:_} = image_converted.unwrap() else { unreachable!() };

        // run node
        let rgba = data.to_rgba8();
        let (width, height) = rgba.dimensions();

        let mut red_buf = RgbaImage::new(width, height);
        let mut green_buf = RgbaImage::new(width, height);
        let mut blue_buf = RgbaImage::new(width, height);
        let mut alpha_buf = RgbaImage::new(width, height);

        for (x, y, pixel) in rgba.enumerate_pixels() {
            let [r, g, b, a] = pixel.0;
            red_buf.put_pixel(x, y, image::Rgba([r, r, r, 255]));
            green_buf.put_pixel(x, y, image::Rgba([g, g, g, 255]));
            blue_buf.put_pixel(x, y, image::Rgba([b, b, b, 255]));
            alpha_buf.put_pixel(x, y, image::Rgba([a, a, a, 255]));
        }

        Ok(OperationResponse {
            time: Instant::now().duration_since(start_time),
            responses: vec![
                OutputResponse { value: Value::DynamicImage { data: Arc::new(image::DynamicImage::ImageRgba8(red_buf)), change_id: get_id() } },
                OutputResponse { value: Value::DynamicImage { data: Arc::new(image::DynamicImage::ImageRgba8(green_buf)), change_id: get_id() } },
                OutputResponse { value: Value::DynamicImage { data: Arc::new(image::DynamicImage::ImageRgba8(blue_buf)), change_id: get_id() } },
                OutputResponse { value: Value::DynamicImage { data: Arc::new(image::DynamicImage::ImageRgba8(alpha_buf)), change_id: get_id() } },
            ],
        })
    }
}
