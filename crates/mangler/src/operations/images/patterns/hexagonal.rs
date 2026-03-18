//! Hexagonal tile pattern image generator.
//!
//! Generates a flat-top hexagonal tile pattern as a grayscale image using
//! axial/cube coordinate rounding to find the nearest hex center, then
//! computing the hexagonal distance for gap detection.

use image::{ImageBuffer, DynamicImage};
use crate::get_id;
use crate::input::{Input, InputSettings};
use crate::node_settings::NodeSettings;
use crate::operations::{OperationResponse, OperationError, OutputResponse, default_image, convert_input};
use crate::output::Output;
use crate::value::{Value, ValueType};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use std::time::Instant;

/// Operation that generates a hexagonal tile pattern as a grayscale image.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpImagePatternHexagonal {}

impl OpImagePatternHexagonal {
    /// Returns the node metadata (name and description) for this operation.
    pub fn settings() -> NodeSettings {
        NodeSettings {
            name: "hexagonal".to_string(),
            description: "Generates a hexagonal tile pattern.".to_string(),
        }
    }

    /// Creates the default inputs: width, height, scale, and gap_size.
    pub fn create_inputs() -> Vec<Input> {
        vec![
            Input::new("width".to_string(), Value::Integer(512), Some(InputSettings::DragValue { clamp: Some((1.0, 10000.0)), speed: None }), None),
            Input::new("height".to_string(), Value::Integer(512), Some(InputSettings::DragValue { clamp: Some((1.0, 10000.0)), speed: None }), None),
            Input::new("scale".to_string(), Value::Decimal(10.0), Some(InputSettings::Slider { range: (1.0, 64.0), step_by: None, clamp_to_range: false }), None),
            Input::new("gap_size".to_string(), Value::Decimal(0.05), Some(InputSettings::Slider { range: (0.0, 0.5), step_by: None, clamp_to_range: true }), None),
        ]
    }

    /// Creates the default output: a single grayscale image.
    pub fn create_outputs() -> Vec<Output> {
        vec![
            Output::new("output".to_string(), Value::DynamicImage { data: default_image(), change_id: get_id() }, None),
        ]
    }

    /// Generates a hexagonal tile pattern image from the given inputs.
    pub async fn run(inputs: &mut Vec<Input>) -> Result<OperationResponse, OperationError> {
        let start_time = Instant::now();
        let mut input_errors: Vec<(usize, String)> = vec![];

        // convert inputs
        let width_converted = convert_input(inputs, 0, ValueType::Integer, &mut input_errors);
        let height_converted = convert_input(inputs, 1, ValueType::Integer, &mut input_errors);
        let scale_converted = convert_input(inputs, 2, ValueType::Decimal, &mut input_errors);
        let gap_size_converted = convert_input(inputs, 3, ValueType::Decimal, &mut input_errors);

        // return if error
        if input_errors.len() > 0 { return Err(OperationError { input_errors, node_error: None }); }

        // get values
        let Value::Integer(mut width) = width_converted.unwrap() else { unreachable!() };
        let Value::Integer(mut height) = height_converted.unwrap() else { unreachable!() };
        let Value::Decimal(scale) = scale_converted.unwrap() else { unreachable!() };
        let Value::Decimal(gap_size) = gap_size_converted.unwrap() else { unreachable!() };

        // run node
        width = width.max(1);
        height = height.max(1);
        let scale = (scale as f64).max(0.1);
        let gap_size = (gap_size as f64).clamp(0.0, 0.5);

        let size = width.max(height) as f64;
        let sqrt3 = 3.0_f64.sqrt();

        let mut image_buffer = ImageBuffer::new(width as u32, height as u32);

        for py in 0..height {
            for px in 0..width {
                // normalize pixel coordinates and scale into hex space
                let x = (px as f64 / size) * scale;
                let y = (py as f64 / size) * scale;

                // convert to axial hex coordinates
                // hex size = 1.0, flat-top hexagons
                let q = (2.0 / 3.0) * x;
                let r = (-1.0 / 3.0) * x + (sqrt3 / 3.0) * y;

                // round to nearest hex center (cube coordinate rounding)
                let s = -q - r;
                let mut rq = q.round();
                let mut rr = r.round();
                let rs = s.round();

                let q_diff = (rq - q).abs();
                let r_diff = (rr - r).abs();
                let s_diff = (rs - s).abs();

                if q_diff > r_diff && q_diff > s_diff {
                    rq = -rr - rs;
                } else if r_diff > s_diff {
                    rr = -rq - rs;
                }

                // convert hex center back to pixel space
                let cx = 1.5 * rq;
                let cy = sqrt3 * (rr + rq / 2.0);

                // compute distance to hex edge
                // for a flat-top hex with size 1, the distance from center to edge
                let dx = (x - cx).abs();
                let dy = (y - cy).abs();

                // hex edge distance using the hexagonal norm
                let hex_dist = dx.abs().max((dx * 0.5 + dy * sqrt3 / 2.0).abs());
                // normalize: the hex edge is at distance 1.0 from center
                let edge_proximity = hex_dist; // 0 at center, ~1 at edge

                let in_gap = edge_proximity > (1.0 - gap_size);

                let g: u8 = if in_gap { 0 } else { 255 };
                image_buffer.put_pixel(px as u32, py as u32, image::Luma([g]));
            }
        }

        let dynamic_image = DynamicImage::ImageLuma8(image_buffer);

        Ok(OperationResponse {
            time: Instant::now().duration_since(start_time),
            responses: vec![
                OutputResponse { value: Value::DynamicImage { data: Arc::new(dynamic_image), change_id: get_id() } },
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
    async fn test_opimagepatternhexagonal_settings() {
        let s = OpImagePatternHexagonal::settings();
        assert_eq!(s.name, "hexagonal");
        assert_eq!(OpImagePatternHexagonal::create_inputs().len(), 4);
        assert_eq!(OpImagePatternHexagonal::create_outputs().len(), 1);
    }


    #[tokio::test]
    async fn test_opimagepatternhexagonal_run() {
        let mut inputs = vec![
            Input::new("i0".to_string(), Value::Integer(4), None, None),
            Input::new("i1".to_string(), Value::Integer(4), None, None),
            Input::new("i2".to_string(), Value::Integer(4), None, None),
            Input::new("i3".to_string(), Value::Integer(4), None, None)
        ];
        let result = OpImagePatternHexagonal::run(&mut inputs).await;
        assert!(result.is_ok(), "run failed: {:?}", result.err());
        match &result.unwrap().responses[0].value {
            Value::DynamicImage { .. } => {}
            other => panic!("Expected DynamicImage, got {:?}", other),
        }
    }

    #[tokio::test]
    async fn test_opimagepatternhexagonal_correct_dimensions() {
        let mut inputs = vec![
            Input::new("width".to_string(), Value::Integer(16), None, None),
            Input::new("height".to_string(), Value::Integer(8), None, None),
            Input::new("scale".to_string(), Value::Decimal(0.2), None, None),
            Input::new("gap_size".to_string(), Value::Decimal(0.05), None, None),
        ];
        let result = OpImagePatternHexagonal::run(&mut inputs).await.unwrap();
        match &result.responses[0].value {
            Value::DynamicImage { data, .. } => {
                assert_eq!(data.width(), 16);
                assert_eq!(data.height(), 8);
            }
            other => panic!("Expected DynamicImage, got {:?}", other),
        }
    }

}
