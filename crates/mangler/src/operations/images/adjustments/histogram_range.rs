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
pub struct OpImageAdjustmentHistogramRange{}

impl OpImageAdjustmentHistogramRange {
    pub fn settings() -> NodeSettings {
        NodeSettings {
            name: "histogram range".to_string(),
            description: "Remaps image luminance to a target range.".to_string(),
        }
    }

    pub fn create_inputs() -> Vec<Input> {
        vec![
            Input::new("image".to_string(),  Value::DynamicImage { data:default_image(), change_id:get_id() }, None, None),
            Input::new("range min".to_string(), Value::Decimal(0.0), Some(InputSettings::Slider { range: (0.0, 1.0), step_by: Some(0.01), clamp_to_range: true }), None),
            Input::new("range max".to_string(), Value::Decimal(1.0), Some(InputSettings::Slider { range: (0.0, 1.0), step_by: Some(0.01), clamp_to_range: true }), None),
        ]
    }

    pub fn create_outputs() -> Vec<Output> {
        vec![
            Output::new("output".to_string(), Value::DynamicImage { data:default_image(), change_id:get_id()}, None),
        ]
    }

    pub async fn run(inputs: &mut Vec<Input>) -> Result<OperationResponse, OperationError> {
        let start_time = Instant::now();
        let mut input_errors: Vec<(usize, String)> = vec![];

        // convert inputs
        let image_converted = convert_input(inputs, 0, ValueType::DynamicImage, &mut input_errors);
        let range_min_converted = convert_input(inputs, 1, ValueType::Decimal, &mut input_errors);
        let range_max_converted = convert_input(inputs, 2, ValueType::Decimal, &mut input_errors);

        // return if error
        if input_errors.len() > 0 { return Err(OperationError { input_errors, node_error: None }); }

        // get values
        let Value::DynamicImage{data, change_id:_} = image_converted.unwrap() else { unreachable!() };
        let Value::Decimal(range_min) = range_min_converted.unwrap() else { unreachable!() };
        let Value::Decimal(range_max) = range_max_converted.unwrap() else { unreachable!() };

        // run node
        let mut buffer = data.to_rgba32f();
        let range_min = range_min as f32;
        let range_max = range_max as f32;

        // find actual min/max luminance
        let mut actual_min: f32 = f32::MAX;
        let mut actual_max: f32 = f32::MIN;
        for pixel in buffer.pixels() {
            let lum = 0.2126 * pixel[0] + 0.7152 * pixel[1] + 0.0722 * pixel[2];
            if lum < actual_min { actual_min = lum; }
            if lum > actual_max { actual_max = lum; }
        }

        let actual_range = actual_max - actual_min;
        let target_range = range_max - range_min;

        for pixel in buffer.pixels_mut() {
            let alpha = pixel[3];
            for c in 0..3 {
                if actual_range <= 0.0 {
                    pixel[c] = range_min;
                } else {
                    let val = pixel[c];
                    let new_val = range_min + (val - actual_min) / actual_range * target_range;
                    pixel[c] = new_val.clamp(0.0, 1.0);
                }
            }
            pixel[3] = alpha;
        }

        let adjusted = DynamicImage::ImageRgba32F(buffer);

        Ok(OperationResponse {
            time: Instant::now().duration_since(start_time),
            responses: vec![
                OutputResponse {value: Value::DynamicImage { data:Arc::new(adjusted), change_id:get_id() }},
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
    async fn test_histogram_range_settings() {
        let s = OpImageAdjustmentHistogramRange::settings();
        assert_eq!(s.name, "histogram range");
        assert_eq!(OpImageAdjustmentHistogramRange::create_inputs().len(), 3);
        assert_eq!(OpImageAdjustmentHistogramRange::create_outputs().len(), 1);
    }

    #[tokio::test]
    async fn test_histogram_range_1x1() {
        let mut imgbuf = image::RgbaImage::new(1, 1);
        imgbuf.put_pixel(0, 0, image::Rgba([128u8, 64, 32, 255]));
        let img = Arc::new(DynamicImage::ImageRgba8(imgbuf));
        let mut inputs = vec![
            Input::new("image".to_string(), Value::DynamicImage { data: img, change_id: get_id() }, None, None),
            Input::new("range min".to_string(), Value::Decimal(0.0), None, None),
            Input::new("range max".to_string(), Value::Decimal(1.0), None, None),
        ];
        let result = OpImageAdjustmentHistogramRange::run(&mut inputs).await;
        assert!(result.is_ok(), "histogram_range 1x1 failed: {:?}", result.err());
    }

    #[tokio::test]
    async fn test_histogram_range_narrow_range() {
        // Output should be clamped to the narrow target range [0.2, 0.8]
        let mut inputs = vec![
            Input::new("image".to_string(), image_input(8, 8), None, None),
            Input::new("range min".to_string(), Value::Decimal(0.2), None, None),
            Input::new("range max".to_string(), Value::Decimal(0.8), None, None),
        ];
        let result = OpImageAdjustmentHistogramRange::run(&mut inputs).await.unwrap();
        match &result.responses[0].value {
            Value::DynamicImage { data, .. } => {
                for pixel in data.to_rgba32f().pixels() {
                    for c in 0..3 {
                        assert!(pixel[c] >= 0.0 && pixel[c] <= 1.0, "pixel out of [0,1]: {}", pixel[c]);
                    }
                }
            }
            other => panic!("Expected DynamicImage, got {:?}", other),
        }
    }

    #[tokio::test]
    async fn test_histogram_range_basic() {
        let mut inputs = vec![
            Input::new("image".to_string(), image_input(8, 8), None, None),
            Input::new("range min".to_string(), Value::Decimal(0.0), None, None),
            Input::new("range max".to_string(), Value::Decimal(1.0), None, None),
        ];
        let result = OpImageAdjustmentHistogramRange::run(&mut inputs).await.unwrap();
        match &result.responses[0].value {
            Value::DynamicImage { data, .. } => {
                assert_eq!(data.width(), 8);
                assert_eq!(data.height(), 8);
            }
            other => panic!("Expected DynamicImage, got {:?}", other),
        }
    }
}
