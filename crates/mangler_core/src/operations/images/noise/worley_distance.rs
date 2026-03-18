//! Worley (cellular) noise distance image generator.
//!
//! Produces a grayscale image based on the distance to the nearest cell point
//! in a Worley noise field. Supports multiple distance functions: Chebyshev,
//! Euclidean, Euclidean squared, Manhattan, and Quadratic.

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

/// Operation that generates a Worley noise image using distance return type.
///
/// The output brightness represents the distance from each pixel to the nearest
/// Worley cell point, producing a cellular/Voronoi-like pattern.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpImageNoiseWorleyDistance {}

impl OpImageNoiseWorleyDistance {
    /// Returns the node metadata (name and description) for this operation.
    pub fn settings() -> NodeSettings {
        NodeSettings {
            name: "worley noise distance".to_string(),
            description: "Creates a worley noise distance image.".to_string(),
        }
    }

    /// Creates the default inputs: seed, width, height, distance function, and frequency.
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

    /// Creates the default output: a single grayscale image.
    pub fn create_outputs() -> Vec<Output> {
        vec![
            Output::new("output".to_string(), Value::DynamicImage { data:default_image(), change_id:get_id() }, None),
        ]
    }

    /// Generates a Worley distance noise image from the given inputs.
    pub async fn run(inputs: &mut [Input]) -> Result<OperationResponse, OperationError> {
        let start_time = Instant::now();
        let mut input_errors: Vec<(usize, String)> = vec![];

        // convert inputs
        let seed_converted = convert_input(inputs, 0, ValueType::Integer, &mut input_errors);
        let width_converted = convert_input(inputs, 1, ValueType::Integer, &mut input_errors);
        let height_converted = convert_input(inputs, 2, ValueType::Integer, &mut input_errors);
        let distance_function_converted = convert_input(inputs, 3, ValueType::NoiseWorleyDistanceFunction, &mut input_errors);
        let frequency_converted = convert_input(inputs, 4, ValueType::Decimal, &mut input_errors);


        // return if error
        if !input_errors.is_empty() { return Err(OperationError { input_errors, node_error: None }); }

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

        // Map the enum variant to the corresponding noise library distance function
        let df = match distance_function {
            NoiseWorleyDistanceFunction::Chebyshev => distance_functions::chebyshev,
            NoiseWorleyDistanceFunction::Euclidean => distance_functions::euclidean,
            NoiseWorleyDistanceFunction::EuclideanSquared => distance_functions::euclidean_squared,
            NoiseWorleyDistanceFunction::Manhattan => distance_functions::manhattan,
            NoiseWorleyDistanceFunction::Quadratic => quadratic_distance,
        };

        let worley = Worley::new(seed as u32).set_return_type(noise::core::worley::ReturnType::Distance).set_distance_function(df).set_frequency(frequency as f64);

        for x in 0..width {
            for y in 0..height {
                let size = width.max(height) as f64;
                let coords_x = (x as f64) / size;
                let coords_y = (y as f64) / size;
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


/// Available distance functions for Worley noise generation.
///
/// Each function measures the distance between two points differently,
/// producing distinct cell patterns in the resulting noise image.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum NoiseWorleyDistanceFunction {
    /// Maximum of absolute differences along each axis (L-infinity norm).
    Chebyshev,
    /// Standard straight-line distance (L2 norm).
    Euclidean,
    /// Squared Euclidean distance (avoids the square root for performance).
    EuclideanSquared,
    /// Sum of absolute differences along each axis (L1 norm / taxicab distance).
    Manhattan,
    /// Custom distance combining sum, absolute sum, and squared sum of differences.
    Quadratic,
}

/// Computes a quadratic distance metric between two points.
///
/// Combines the raw sum, absolute sum, and squared sum of per-axis differences
/// to produce a non-standard distance that creates unique cell shapes.
pub fn quadratic_distance(p1: &[f64], p2: &[f64]) -> f64 {
    let (sum, abs_sum, sq_sum) = p1.iter()
        .zip(p2.iter())
        .map(|(a, b)| a - b)
        .fold((0.0, 0.0, 0.0), |(sum, abs_sum, sq_sum), d| {
            (sum + d, abs_sum + d.abs(), sq_sum + d * d)
        });
    abs_sum + sq_sum + sum
}

impl NoiseWorleyDistanceFunction {
    /// Returns an array of all available distance function variants.
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
    async fn test_opimagenoiseworleydistance_settings() {
        let s = OpImageNoiseWorleyDistance::settings();
        assert_eq!(s.name, "worley noise distance");
        assert_eq!(OpImageNoiseWorleyDistance::create_inputs().len(), 5);
        assert_eq!(OpImageNoiseWorleyDistance::create_outputs().len(), 1);
    }


    #[tokio::test]
    async fn test_opimagenoiseworleydistance_run() {
        let mut inputs = vec![
            Input::new("seed".to_string(), Value::Integer(1), None, None),
            Input::new("width".to_string(), Value::Integer(16), None, None),
            Input::new("height".to_string(), Value::Integer(16), None, None),
            Input::new("distance_function".to_string(), Value::NoiseWorleyDistanceFunction(NoiseWorleyDistanceFunction::EuclideanSquared), None, None),
            Input::new("frequency".to_string(), Value::Decimal(5.0), None, None),
        ];
        let result = OpImageNoiseWorleyDistance::run(&mut inputs).await;
        assert!(result.is_ok(), "run failed: {:?}", result.err());
        match &result.unwrap().responses[0].value {
            Value::DynamicImage { .. } => {}
            other => panic!("Expected DynamicImage, got {:?}", other),
        }
    }

    #[tokio::test]
    async fn test_opimagenoiseworleydistance_all_distance_functions() {
        let functions = [
            NoiseWorleyDistanceFunction::Chebyshev,
            NoiseWorleyDistanceFunction::Euclidean,
            NoiseWorleyDistanceFunction::EuclideanSquared,
            NoiseWorleyDistanceFunction::Manhattan,
            NoiseWorleyDistanceFunction::Quadratic,
        ];
        for df in &functions {
            let mut inputs = vec![
                Input::new("seed".to_string(), Value::Integer(1), None, None),
                Input::new("width".to_string(), Value::Integer(8), None, None),
                Input::new("height".to_string(), Value::Integer(8), None, None),
                Input::new("distance_function".to_string(), Value::NoiseWorleyDistanceFunction(df.clone()), None, None),
                Input::new("frequency".to_string(), Value::Decimal(5.0), None, None),
            ];
            let result = OpImageNoiseWorleyDistance::run(&mut inputs).await;
            assert!(result.is_ok(), "worley distance with {:?} failed: {:?}", df, result.err());
        }
    }

    #[tokio::test]
    async fn test_opimagenoiseworleydistance_correct_dimensions() {
        let mut inputs = vec![
            Input::new("seed".to_string(), Value::Integer(1), None, None),
            Input::new("width".to_string(), Value::Integer(16), None, None),
            Input::new("height".to_string(), Value::Integer(8), None, None),
            Input::new("distance_function".to_string(), Value::NoiseWorleyDistanceFunction(NoiseWorleyDistanceFunction::Euclidean), None, None),
            Input::new("frequency".to_string(), Value::Decimal(5.0), None, None),
        ];
        let result = OpImageNoiseWorleyDistance::run(&mut inputs).await.unwrap();
        match &result.responses[0].value {
            Value::DynamicImage { data, .. } => {
                assert_eq!(data.width(), 16);
                assert_eq!(data.height(), 8);
            }
            other => panic!("Expected DynamicImage, got {:?}", other),
        }
    }

}
