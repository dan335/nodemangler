use image::{RgbaImage, ImageBuffer, DynamicImage};
use crate::color::Color;
use crate::get_id;
use crate::input::{Input, InputSettings};
use crate::node_settings::NodeSettings;
use crate::operations::{OperationResponse, OperationError, OutputResponse, default_image};
use crate::output::Output;
use crate::value::{Value, ValueType};
use serde::{Deserialize, Serialize};
use std::time::Instant;
use noise::{NoiseFn, Seedable, BasicMulti, MultiFractal, Perlin, HybridMulti, RidgedMulti};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpImageNoiseRidgedMultifractalNoise {}

impl OpImageNoiseRidgedMultifractalNoise {
    pub fn settings() -> NodeSettings {
        NodeSettings {
            name: "ridged multifractal noise".to_string(),
            description: "Noise function that outputs ridged-multifractal noise.

            This noise function, heavily based on the fBm-noise function, generates ridged-multifractal noise. Ridged-multifractal noise is generated in much the same way as fBm noise, except the output of each octave is modified by an absolute-value function. Modifying the octave values in this way produces ridge-like formations.
            
            The values output from this function will usually range from -1.0 to 1.0 with default values for the parameters, but there are no guarantees that all output values will exist within this range. If the parameters are modified from their defaults, then the output will need to be scaled to remain in the [-1,1] range.
            
            Ridged-multifractal noise is often used to generate craggy mountainous terrain or marble-like textures.".to_string(),
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
            Input::new("attenuation".to_string(), Value::Decimal(2.0), Some(InputSettings::DragValue { clamp: None, speed: Some(0.01) }), None),
        ]
    }

    pub fn create_outputs() -> Vec<Output> {
        vec![
            Output::new("output".to_string(), Value::DynamicImage { data:default_image(), change_id:get_id() }, None),
        ]
    }

    pub async fn run(inputs: &Vec<Input>) -> Result<OperationResponse, OperationError> {
        let start_time = Instant::now();

        let Ok(Value::Integer(mut seed)) = inputs[0].value.try_convert_to(ValueType::Integer) else { return Err(OperationError { message: "Unable to convert to integer.".to_string() })};
        let Ok(Value::Integer(mut width)) = inputs[1].value.try_convert_to(ValueType::Integer) else { return Err(OperationError { message: "Unable to convert to integer.".to_string() })};
        let Ok(Value::Integer(mut height)) = inputs[2].value.try_convert_to(ValueType::Integer) else { return Err(OperationError { message: "Unable to convert to integer.".to_string() })};
        let Ok(Value::Integer(octaves)) = inputs[3].value.try_convert_to(ValueType::Integer) else { return Err(OperationError { message: "Unable to convert to integer.".to_string() })};
        let Ok(Value::Decimal(frequency)) = inputs[4].value.try_convert_to(ValueType::Decimal) else { return Err(OperationError { message: "Unable to convert to integer.".to_string() })};
        let Ok(Value::Decimal(lacunarity)) = inputs[5].value.try_convert_to(ValueType::Decimal) else { return Err(OperationError { message: "Unable to convert to integer.".to_string() })};
        let Ok(Value::Decimal(persistence)) = inputs[6].value.try_convert_to(ValueType::Decimal) else { return Err(OperationError { message: "Unable to convert to integer.".to_string() })};
        let Ok(Value::Decimal(attenuation)) = inputs[7].value.try_convert_to(ValueType::Decimal) else { return Err(OperationError { message: "Unable to convert to integer.".to_string() })};

        width = width.max(1);
        height = height.max(1);
        seed = seed.max(1);
        //scale = scale.max(0.0001);

        let mut image_buffer = ImageBuffer::new(width as u32, height as u32);

        let basicmulti = RidgedMulti::<Perlin>::new(seed as u32).set_frequency(frequency as f64).set_octaves(octaves as usize).set_lacunarity(lacunarity as f64).set_persistence(persistence as f64).set_attenuation(attenuation as f64);

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
                OutputResponse { value: Value::DynamicImage { data: dynamic_image, change_id: get_id() } },
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