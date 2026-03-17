use image::{ImageBuffer, DynamicImage};
use noise::core::worley::distance_functions;
use crate::get_id;
use crate::input::{Input, InputSettings};
use crate::node_settings::NodeSettings;
use crate::operations::{OperationResponse, OperationError, OutputResponse, default_image, convert_input};
use crate::output::Output;
use crate::value::{Value, ValueType};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use std::time::Instant;
use noise::{NoiseFn, Worley};

use super::worley_distance::NoiseWorleyDistanceFunction;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpImageNoiseWorleyValue {}

impl OpImageNoiseWorleyValue {
    pub fn settings() -> NodeSettings {
        NodeSettings {
            name: "worley noise value".to_string(),
            description: "Creates a worley noise value image.".to_string(),
        }
    }

    pub fn create_inputs() -> Vec<Input> {
        vec![
            Input::new("seed".to_string(), Value::Integer(1), Some(InputSettings::DragValue { clamp: None, speed: None }), None),
            Input::new("width".to_string(), Value::Integer(512), Some(InputSettings::DragValue {clamp:Some((1.0,10000.0)), speed: None }), None),
            Input::new("height".to_string(), Value::Integer(512), Some(InputSettings::DragValue {clamp:Some((1.0,10000.0)), speed: None }), None),
            //Input::new("scale".to_string(), Value::Decimal(10.0), Some(InputSettings::Slider { range: (0.01, 100.0), step_by: Some(0.1), clamp_to_range:false }), None),
            Input::new("distance_function".to_string(), Value::NoiseWorleyDistanceFunction(NoiseWorleyDistanceFunction::EuclideanSquared), None, None),
            Input::new("frequency".to_string(), Value::Decimal(5.0), Some(InputSettings::Slider { range: (1.0, 50.0), step_by: Some(0.1), clamp_to_range:false }), None),
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
        let distance_function_converted = convert_input(inputs, 3, ValueType::NoiseWorleyDistanceFunction, &mut input_errors);
        let frequency_converted = convert_input(inputs, 4, ValueType::Decimal, &mut input_errors);


        // return if error
        if input_errors.len() > 0 { return Err(OperationError { input_errors, node_error: None }); }

        // get values
        let Value::Integer(mut seed) = seed_converted.unwrap() else { unreachable!() };
        let Value::Integer(mut width) = width_converted.unwrap() else { unreachable!() };
        let Value::Integer(mut height) = height_converted.unwrap() else { unreachable!() };
        let Value::NoiseWorleyDistanceFunction(distance_function) = distance_function_converted.unwrap() else { unreachable!() };
        let Value::Decimal(frequency) = frequency_converted.unwrap() else { unreachable!() };

        // run node
        width = width.max(1);
        height = height.max(1);
        seed = seed.max(1);

        let mut image_buffer = ImageBuffer::new(width as u32, height as u32);

        let df = match distance_function {
            NoiseWorleyDistanceFunction::Chebyshev => distance_functions::chebyshev,
            NoiseWorleyDistanceFunction::Euclidean => distance_functions::euclidean,
            NoiseWorleyDistanceFunction::EuclideanSquared => distance_functions::euclidean_squared,
            NoiseWorleyDistanceFunction::Manhattan => distance_functions::manhattan,
            NoiseWorleyDistanceFunction::Quadratic => super::worley_distance::quadratic_distance,
        };

        let worley = Worley::new(seed as u32).set_return_type(noise::core::worley::ReturnType::Value).set_distance_function(df).set_frequency(frequency as f64);

        for x in 0..width {
            for y in 0..height {
                let size = width.max(height) as f64;
                let coords_x = (x as f64) / (size as f64);
                let coords_y = (y as f64) / (size as f64);
                let noise = worley.get([coords_x, coords_y]) as f32 * 0.5 + 0.5;
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