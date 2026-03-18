//! Ambient occlusion generation from a height map.
//!
//! Approximates ambient occlusion by sampling height differences at evenly
//! spaced angles around each pixel. Higher neighboring surfaces contribute
//! more occlusion, producing darker values in concavities and crevices.

use crate::get_id;
use crate::value::ValueType;
use image::DynamicImage;
use crate::input::{Input, InputSettings};
use crate::node_settings::NodeSettings;
use crate::operations::{OperationResponse, OperationError, OutputResponse, default_image, convert_input};
use crate::output::Output;
use crate::value::Value;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use std::time::Instant;

/// Operation that computes ambient occlusion from a grayscale height map.
///
/// For each pixel, samples are taken at evenly spaced angles at the given radius.
/// The height difference (clamped to positive) divided by distance accumulates
/// occlusion, which is then scaled by intensity and subtracted from 1.0.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpImagePbrAoFromHeight {}

impl OpImagePbrAoFromHeight {
    /// Returns the node metadata (name and description) for this operation.
    pub fn settings() -> NodeSettings {
        NodeSettings {
            name: "ao from height".to_string(),
            description: "Computes ambient occlusion from a height map.".to_string(),
        }
    }

    /// Creates the default inputs: height map image, radius, intensity, and sample count.
    pub fn create_inputs() -> Vec<Input> {
        vec![
            Input::new("image".to_string(), Value::DynamicImage { data: default_image(), change_id: get_id() }, None, None),
            Input::new("radius".to_string(), Value::Integer(8), Some(InputSettings::DragValue { speed: None, clamp: Some((1.0, 64.0)) }), None),
            Input::new("intensity".to_string(), Value::Decimal(1.0), Some(InputSettings::Slider { range: (0.1, 10.0), step_by: Some(0.1), clamp_to_range: true }), None),
            Input::new("samples".to_string(), Value::Integer(16), Some(InputSettings::DragValue { speed: None, clamp: Some((4.0, 64.0)) }), None),
        ]
    }

    /// Creates the default output: a single RGBA32F ambient occlusion image.
    pub fn create_outputs() -> Vec<Output> {
        vec![
            Output::new("output".to_string(), Value::DynamicImage { data: default_image(), change_id: get_id() }, None),
        ]
    }

    /// Computes ambient occlusion from the input height map by radial sampling.
    pub async fn run(inputs: &mut Vec<Input>) -> Result<OperationResponse, OperationError> {
        let start_time = Instant::now();
        let mut input_errors: Vec<(usize, String)> = vec![];

        // convert inputs
        let image_converted = convert_input(inputs, 0, ValueType::DynamicImage, &mut input_errors);
        let radius_converted = convert_input(inputs, 1, ValueType::Integer, &mut input_errors);
        let intensity_converted = convert_input(inputs, 2, ValueType::Decimal, &mut input_errors);
        let samples_converted = convert_input(inputs, 3, ValueType::Integer, &mut input_errors);

        // return if error
        if input_errors.len() > 0 { return Err(OperationError { input_errors, node_error: None }); }

        // get values
        let Value::DynamicImage { data, change_id: _ } = image_converted.unwrap() else { unreachable!() };
        let Value::Integer(radius) = radius_converted.unwrap() else { unreachable!() };
        let Value::Decimal(intensity) = intensity_converted.unwrap() else { unreachable!() };
        let Value::Integer(samples) = samples_converted.unwrap() else { unreachable!() };

        // run node
        let buffer = data.to_rgba32f();
        let width = buffer.width() as usize;
        let height = buffer.height() as usize;
        let radius = (radius as i64).clamp(1, 64) as usize;
        let intensity = intensity as f32;
        let samples = (samples as i64).clamp(4, 64) as usize;

        // Extract luminance (Rec. 709) as height values
        let mut heights: Vec<f32> = Vec::with_capacity(width * height);
        for pixel in buffer.pixels() {
            let h = 0.2126 * pixel[0] + 0.7152 * pixel[1] + 0.0722 * pixel[2];
            heights.push(h);
        }

        let mut out_buffer = image::ImageBuffer::new(width as u32, height as u32);
        let two_pi = std::f32::consts::TAU;

        for y in 0..height {
            for x in 0..width {
                let h = heights[y * width + x];
                let mut occlusion = 0.0f32;

                for i in 0..samples {
                    let angle = i as f32 * two_pi / samples as f32;
                    let dx = angle.cos() * radius as f32;
                    let dy = angle.sin() * radius as f32;

                    // Clamp sample coordinates to image bounds
                    let sx = (x as f32 + dx).round().clamp(0.0, (width - 1) as f32) as usize;
                    let sy = (y as f32 + dy).round().clamp(0.0, (height - 1) as f32) as usize;

                    let nh = heights[sy * width + sx];
                    let dist = ((sx as f32 - x as f32).powi(2) + (sy as f32 - y as f32).powi(2)).sqrt().max(1.0);
                    let diff = (nh - h).max(0.0);
                    occlusion += diff / dist;
                }

                occlusion /= samples as f32;
                let ao = (1.0 - occlusion * intensity).clamp(0.0, 1.0);
                out_buffer.put_pixel(x as u32, y as u32, image::Rgba([ao, ao, ao, 1.0]));
            }
        }

        let result = DynamicImage::ImageRgba32F(out_buffer);

        Ok(OperationResponse {
            time: Instant::now().duration_since(start_time),
            responses: vec![
                OutputResponse { value: Value::DynamicImage { data: Arc::new(result), change_id: get_id() } },
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
    async fn test_opimagepbraofromheight_settings() {
        let s = OpImagePbrAoFromHeight::settings();
        assert_eq!(s.name, "ao from height");
        assert_eq!(OpImagePbrAoFromHeight::create_inputs().len(), 4);
        assert_eq!(OpImagePbrAoFromHeight::create_outputs().len(), 1);
    }


    #[tokio::test]
    async fn test_opimagepbraofromheight_run() {
        let mut inputs = vec![
            Input::new("img".to_string(), image_input(16, 16), None, None),
            Input::new("i1".to_string(), Value::Decimal(1.0), None, None),
            Input::new("i2".to_string(), Value::Decimal(1.0), None, None),
            Input::new("i3".to_string(), Value::Decimal(1.0), None, None)
        ];
        let result = OpImagePbrAoFromHeight::run(&mut inputs).await;
        assert!(result.is_ok(), "run failed: {:?}", result.err());
        match &result.unwrap().responses[0].value {
            Value::DynamicImage { .. } => {}
            other => panic!("Expected DynamicImage, got {:?}", other),
        }
    }

    #[tokio::test]
    async fn test_opimagepbraofromheight_1x1() {
        let mut inputs = vec![
            Input::new("image".to_string(), image_input(1, 1), None, None),
            Input::new("radius".to_string(), Value::Integer(1), None, None),
            Input::new("intensity".to_string(), Value::Decimal(1.0), None, None),
            Input::new("samples".to_string(), Value::Integer(4), None, None),
        ];
        let result = OpImagePbrAoFromHeight::run(&mut inputs).await;
        assert!(result.is_ok(), "1x1 ao_from_height failed: {:?}", result.err());
    }

    #[tokio::test]
    async fn test_opimagepbraofromheight_uniform_flat_is_white() {
        // Uniform height = no occlusion = AO should be 1.0 (white)
        let flat = Arc::new(DynamicImage::ImageRgba8(
            image::RgbaImage::from_pixel(8, 8, image::Rgba([128u8, 128, 128, 255]))
        ));
        let mut inputs = vec![
            Input::new("image".to_string(), Value::DynamicImage { data: flat, change_id: get_id() }, None, None),
            Input::new("radius".to_string(), Value::Integer(2), None, None),
            Input::new("intensity".to_string(), Value::Decimal(1.0), None, None),
            Input::new("samples".to_string(), Value::Integer(8), None, None),
        ];
        let result = OpImagePbrAoFromHeight::run(&mut inputs).await.unwrap();
        match &result.responses[0].value {
            Value::DynamicImage { data, .. } => {
                let buf = data.to_rgba32f();
                let px = buf.get_pixel(4, 4);
                // Flat surface: all neighbors at same height, no occlusion, AO = 1.0
                assert!((px[0] - 1.0).abs() < 0.01, "flat AO center should be 1.0, got {}", px[0]);
            }
            other => panic!("Expected DynamicImage, got {:?}", other),
        }
    }

    #[tokio::test]
    async fn test_opimagepbraofromheight_output_range() {
        let mut inputs = vec![
            Input::new("image".to_string(), image_input(8, 8), None, None),
            Input::new("radius".to_string(), Value::Integer(2), None, None),
            Input::new("intensity".to_string(), Value::Decimal(1.0), None, None),
            Input::new("samples".to_string(), Value::Integer(8), None, None),
        ];
        let result = OpImagePbrAoFromHeight::run(&mut inputs).await.unwrap();
        match &result.responses[0].value {
            Value::DynamicImage { data, .. } => {
                let buf = data.to_rgba32f();
                for px in buf.pixels() {
                    assert!(px[0] >= 0.0 && px[0] <= 1.0, "AO out of range: {}", px[0]);
                }
            }
            other => panic!("Expected DynamicImage, got {:?}", other),
        }
    }

}
