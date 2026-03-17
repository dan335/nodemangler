use image::{ImageBuffer, DynamicImage};
use crate::get_id;
use crate::input::{Input, InputSettings};
use crate::node_settings::NodeSettings;
use crate::operations::{OperationResponse, OperationError, OutputResponse, default_image};
use crate::output::Output;
use crate::value::{Value, ValueType};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use std::time::Instant;
use noise::{NoiseFn, MultiFractal, Perlin, HybridMulti};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpImageNoiseHybridMultifractalNoise {}

impl OpImageNoiseHybridMultifractalNoise {
    pub fn settings() -> NodeSettings {
        NodeSettings {
            name: "hybrid multifractal noise".to_string(),
            description: "Noise function that outputs hybrid Multifractal noise. The result of this multifractal noise is that valleys in the noise should have smooth bottoms at all altitudes.".to_string(),
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
        let seed_converted = inputs[0].value.try_convert_to(ValueType::Integer);
        let width_converted = inputs[1].value.try_convert_to(ValueType::Integer);
        let height_converted = inputs[2].value.try_convert_to(ValueType::Integer);
        let octaves_converted = inputs[3].value.try_convert_to(ValueType::Integer);
        let frequency_converted = inputs[4].value.try_convert_to(ValueType::Decimal);
        let lacunarity_converted = inputs[5].value.try_convert_to(ValueType::Decimal);
        let persistence_converted = inputs[6].value.try_convert_to(ValueType::Decimal);

        // gather errors
        if seed_converted.is_err() { input_errors.push((0, seed_converted.as_ref().err().unwrap().message.clone())); }
        if width_converted.is_err() { input_errors.push((1, width_converted.as_ref().err().unwrap().message.clone())); }
        if height_converted.is_err() { input_errors.push((2, height_converted.as_ref().err().unwrap().message.clone())); }
        if octaves_converted.is_err() { input_errors.push((3, octaves_converted.as_ref().err().unwrap().message.clone())); }
        if frequency_converted.is_err() { input_errors.push((4, frequency_converted.as_ref().err().unwrap().message.clone())); }
        if lacunarity_converted.is_err() { input_errors.push((5, lacunarity_converted.as_ref().err().unwrap().message.clone())); }
        if persistence_converted.is_err() { input_errors.push((6, persistence_converted.as_ref().err().unwrap().message.clone())); }

        // return if error
        if input_errors.len() > 0 { return Err(OperationError { input_errors, node_error: None }); }

        // get values
        let Ok(Value::Integer(mut seed)) = seed_converted else { return Err(OperationError { input_errors, node_error: Some("Error converting.".to_string()) }); };
        let Ok(Value::Integer(mut width)) = width_converted else { return Err(OperationError { input_errors, node_error: Some("Error converting.".to_string()) }); };
        let Ok(Value::Integer(mut height)) = height_converted else { return Err(OperationError { input_errors, node_error: Some("Error converting.".to_string()) }); };
        let Ok(Value::Integer(octaves)) = octaves_converted else { return Err(OperationError { input_errors, node_error: Some("Error converting.".to_string()) }); };
        let Ok(Value::Decimal(frequency)) = frequency_converted else { return Err(OperationError { input_errors, node_error: Some("Error converting.".to_string()) }); };
        let Ok(Value::Decimal(lacunarity)) = lacunarity_converted else { return Err(OperationError { input_errors, node_error: Some("Error converting.".to_string()) }); };
        let Ok(Value::Decimal(persistence)) = persistence_converted else { return Err(OperationError { input_errors, node_error: Some("Error converting.".to_string()) }); };

        // run node
        width = width.max(1);
        height = height.max(1);
        seed = seed.max(1);

        let mut image_buffer = ImageBuffer::new(width as u32, height as u32);

        let basicmulti = HybridMulti::<Perlin>::new(seed as u32).set_frequency(frequency as f64).set_octaves(octaves as usize).set_lacunarity(lacunarity as f64).set_persistence(persistence as f64);

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


#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum NoiseWorleyDistanceFunction {
    Chebyshev,
    Euclidean,
    EuclideanSquared,
    Manhattan,
    Quadratic,
}

impl NoiseWorleyDistanceFunction {
    pub fn types() -> [NoiseWorleyDistanceFunction; 5] {
        let types: [NoiseWorleyDistanceFunction; 5] = [
            NoiseWorleyDistanceFunction::Chebyshev,
            NoiseWorleyDistanceFunction::Euclidean,
            NoiseWorleyDistanceFunction::EuclideanSquared,
            NoiseWorleyDistanceFunction::Manhattan,
            NoiseWorleyDistanceFunction::Quadratic,
        ];

        types
    }
}