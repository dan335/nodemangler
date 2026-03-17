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
use noise::{NoiseFn, Billow, MultiFractal, Perlin};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpImageNoiseBillow {}

impl OpImageNoiseBillow {
    pub fn settings() -> NodeSettings {
        NodeSettings {
            name: "billow noise".to_string(),
            description: "Creates a billowy noise image.".to_string(),
        }
    }

    pub fn create_inputs() -> Vec<Input> {
        vec![
            Input::new("seed".to_string(), Value::Integer(1), Some(InputSettings::DragValue { clamp: None, speed: None }), None),
            Input::new("width".to_string(), Value::Integer(512), Some(InputSettings::DragValue {clamp:Some((1.0,10000.0)), speed: None }), None),
            Input::new("height".to_string(), Value::Integer(512), Some(InputSettings::DragValue {clamp:Some((1.0,10000.0)), speed: None }), None),
            Input::new("octaves".to_string(), Value::Integer(6), Some(InputSettings::Slider { range: (0.0, 32.0), step_by: Some(1.0), clamp_to_range: true }), None),
            Input::new("frequency".to_string(), Value::Decimal(5.0), Some(InputSettings::DragValue { clamp: None, speed: Some(0.01) }), None),
            Input::new("lacunarity".to_string(), Value::Decimal(2.0943951023931953), Some(InputSettings::DragValue { clamp: None, speed: Some(0.01) }), None),
            Input::new("persitence".to_string(), Value::Decimal(0.5), Some(InputSettings::DragValue { clamp: None, speed: Some(0.01) }), None),
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
        let seed_converted = convert_input(inputs, 0, ValueType::Integer, &mut input_errors);
        let width_converted = convert_input(inputs, 1, ValueType::Integer, &mut input_errors);
        let height_converted = convert_input(inputs, 2, ValueType::Integer, &mut input_errors);
        let octaves_converted = convert_input(inputs, 3, ValueType::Integer, &mut input_errors);
        let frequency_converted = convert_input(inputs, 4, ValueType::Decimal, &mut input_errors);
        let lacunarity_converted = convert_input(inputs, 5, ValueType::Decimal, &mut input_errors);
        let persistence_converted = convert_input(inputs, 6, ValueType::Decimal, &mut input_errors);


        // return if error
        if input_errors.len() > 0 { return Err(OperationError { input_errors, node_error: None }); }

        // get values
        let Value::Integer(mut seed) = seed_converted.unwrap() else { unreachable!() };
        let Value::Integer(mut width) = width_converted.unwrap() else { unreachable!() };
        let Value::Integer(mut height) = height_converted.unwrap() else { unreachable!() };
        let Value::Integer(octaves) = octaves_converted.unwrap() else { unreachable!() };
        let Value::Decimal(frequency) = frequency_converted.unwrap() else { unreachable!() };
        let Value::Decimal(lacunarity) = lacunarity_converted.unwrap() else { unreachable!() };
        let Value::Decimal(persistence) = persistence_converted.unwrap() else { unreachable!() };
        
        // run node
        width = width.max(1);
        height = height.max(1);
        seed = seed.max(1);
        //scale = scale.max(0.0001);

        let mut image_buffer = ImageBuffer::new(width as u32, height as u32);

        let basicmulti = Billow::<Perlin>::new(seed as u32).set_frequency(frequency as f64).set_octaves(octaves as usize).set_lacunarity(lacunarity as f64).set_persistence(persistence as f64);

        for x in 0..width {
            for y in 0..height {
                let size = width.max(height) as f64;
                let coords_x = (x as f64) / (size as f64);
                let coords_y = (y as f64) / (size as f64);
                let noise = basicmulti.get([coords_x, coords_y]) as f32 * 0.5 + 0.5;
                let non_linear = crate::color::color_spaces::rgb_linear::linear_to_nonlinear_srgb(noise);
                let g = (non_linear * 255.0) as u8;
                image_buffer.put_pixel(x as u32, y as u32, image::Luma([g]));
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
    async fn test_opimagenoisebillow_settings() {
        let s = OpImageNoiseBillow::settings();
        assert_eq!(s.name, "billow noise");
        assert_eq!(OpImageNoiseBillow::create_inputs().len(), 7);
        assert_eq!(OpImageNoiseBillow::create_outputs().len(), 1);
    }


    #[tokio::test]
    async fn test_opimagenoisebillow_run() {
        let mut inputs = vec![
            Input::new("i0".to_string(), Value::Integer(4), None, None),
            Input::new("i1".to_string(), Value::Integer(4), None, None),
            Input::new("i2".to_string(), Value::Integer(4), None, None),
            Input::new("i3".to_string(), Value::Integer(4), None, None),
            Input::new("i4".to_string(), Value::Integer(4), None, None),
            Input::new("i5".to_string(), Value::Integer(4), None, None),
            Input::new("i6".to_string(), Value::Integer(4), None, None)
        ];
        let result = OpImageNoiseBillow::run(&mut inputs).await;
        assert!(result.is_ok(), "run failed: {:?}", result.err());
        match &result.unwrap().responses[0].value {
            Value::DynamicImage { .. } => {}
            other => panic!("Expected DynamicImage, got {:?}", other),
        }
    }

    #[tokio::test]
    async fn test_opimagenoisebillow_correct_dimensions() {
        let mut inputs = vec![
            Input::new("seed".to_string(), Value::Integer(1), None, None),
            Input::new("width".to_string(), Value::Integer(16), None, None),
            Input::new("height".to_string(), Value::Integer(8), None, None),
            Input::new("octaves".to_string(), Value::Integer(4), None, None),
            Input::new("frequency".to_string(), Value::Decimal(5.0), None, None),
            Input::new("lacunarity".to_string(), Value::Decimal(2.0), None, None),
            Input::new("persistence".to_string(), Value::Decimal(0.5), None, None),
        ];
        let result = OpImageNoiseBillow::run(&mut inputs).await.unwrap();
        match &result.responses[0].value {
            Value::DynamicImage { data, .. } => {
                assert_eq!(data.width(), 16);
                assert_eq!(data.height(), 8);
            }
            other => panic!("Expected DynamicImage, got {:?}", other),
        }
    }

    #[tokio::test]
    async fn test_opimagenoisebillow_deterministic() {
        let make_inputs = || vec![
            Input::new("seed".to_string(), Value::Integer(42), None, None),
            Input::new("width".to_string(), Value::Integer(8), None, None),
            Input::new("height".to_string(), Value::Integer(8), None, None),
            Input::new("octaves".to_string(), Value::Integer(3), None, None),
            Input::new("frequency".to_string(), Value::Decimal(5.0), None, None),
            Input::new("lacunarity".to_string(), Value::Decimal(2.0), None, None),
            Input::new("persistence".to_string(), Value::Decimal(0.5), None, None),
        ];
        let r1 = OpImageNoiseBillow::run(&mut make_inputs()).await.unwrap();
        let r2 = OpImageNoiseBillow::run(&mut make_inputs()).await.unwrap();
        match (&r1.responses[0].value, &r2.responses[0].value) {
            (Value::DynamicImage { data: d1, .. }, Value::DynamicImage { data: d2, .. }) => {
                let buf1 = d1.to_luma8();
                let buf2 = d2.to_luma8();
                assert_eq!(buf1.pixels().collect::<Vec<_>>(),
                           buf2.pixels().collect::<Vec<_>>(),
                           "billow noise is not deterministic");
            }
            _ => panic!("Expected DynamicImage"),
        }
    }

}
