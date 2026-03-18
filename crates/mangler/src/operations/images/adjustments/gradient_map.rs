//! Gradient map operation for images.
//!
//! Maps each pixel's luminance to a position on a color gradient, replacing
//! the original color. Supports two-color or three-color gradients with a
//! configurable midpoint position.

use crate::color::Color;
use crate::get_id;
use crate::input::{Input, InputSettings};
use crate::node_settings::NodeSettings;
use crate::operations::{OperationResponse, OperationError, OutputResponse, default_image, convert_input};
use crate::output::Output;
use crate::value::{Value, ValueType};
use image::{DynamicImage, Rgba32FImage};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use std::time::Instant;

/// Gradient map operation that recolors an image by mapping luminance to a color gradient.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpImageAdjustmentGradientMap {}

impl OpImageAdjustmentGradientMap {
    /// Returns the node metadata (name and description) for the gradient map operation.
    pub fn settings() -> NodeSettings {
        NodeSettings {
            name: "gradient map".to_string(),
            description: "Maps image luminance to a color gradient.".to_string(),
        }
    }

    /// Creates the input ports: image, two endpoint colors (a, b), an optional mid color (c),
    /// a toggle for using the mid color, and a mid position slider.
    pub fn create_inputs() -> Vec<Input> {
        vec![
            Input::new("image".to_string(), Value::DynamicImage { data: default_image(), change_id: get_id() }, None, None),
            Input::new("color a".to_string(), Value::Color(Color::default()), None, None),
            Input::new("color b".to_string(), Value::Color(Color::from_srgb_float(1.0, 1.0, 1.0, 1.0)), None, None),
            Input::new("color c".to_string(), Value::Color(Color::from_srgb_float(0.5, 0.5, 0.5, 1.0)), None, None),
            Input::new("use mid color".to_string(), Value::Bool(false), None, None),
            Input::new("mid position".to_string(), Value::Decimal(0.5), Some(InputSettings::Slider { range: (0.0, 1.0), step_by: Some(0.01), clamp_to_range: true }), None),
        ]
    }

    /// Creates the output port: the gradient-mapped image.
    pub fn create_outputs() -> Vec<Output> {
        vec![
            Output::new("output".to_string(), Value::DynamicImage { data: default_image(), change_id: get_id() }, None),
        ]
    }

    /// Executes the gradient map. Computes Rec. 709 luminance per pixel and interpolates
    /// between gradient colors based on luminance position.
    pub async fn run(inputs: &mut Vec<Input>) -> Result<OperationResponse, OperationError> {
        let start_time = Instant::now();
        let mut input_errors: Vec<(usize, String)> = vec![];

        // convert inputs
        let image_converted = convert_input(inputs, 0, ValueType::DynamicImage, &mut input_errors);
        let color_a_converted = convert_input(inputs, 1, ValueType::Color, &mut input_errors);
        let color_b_converted = convert_input(inputs, 2, ValueType::Color, &mut input_errors);
        let color_c_converted = convert_input(inputs, 3, ValueType::Color, &mut input_errors);
        let use_mid_converted = convert_input(inputs, 4, ValueType::Bool, &mut input_errors);
        let mid_pos_converted = convert_input(inputs, 5, ValueType::Decimal, &mut input_errors);


        // return if error
        if input_errors.len() > 0 { return Err(OperationError { input_errors, node_error: None }); }

        // get values
        let Value::DynamicImage{data, change_id:_} = image_converted.unwrap() else { unreachable!() };
        let Value::Color(color_a) = color_a_converted.unwrap() else { unreachable!() };
        let Value::Color(color_b) = color_b_converted.unwrap() else { unreachable!() };
        let Value::Color(color_c) = color_c_converted.unwrap() else { unreachable!() };
        let Value::Bool(use_mid) = use_mid_converted.unwrap() else { unreachable!() };
        let Value::Decimal(mid_pos) = mid_pos_converted.unwrap() else { unreachable!() };

        // run node
        let rgba32f = data.to_rgba32f();
        let (width, height) = rgba32f.dimensions();

        let (ar, ag, ab, aa) = color_a.to_srgb_float();
        let (br, bg, bb, ba) = color_b.to_srgb_float();
        let (cr, cg, cb, ca) = color_c.to_srgb_float();

        let mut buffer = Rgba32FImage::new(width, height);

        for (x, y, pixel) in rgba32f.enumerate_pixels() {
            let r = pixel[0];
            let g = pixel[1];
            let b = pixel[2];
            let original_a = pixel[3];

            // Rec. 709 luminance
            let lum = (0.2126 * r + 0.7152 * g + 0.0722 * b).clamp(0.0, 1.0);

            let (out_r, out_g, out_b, _out_a) = if use_mid {
                // Three-color gradient: lerp A->C below midpoint, C->B above midpoint
                if lum < mid_pos as f32 {
                    let t = if mid_pos > 0.0 { lum / mid_pos as f32 } else { 0.0 };
                    (
                        ar + (cr - ar) * t,
                        ag + (cg - ag) * t,
                        ab + (cb - ab) * t,
                        aa + (ca - aa) * t,
                    )
                } else {
                    let t = if mid_pos < 1.0 { (lum - mid_pos as f32) / (1.0 - mid_pos as f32) } else { 1.0 };
                    (
                        cr + (br - cr) * t,
                        cg + (bg - cg) * t,
                        cb + (bb - cb) * t,
                        ca + (ba - ca) * t,
                    )
                }
            } else {
                // Two-color gradient: simple linear interpolation A->B
                (
                    ar + (br - ar) * lum,
                    ag + (bg - ag) * lum,
                    ab + (bb - ab) * lum,
                    aa + (ba - aa) * lum,
                )
            };

            buffer.put_pixel(x, y, image::Rgba([out_r, out_g, out_b, original_a]));
        }

        Ok(OperationResponse {
            time: Instant::now().duration_since(start_time),
            responses: vec![
                OutputResponse {value: Value::DynamicImage { data: Arc::new(DynamicImage::ImageRgba32F(buffer)), change_id: get_id() }},
            ],
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::color::Color;

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
    async fn test_gradient_map_settings() {
        let s = OpImageAdjustmentGradientMap::settings();
        assert_eq!(s.name, "gradient map");
        assert_eq!(OpImageAdjustmentGradientMap::create_inputs().len(), 6);
        assert_eq!(OpImageAdjustmentGradientMap::create_outputs().len(), 1);
    }

    #[tokio::test]
    async fn test_gradient_map_1x1() {
        let mut imgbuf = image::RgbaImage::new(1, 1);
        imgbuf.put_pixel(0, 0, image::Rgba([128u8, 128, 128, 255]));
        let img = Arc::new(DynamicImage::ImageRgba8(imgbuf));
        let mut inputs = vec![
            Input::new("image".to_string(), Value::DynamicImage { data: img, change_id: get_id() }, None, None),
            Input::new("color a".to_string(), Value::Color(Color::from_srgb_float(0.0, 0.0, 0.0, 1.0)), None, None),
            Input::new("color b".to_string(), Value::Color(Color::from_srgb_float(1.0, 1.0, 1.0, 1.0)), None, None),
            Input::new("color c".to_string(), Value::Color(Color::from_srgb_float(0.5, 0.5, 0.5, 1.0)), None, None),
            Input::new("use mid color".to_string(), Value::Bool(false), None, None),
            Input::new("mid position".to_string(), Value::Decimal(0.5), None, None),
        ];
        let result = OpImageAdjustmentGradientMap::run(&mut inputs).await;
        assert!(result.is_ok(), "gradient_map 1x1 failed: {:?}", result.err());
    }

    #[tokio::test]
    async fn test_gradient_map_with_mid_color() {
        let mut inputs = vec![
            Input::new("image".to_string(), image_input(4, 4), None, None),
            Input::new("color a".to_string(), Value::Color(Color::from_srgb_float(0.0, 0.0, 0.0, 1.0)), None, None),
            Input::new("color b".to_string(), Value::Color(Color::from_srgb_float(1.0, 1.0, 1.0, 1.0)), None, None),
            Input::new("color c".to_string(), Value::Color(Color::from_srgb_float(1.0, 0.0, 0.0, 1.0)), None, None),
            Input::new("use mid color".to_string(), Value::Bool(true), None, None),
            Input::new("mid position".to_string(), Value::Decimal(0.5), None, None),
        ];
        let result = OpImageAdjustmentGradientMap::run(&mut inputs).await.unwrap();
        match &result.responses[0].value {
            Value::DynamicImage { data, .. } => {
                assert_eq!(data.width(), 4);
                assert_eq!(data.height(), 4);
            }
            other => panic!("Expected DynamicImage, got {:?}", other),
        }
    }

    #[tokio::test]
    async fn test_gradient_map_two_color() {
        let mut inputs = vec![
            Input::new("image".to_string(), image_input(4, 4), None, None),
            Input::new("color a".to_string(), Value::Color(Color::from_srgb_float(0.0, 0.0, 0.0, 1.0)), None, None),
            Input::new("color b".to_string(), Value::Color(Color::from_srgb_float(1.0, 0.0, 0.0, 1.0)), None, None),
            Input::new("color c".to_string(), Value::Color(Color::from_srgb_float(0.5, 0.5, 0.5, 1.0)), None, None),
            Input::new("use mid color".to_string(), Value::Bool(false), None, None),
            Input::new("mid position".to_string(), Value::Decimal(0.5), None, None),
        ];
        let result = OpImageAdjustmentGradientMap::run(&mut inputs).await.unwrap();
        match &result.responses[0].value {
            Value::DynamicImage { .. } => {}
            other => panic!("Expected DynamicImage, got {:?}", other),
        }
    }
}
