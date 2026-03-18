//! Channel shuffle (remap) operation.
//!
//! Remaps the RGBA channels of an image by selecting which source channel
//! (0=R, 1=G, 2=B, 3=A) feeds each output channel. This allows swapping,
//! duplicating, or rearranging channels arbitrarily.

use crate::get_id;
use crate::value::ValueType;
use image::RgbaImage;
use crate::input::{Input, InputSettings};
use crate::node_settings::NodeSettings;
use crate::operations::{OperationResponse, OperationError, OutputResponse, default_image, convert_input};
use crate::output::Output;
use crate::value::Value;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use std::time::Instant;

/// Operation that remaps image channels using selectable source indices.
///
/// Each output channel (R, G, B, A) is assigned a source channel index
/// (0=Red, 1=Green, 2=Blue, 3=Alpha). This enables channel swapping
/// (e.g., swap R and B), duplication (e.g., copy R to all channels),
/// or any arbitrary remapping.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpImageChannelShuffle {}

impl OpImageChannelShuffle {
    /// Returns the node metadata (name and description) for this operation.
    pub fn settings() -> NodeSettings {
        NodeSettings {
            name: "channel shuffle".to_string(),
            description: "Remaps image channels using selectable source channels.".to_string(),
        }
    }

    /// Creates the input definitions: an image and four source channel selectors (0-3 sliders).
    pub fn create_inputs() -> Vec<Input> {
        vec![
            Input::new("image".to_string(), Value::DynamicImage { data: default_image(), change_id: get_id() }, None, None),
            Input::new("red source".to_string(), Value::Integer(0), Some(InputSettings::Slider { range: (0.0, 3.0), step_by: Some(1.0), clamp_to_range: true }), None),
            Input::new("green source".to_string(), Value::Integer(1), Some(InputSettings::Slider { range: (0.0, 3.0), step_by: Some(1.0), clamp_to_range: true }), None),
            Input::new("blue source".to_string(), Value::Integer(2), Some(InputSettings::Slider { range: (0.0, 3.0), step_by: Some(1.0), clamp_to_range: true }), None),
            Input::new("alpha source".to_string(), Value::Integer(3), Some(InputSettings::Slider { range: (0.0, 3.0), step_by: Some(1.0), clamp_to_range: true }), None),
        ]
    }

    /// Creates the output definitions: the channel-remapped image.
    pub fn create_outputs() -> Vec<Output> {
        vec![
            Output::new("output".to_string(), Value::DynamicImage { data: default_image(), change_id: get_id() }, None),
        ]
    }

    /// Executes the operation: remaps each pixel's channels based on the source indices.
    pub async fn run(inputs: &mut [Input]) -> Result<OperationResponse, OperationError> {
        let start_time = Instant::now();
        let mut input_errors: Vec<(usize, String)> = vec![];

        // convert inputs
        let image_converted = convert_input(inputs, 0, ValueType::DynamicImage, &mut input_errors);
        let red_source_converted = convert_input(inputs, 1, ValueType::Integer, &mut input_errors);
        let green_source_converted = convert_input(inputs, 2, ValueType::Integer, &mut input_errors);
        let blue_source_converted = convert_input(inputs, 3, ValueType::Integer, &mut input_errors);
        let alpha_source_converted = convert_input(inputs, 4, ValueType::Integer, &mut input_errors);

        // return if error
        if !input_errors.is_empty() { return Err(OperationError { input_errors, node_error: None }); }

        // get values
        let Value::DynamicImage{data, change_id:_} = image_converted.unwrap() else { unreachable!() };
        let Value::Integer(red_source) = red_source_converted.unwrap() else { unreachable!() };
        let Value::Integer(green_source) = green_source_converted.unwrap() else { unreachable!() };
        let Value::Integer(blue_source) = blue_source_converted.unwrap() else { unreachable!() };
        let Value::Integer(alpha_source) = alpha_source_converted.unwrap() else { unreachable!() };

        // run node — clamp source indices to valid channel range [0, 3]
        let red_idx = (red_source.clamp(0, 3)) as usize;
        let green_idx = (green_source.clamp(0, 3)) as usize;
        let blue_idx = (blue_source.clamp(0, 3)) as usize;
        let alpha_idx = (alpha_source.clamp(0, 3)) as usize;

        let rgba = data.to_rgba8();
        let (width, height) = rgba.dimensions();
        let mut output = RgbaImage::new(width, height);

        // Remap each pixel by indexing into the source channel array
        for (x, y, pixel) in rgba.enumerate_pixels() {
            let channels = pixel.0;
            output.put_pixel(x, y, image::Rgba([
                channels[red_idx],
                channels[green_idx],
                channels[blue_idx],
                channels[alpha_idx],
            ]));
        }

        Ok(OperationResponse {
            time: Instant::now().duration_since(start_time),
            responses: vec![
                OutputResponse { value: Value::DynamicImage { data: Arc::new(image::DynamicImage::ImageRgba8(output)), change_id: get_id() } },
            ],
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use crate::get_id;
    use crate::input::Input;
    use crate::value::Value;
    use image::DynamicImage;
    use std::sync::Arc;

    fn test_image(w: u32, h: u32) -> Arc<DynamicImage> {
        let mut imgbuf = image::RgbaImage::new(w, h);
        for (x, y, pixel) in imgbuf.enumerate_pixels_mut() {
            let r = (x * 255 / w.max(1)) as u8;
            let g = (y * 255 / h.max(1)) as u8;
            *pixel = image::Rgba([r, g, 128, 255]);
        }
        Arc::new(DynamicImage::ImageRgba8(imgbuf))
    }

    fn image_input(w: u32, h: u32) -> Value {
        Value::DynamicImage { data: test_image(w, h), change_id: get_id() }
    }

    #[tokio::test]
    async fn test_shuffle_settings() {
        let s = OpImageChannelShuffle::settings();
        assert_eq!(s.name, "channel shuffle");
        assert_eq!(OpImageChannelShuffle::create_inputs().len(), 5);
        assert_eq!(OpImageChannelShuffle::create_outputs().len(), 1);
    }

    #[tokio::test]
    async fn test_shuffle_identity() {
        let mut imgbuf = image::RgbaImage::new(1, 1);
        imgbuf.put_pixel(0, 0, image::Rgba([10, 20, 30, 40]));
        let img = Arc::new(DynamicImage::ImageRgba8(imgbuf));
        let mut inputs = vec![
            Input::new("image".to_string(), Value::DynamicImage { data: img, change_id: get_id() }, None, None),
            Input::new("red source".to_string(), Value::Integer(0), None, None),
            Input::new("green source".to_string(), Value::Integer(1), None, None),
            Input::new("blue source".to_string(), Value::Integer(2), None, None),
            Input::new("alpha source".to_string(), Value::Integer(3), None, None),
        ];
        let result = OpImageChannelShuffle::run(&mut inputs).await.unwrap();
        match &result.responses[0].value {
            Value::DynamicImage { data, .. } => {
                let p = data.to_rgba8().get_pixel(0, 0).0;
                assert_eq!(p, [10, 20, 30, 40]);
            }
            other => panic!("Expected DynamicImage, got {:?}", other),
        }
    }

    #[tokio::test]
    async fn test_shuffle_swap_red_blue() {
        let mut imgbuf = image::RgbaImage::new(1, 1);
        imgbuf.put_pixel(0, 0, image::Rgba([10, 20, 30, 40]));
        let img = Arc::new(DynamicImage::ImageRgba8(imgbuf));
        let mut inputs = vec![
            Input::new("image".to_string(), Value::DynamicImage { data: img, change_id: get_id() }, None, None),
            Input::new("red source".to_string(), Value::Integer(2), None, None),
            Input::new("green source".to_string(), Value::Integer(1), None, None),
            Input::new("blue source".to_string(), Value::Integer(0), None, None),
            Input::new("alpha source".to_string(), Value::Integer(3), None, None),
        ];
        let result = OpImageChannelShuffle::run(&mut inputs).await.unwrap();
        match &result.responses[0].value {
            Value::DynamicImage { data, .. } => {
                let p = data.to_rgba8().get_pixel(0, 0).0;
                assert_eq!(p, [30, 20, 10, 40]);
            }
            other => panic!("Expected DynamicImage, got {:?}", other),
        }
    }
}
