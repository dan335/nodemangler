//! Curvature detection from a normal map.
//!
//! Computes surface curvature by measuring the divergence of the normal field.
//! The output encodes curvature as a grayscale value where 0.5 is flat, values
//! above 0.5 indicate convex regions (edges), and values below 0.5 indicate
//! concave regions (grooves).

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

/// Operation that detects surface curvature from a normal map.
///
/// Uses finite differences on the normal map's X and Y components to compute
/// the divergence, which approximates the surface curvature.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpImagePbrCurvature {}

impl OpImagePbrCurvature {
    /// Returns the node metadata (name and description) for this operation.
    pub fn settings() -> NodeSettings {
        NodeSettings {
            name: "curvature".to_string(),
            description: "Detects convex and concave areas from a normal map.".to_string(),
        }
    }

    /// Creates the default inputs: normal map image and intensity multiplier.
    pub fn create_inputs() -> Vec<Input> {
        vec![
            Input::new("image".to_string(), Value::DynamicImage { data: default_image(), change_id: get_id() }, None, None),
            Input::new("intensity".to_string(), Value::Decimal(1.0), Some(InputSettings::Slider { range: (0.1, 10.0), step_by: Some(0.1), clamp_to_range: true }), None),
        ]
    }

    /// Creates the default output: a single RGBA32F curvature map image.
    pub fn create_outputs() -> Vec<Output> {
        vec![
            Output::new("output".to_string(), Value::DynamicImage { data: default_image(), change_id: get_id() }, None),
        ]
    }

    /// Computes curvature from the input normal map using divergence of the normal field.
    pub async fn run(inputs: &mut [Input]) -> Result<OperationResponse, OperationError> {
        let start_time = Instant::now();
        let mut input_errors: Vec<(usize, String)> = vec![];

        // convert inputs
        let image_converted = convert_input(inputs, 0, ValueType::DynamicImage, &mut input_errors);
        let intensity_converted = convert_input(inputs, 1, ValueType::Decimal, &mut input_errors);

        // return if error
        if !input_errors.is_empty() { return Err(OperationError { input_errors, node_error: None }); }

        // get values
        let Value::DynamicImage { data, change_id: _ } = image_converted.unwrap() else { unreachable!() };
        let Value::Decimal(intensity) = intensity_converted.unwrap() else { unreachable!() };

        // run node
        let rgba = data.to_rgba32f();
        let width = rgba.width() as i32;
        let height = rgba.height() as i32;
        let intensity = intensity;

        let mut buffer = image::ImageBuffer::new(width as u32, height as u32);

        for y in 0..height {
            for x in 0..width {
                // Decode normal X/Y from the [0,1] encoded normal map at neighboring pixels
                let left_x = (x - 1).max(0);
                let right_x = (x + 1).min(width - 1);
                let top_y = (y - 1).max(0);
                let bottom_y = (y + 1).min(height - 1);

                let left_nx = rgba.get_pixel(left_x as u32, y as u32)[0] * 2.0 - 1.0;
                let right_nx = rgba.get_pixel(right_x as u32, y as u32)[0] * 2.0 - 1.0;
                let top_ny = rgba.get_pixel(x as u32, top_y as u32)[1] * 2.0 - 1.0;
                let bottom_ny = rgba.get_pixel(x as u32, bottom_y as u32)[1] * 2.0 - 1.0;

                // Compute divergence of the normal field using finite differences
                let dnx_dx = right_nx - left_nx;
                let dny_dy = bottom_ny - top_ny;
                let curvature_raw = (dnx_dx + dny_dy) * 0.5;

                // Map to output: 0.5 = flat, >0.5 = convex, <0.5 = concave
                let output = (0.5 + curvature_raw * intensity).clamp(0.0, 1.0);

                buffer.put_pixel(x as u32, y as u32, image::Rgba([output, output, output, 1.0]));
            }
        }

        let result = DynamicImage::ImageRgba32F(buffer);

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
    async fn test_opimagepbrcurvature_settings() {
        let s = OpImagePbrCurvature::settings();
        assert_eq!(s.name, "curvature");
        assert_eq!(OpImagePbrCurvature::create_inputs().len(), 2);
        assert_eq!(OpImagePbrCurvature::create_outputs().len(), 1);
    }


    #[tokio::test]
    async fn test_opimagepbrcurvature_run() {
        let mut inputs = vec![
            Input::new("img".to_string(), image_input(16, 16), None, None),
            Input::new("i1".to_string(), Value::Decimal(1.0), None, None)
        ];
        let result = OpImagePbrCurvature::run(&mut inputs).await;
        assert!(result.is_ok(), "run failed: {:?}", result.err());
        match &result.unwrap().responses[0].value {
            Value::DynamicImage { .. } => {}
            other => panic!("Expected DynamicImage, got {:?}", other),
        }
    }

    #[tokio::test]
    async fn test_opimagepbrcurvature_1x1() {
        let mut inputs = vec![
            Input::new("image".to_string(), image_input(1, 1), None, None),
            Input::new("intensity".to_string(), Value::Decimal(1.0), None, None),
        ];
        let result = OpImagePbrCurvature::run(&mut inputs).await;
        assert!(result.is_ok(), "1x1 curvature failed: {:?}", result.err());
    }

    #[tokio::test]
    async fn test_opimagepbrcurvature_flat_normal_map_is_mid() {
        // A flat normal map (0.5, 0.5 in R/G channels) -> divergence near zero -> output near 0.5
        let flat_normal = Arc::new(DynamicImage::ImageRgba8(
            image::RgbaImage::from_pixel(8, 8, image::Rgba([128u8, 128, 128, 255]))
        ));
        let mut inputs = vec![
            Input::new("image".to_string(), Value::DynamicImage { data: flat_normal, change_id: get_id() }, None, None),
            Input::new("intensity".to_string(), Value::Decimal(1.0), None, None),
        ];
        let result = OpImagePbrCurvature::run(&mut inputs).await.unwrap();
        match &result.responses[0].value {
            Value::DynamicImage { data, .. } => {
                let buf = data.to_rgba32f();
                let px = buf.get_pixel(4, 4);
                // Flat normal map: no curvature, output should be near 0.5
                assert!((px[0] - 0.5).abs() < 0.1, "flat curvature should be ~0.5, got {}", px[0]);
            }
            other => panic!("Expected DynamicImage, got {:?}", other),
        }
    }

    #[tokio::test]
    async fn test_opimagepbrcurvature_output_range() {
        let mut inputs = vec![
            Input::new("image".to_string(), image_input(8, 8), None, None),
            Input::new("intensity".to_string(), Value::Decimal(1.0), None, None),
        ];
        let result = OpImagePbrCurvature::run(&mut inputs).await.unwrap();
        match &result.responses[0].value {
            Value::DynamicImage { data, .. } => {
                let buf = data.to_rgba32f();
                for px in buf.pixels() {
                    assert!(px[0] >= 0.0 && px[0] <= 1.0, "curvature out of range: {}", px[0]);
                }
            }
            other => panic!("Expected DynamicImage, got {:?}", other),
        }
    }

}
