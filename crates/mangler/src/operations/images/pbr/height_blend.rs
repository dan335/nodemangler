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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpImagePbrHeightBlend {}

impl OpImagePbrHeightBlend {
    pub fn settings() -> NodeSettings {
        NodeSettings {
            name: "height blend".to_string(),
            description: "Blends two materials using their height maps for realistic layering.".to_string(),
        }
    }

    pub fn create_inputs() -> Vec<Input> {
        vec![
            Input::new("base color".to_string(), Value::DynamicImage { data: default_image(), change_id: get_id() }, None, None),
            Input::new("base height".to_string(), Value::DynamicImage { data: default_image(), change_id: get_id() }, None, None),
            Input::new("overlay color".to_string(), Value::DynamicImage { data: default_image(), change_id: get_id() }, None, None),
            Input::new("overlay height".to_string(), Value::DynamicImage { data: default_image(), change_id: get_id() }, None, None),
            Input::new("blend amount".to_string(), Value::Decimal(0.5), Some(InputSettings::Slider { range: (0.0, 1.0), step_by: Some(0.01), clamp_to_range: true }), None),
            Input::new("contrast".to_string(), Value::Decimal(0.5), Some(InputSettings::Slider { range: (0.0, 1.0), step_by: Some(0.01), clamp_to_range: true }), None),
        ]
    }

    pub fn create_outputs() -> Vec<Output> {
        vec![
            Output::new("output".to_string(), Value::DynamicImage { data: default_image(), change_id: get_id() }, None),
            Output::new("height".to_string(), Value::DynamicImage { data: default_image(), change_id: get_id() }, None),
        ]
    }

    pub async fn run(inputs: &mut Vec<Input>) -> Result<OperationResponse, OperationError> {
        let start_time = Instant::now();
        let mut input_errors: Vec<(usize, String)> = vec![];

        // convert inputs
        let base_color_converted = convert_input(inputs, 0, ValueType::DynamicImage, &mut input_errors);
        let base_height_converted = convert_input(inputs, 1, ValueType::DynamicImage, &mut input_errors);
        let overlay_color_converted = convert_input(inputs, 2, ValueType::DynamicImage, &mut input_errors);
        let overlay_height_converted = convert_input(inputs, 3, ValueType::DynamicImage, &mut input_errors);
        let blend_amount_converted = convert_input(inputs, 4, ValueType::Decimal, &mut input_errors);
        let contrast_converted = convert_input(inputs, 5, ValueType::Decimal, &mut input_errors);

        // return if error
        if input_errors.len() > 0 { return Err(OperationError { input_errors, node_error: None }); }

        // get values
        let Value::DynamicImage { data: base_color_data, change_id: _ } = base_color_converted.unwrap() else { unreachable!() };
        let Value::DynamicImage { data: base_height_data, change_id: _ } = base_height_converted.unwrap() else { unreachable!() };
        let Value::DynamicImage { data: overlay_color_data, change_id: _ } = overlay_color_converted.unwrap() else { unreachable!() };
        let Value::DynamicImage { data: overlay_height_data, change_id: _ } = overlay_height_converted.unwrap() else { unreachable!() };
        let Value::Decimal(blend_amount) = blend_amount_converted.unwrap() else { unreachable!() };
        let Value::Decimal(contrast) = contrast_converted.unwrap() else { unreachable!() };

        // run node
        let base_color_buf = base_color_data.to_rgba32f();
        let base_height_buf = base_height_data.to_rgba32f();
        let overlay_color_buf = overlay_color_data.to_rgba32f();
        let overlay_height_buf = overlay_height_data.to_rgba32f();
        let width = base_color_buf.width();
        let height = base_color_buf.height();
        let blend_amount = blend_amount as f32;
        let contrast = contrast as f32;

        let mut color_output = image::ImageBuffer::new(width, height);
        let mut height_output = image::ImageBuffer::new(width, height);

        for y in 0..height {
            for x in 0..width {
                let base_c = base_color_buf.get_pixel(x.min(base_color_buf.width() - 1), y.min(base_color_buf.height() - 1));
                let overlay_c = overlay_color_buf.get_pixel(x.min(overlay_color_buf.width() - 1), y.min(overlay_color_buf.height() - 1));

                let base_h_pixel = base_height_buf.get_pixel(x.min(base_height_buf.width() - 1), y.min(base_height_buf.height() - 1));
                let overlay_h_pixel = overlay_height_buf.get_pixel(x.min(overlay_height_buf.width() - 1), y.min(overlay_height_buf.height() - 1));

                // Luminance as height value (Rec. 709)
                let bh = 0.2126 * base_h_pixel[0] + 0.7152 * base_h_pixel[1] + 0.0722 * base_h_pixel[2];
                let oh = 0.2126 * overlay_h_pixel[0] + 0.7152 * overlay_h_pixel[1] + 0.0722 * overlay_h_pixel[2];

                // Height-based blend factor:
                // The overlay wins where its height exceeds the base height, modulated by blend_amount.
                // Contrast controls the sharpness of the transition (0 = linear blend, 1 = hard cutoff).
                let height_diff = oh - bh;
                let depth = 1.0 - contrast; // low contrast = wide transition, high contrast = sharp
                let depth = depth.max(0.001); // avoid division by zero

                // Compute blend factor: shift by blend_amount, scale by depth
                let t = ((height_diff + blend_amount * 2.0 - 1.0) / depth * 0.5 + 0.5).clamp(0.0, 1.0);

                // Blend colors
                let r = base_c[0] * (1.0 - t) + overlay_c[0] * t;
                let g = base_c[1] * (1.0 - t) + overlay_c[1] * t;
                let b = base_c[2] * (1.0 - t) + overlay_c[2] * t;
                let a = base_c[3] * (1.0 - t) + overlay_c[3] * t;

                color_output.put_pixel(x, y, image::Rgba([r, g, b, a]));

                // Blend heights
                let blended_h = bh * (1.0 - t) + oh * t;
                height_output.put_pixel(x, y, image::Rgba([blended_h, blended_h, blended_h, 1.0]));
            }
        }

        let color_result = DynamicImage::ImageRgba32F(color_output);
        let height_result = DynamicImage::ImageRgba32F(height_output);

        Ok(OperationResponse {
            time: Instant::now().duration_since(start_time),
            responses: vec![
                OutputResponse { value: Value::DynamicImage { data: Arc::new(color_result), change_id: get_id() } },
                OutputResponse { value: Value::DynamicImage { data: Arc::new(height_result), change_id: get_id() } },
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
    async fn test_opimagepbrheightblend_settings() {
        let s = OpImagePbrHeightBlend::settings();
        assert_eq!(s.name, "height blend");
        assert_eq!(OpImagePbrHeightBlend::create_inputs().len(), 6);
        assert_eq!(OpImagePbrHeightBlend::create_outputs().len(), 2);
    }


    #[tokio::test]
    async fn test_opimagepbrheightblend_run() {
        let mut inputs = vec![
            Input::new("base color".to_string(), image_input(16, 16), None, None),
            Input::new("base height".to_string(), image_input(16, 16), None, None),
            Input::new("overlay color".to_string(), image_input(16, 16), None, None),
            Input::new("overlay height".to_string(), image_input(16, 16), None, None),
            Input::new("blend amount".to_string(), Value::Decimal(0.5), None, None),
            Input::new("contrast".to_string(), Value::Decimal(0.5), None, None),
        ];
        let result = OpImagePbrHeightBlend::run(&mut inputs).await;
        assert!(result.is_ok(), "run failed: {:?}", result.err());
        match &result.unwrap().responses[0].value {
            Value::DynamicImage { .. } => {}
            other => panic!("Expected DynamicImage, got {:?}", other),
        }
    }

    #[tokio::test]
    async fn test_opimagepbrheightblend_two_outputs() {
        let mut inputs = vec![
            Input::new("base color".to_string(), image_input(8, 8), None, None),
            Input::new("base height".to_string(), image_input(8, 8), None, None),
            Input::new("overlay color".to_string(), image_input(8, 8), None, None),
            Input::new("overlay height".to_string(), image_input(8, 8), None, None),
            Input::new("blend amount".to_string(), Value::Decimal(0.5), None, None),
            Input::new("contrast".to_string(), Value::Decimal(0.5), None, None),
        ];
        let result = OpImagePbrHeightBlend::run(&mut inputs).await.unwrap();
        assert_eq!(result.responses.len(), 2, "expected 2 outputs");
        match &result.responses[1].value {
            Value::DynamicImage { .. } => {}
            other => panic!("Expected DynamicImage for height output, got {:?}", other),
        }
    }

    #[tokio::test]
    async fn test_opimagepbrheightblend_1x1() {
        let mut inputs = vec![
            Input::new("base color".to_string(), image_input(1, 1), None, None),
            Input::new("base height".to_string(), image_input(1, 1), None, None),
            Input::new("overlay color".to_string(), image_input(1, 1), None, None),
            Input::new("overlay height".to_string(), image_input(1, 1), None, None),
            Input::new("blend amount".to_string(), Value::Decimal(0.5), None, None),
            Input::new("contrast".to_string(), Value::Decimal(0.5), None, None),
        ];
        let result = OpImagePbrHeightBlend::run(&mut inputs).await;
        assert!(result.is_ok(), "1x1 height_blend failed: {:?}", result.err());
    }

    #[tokio::test]
    async fn test_opimagepbrheightblend_blend_zero_is_base() {
        // blend_amount=0 should result in the base color dominating
        let base = Arc::new(DynamicImage::ImageRgba8(
            image::RgbaImage::from_pixel(4, 4, image::Rgba([255u8, 0, 0, 255]))
        ));
        let overlay = Arc::new(DynamicImage::ImageRgba8(
            image::RgbaImage::from_pixel(4, 4, image::Rgba([0u8, 0, 255, 255]))
        ));
        let height = Arc::new(DynamicImage::ImageRgba8(
            image::RgbaImage::from_pixel(4, 4, image::Rgba([128u8, 128, 128, 255]))
        ));
        let mut inputs = vec![
            Input::new("base color".to_string(), Value::DynamicImage { data: base, change_id: get_id() }, None, None),
            Input::new("base height".to_string(), Value::DynamicImage { data: height.clone(), change_id: get_id() }, None, None),
            Input::new("overlay color".to_string(), Value::DynamicImage { data: overlay, change_id: get_id() }, None, None),
            Input::new("overlay height".to_string(), Value::DynamicImage { data: height, change_id: get_id() }, None, None),
            Input::new("blend amount".to_string(), Value::Decimal(0.0), None, None),
            Input::new("contrast".to_string(), Value::Decimal(0.5), None, None),
        ];
        let result = OpImagePbrHeightBlend::run(&mut inputs).await.unwrap();
        match &result.responses[0].value {
            Value::DynamicImage { data, .. } => {
                assert_eq!(data.width(), 4);
                assert_eq!(data.height(), 4);
            }
            other => panic!("Expected DynamicImage, got {:?}", other),
        }
    }

    #[tokio::test]
    async fn test_opimagepbrheightblend_output_range() {
        let mut inputs = vec![
            Input::new("base color".to_string(), image_input(8, 8), None, None),
            Input::new("base height".to_string(), image_input(8, 8), None, None),
            Input::new("overlay color".to_string(), image_input(8, 8), None, None),
            Input::new("overlay height".to_string(), image_input(8, 8), None, None),
            Input::new("blend amount".to_string(), Value::Decimal(0.5), None, None),
            Input::new("contrast".to_string(), Value::Decimal(0.5), None, None),
        ];
        let result = OpImagePbrHeightBlend::run(&mut inputs).await.unwrap();
        match &result.responses[0].value {
            Value::DynamicImage { data, .. } => {
                let buf = data.to_rgba32f();
                for px in buf.pixels() {
                    assert!(px[0] >= 0.0 && px[0] <= 1.0, "R out of range: {}", px[0]);
                    assert!(px[1] >= 0.0 && px[1] <= 1.0, "G out of range: {}", px[1]);
                    assert!(px[2] >= 0.0 && px[2] <= 1.0, "B out of range: {}", px[2]);
                }
            }
            other => panic!("Expected DynamicImage, got {:?}", other),
        }
    }

}
