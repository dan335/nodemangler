use image::{RgbaImage, ImageBuffer, DynamicImage};
use crate::color::Color;
use crate::color::color_spaces::rgb_linear::{nonlinear_to_linear_rgb, linear_to_nonlinear_srgb};
use crate::get_id;
use crate::input::{Input, InputSettings};
use crate::node_settings::NodeSettings;
use crate::operations::{OperationResponse, OperationError, OutputResponse, default_image};
use crate::output::Output;
use crate::value::{Value, ValueType};
use serde::{Deserialize, Serialize};
use std::time::Instant;
use noise::{NoiseFn, Perlin, Seedable, Checkerboard};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpImageNoiseCheckerboard {}

impl OpImageNoiseCheckerboard {
    pub fn settings() -> NodeSettings {
        NodeSettings {
            name: "checkerboard noise".to_string(),
            description: "Creates a checkerboard noise image.".to_string(),
        }
    }

    pub fn create_inputs() -> Vec<Input> {
        vec![
            Input::new("width".to_string(), Value::Integer(512), Some(InputSettings::DragValue {clamp:Some((1.0,10000.0)), speed: None }), None),
            Input::new("height".to_string(), Value::Integer(512), Some(InputSettings::DragValue {clamp:Some((1.0,10000.0)), speed: None }), None),
            Input::new("size".to_string(), Value::Integer(10), Some(InputSettings::DragValue { clamp: None, speed: None }), None),
        ]
    }

    pub fn create_outputs() -> Vec<Output> {
        vec![
            Output::new("output".to_string(), Value::DynamicImage { data:default_image(), change_id:get_id() }, None),
        ]
    }

    pub async fn run(inputs: &mut Vec<Input>) -> Result<OperationResponse, OperationError> {
        let start_time = Instant::now();
        let mut input_errors: Vec<(usize, String)> = vec![];

        // convert inputs
        let width_converted = inputs[0].value.try_convert_to(ValueType::Integer);
        let height_converted = inputs[1].value.try_convert_to(ValueType::Integer);
        let size_converted = inputs[2].value.try_convert_to(ValueType::Integer);
        
        // gather errors

        // return if error
        if input_errors.len() > 0 { return Err(OperationError { input_errors, node_error: None }); }

        // get values
        // run node

        let Ok(Value::Integer(mut width)) = inputs[0].value.try_convert_to(ValueType::Integer) else { return Err(OperationError { input_errors: vec![(0, "Unable to convert to integer.".to_string())], node_error: None })};
        let Ok(Value::Integer(mut height)) = inputs[1].value.try_convert_to(ValueType::Integer) else { return Err(OperationError { input_errors: vec![(1, "Unable to convert to integer.".to_string())], node_error: None })};
        let Ok(Value::Integer(mut size)) = inputs[2].value.try_convert_to(ValueType::Integer) else { return Err(OperationError { input_errors: vec![(2, "Unable to convert to integer.".to_string())], node_error: None })};
        
        width = width.max(1);
        height = height.max(1);
        size = size.max(1);

        let mut image_buffer = ImageBuffer::new(width as u32, height as u32);

        let perlin = Checkerboard::new(size as usize);

        for x in 0..width {
            for y in 0..height {
                let size = width.max(height) as f64;
                let coords_x = (x as f64) / (size as f64);
                let coords_y = (y as f64) / (size as f64);
                let noise = perlin.get([coords_x, coords_y]) as f32 * 0.5 + 0.5;
                let non_linear = linear_to_nonlinear_srgb(noise);
                let g = (non_linear * 255.0) as u8;
                image_buffer.put_pixel(x as u32, y as u32, image::Luma([g]));
            }
        }
        
        let dynamic_image = DynamicImage::ImageLuma8(image_buffer);

        Ok(OperationResponse {
            time: Instant::now().duration_since(start_time),
            responses: vec![
                OutputResponse { value: Value::DynamicImage { data: dynamic_image, change_id: get_id() } },
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
    use image::{DynamicImage, RgbaImage};
    use std::sync::Arc;

    fn test_image(w: u32, h: u32) -> Arc<DynamicImage> {
        let mut img = RgbaImage::new(w, h);
        for y in 0..h {
            for x in 0..w {
                let r = ((x as f32 / w as f32) * 255.0) as u8;
                let g = ((y as f32 / h as f32) * 255.0) as u8;
                img.put_pixel(x, y, image::Rgba([r, g, 128, 255]));
            }
        }
        Arc::new(DynamicImage::ImageRgba8(img))
    }

    fn image_input(w: u32, h: u32) -> Value {
        Value::DynamicImage { data: test_image(w, h), change_id: get_id() }
    }


    #[tokio::test]
    async fn test_opimagenoisecheckerboard_settings() {
        let s = OpImageNoiseCheckerboard::settings();
        assert_eq!(s.name, "checkerboard noise");
        assert_eq!(OpImageNoiseCheckerboard::create_inputs().len(), 3);
        assert_eq!(OpImageNoiseCheckerboard::create_outputs().len(), 1);
    }


    #[tokio::test]
    async fn test_opimagenoisecheckerboard_run() {
        let mut inputs = vec![
            Input::new("i0".to_string(), Value::Integer(4), None, None),
            Input::new("i1".to_string(), Value::Integer(4), None, None),
            Input::new("i2".to_string(), Value::Integer(4), None, None)
        ];
        let result = OpImageNoiseCheckerboard::run(&mut inputs).await;
        assert!(result.is_ok(), "run failed: {:?}", result.err());
        match &result.unwrap().responses[0].value {
            Value::DynamicImage { .. } => {}
            other => panic!("Expected DynamicImage, got {:?}", other),
        }
    }

    #[tokio::test]
    async fn test_opimagenoisecheckerboard_1x1() {
        let mut inputs = vec![
            Input::new("width".to_string(), Value::Integer(1), None, None),
            Input::new("height".to_string(), Value::Integer(1), None, None),
            Input::new("size".to_string(), Value::Integer(1), None, None),
        ];
        let result = OpImageNoiseCheckerboard::run(&mut inputs).await;
        assert!(result.is_ok(), "checkerboard 1x1 failed: {:?}", result.err());
    }

    #[tokio::test]
    async fn test_opimagenoisecheckerboard_correct_dimensions() {
        let mut inputs = vec![
            Input::new("width".to_string(), Value::Integer(16), None, None),
            Input::new("height".to_string(), Value::Integer(8), None, None),
            Input::new("size".to_string(), Value::Integer(4), None, None),
        ];
        let result = OpImageNoiseCheckerboard::run(&mut inputs).await.unwrap();
        match &result.responses[0].value {
            Value::DynamicImage { data, .. } => {
                assert_eq!(data.width(), 16);
                assert_eq!(data.height(), 8);
            }
            other => panic!("Expected DynamicImage, got {:?}", other),
        }
    }

    #[tokio::test]
    async fn test_opimagenoisecheckerboard_deterministic() {
        // Same inputs should give identical outputs
        let make_inputs = || vec![
            Input::new("width".to_string(), Value::Integer(8), None, None),
            Input::new("height".to_string(), Value::Integer(8), None, None),
            Input::new("size".to_string(), Value::Integer(2), None, None),
        ];
        let r1 = OpImageNoiseCheckerboard::run(&mut make_inputs()).await.unwrap();
        let r2 = OpImageNoiseCheckerboard::run(&mut make_inputs()).await.unwrap();
        match (&r1.responses[0].value, &r2.responses[0].value) {
            (Value::DynamicImage { data: d1, .. }, Value::DynamicImage { data: d2, .. }) => {
                assert_eq!(d1.to_luma8().as_raw(), d2.to_luma8().as_raw(), "checkerboard should be deterministic");
            }
            _ => panic!("Expected DynamicImage"),
        }
    }

}
