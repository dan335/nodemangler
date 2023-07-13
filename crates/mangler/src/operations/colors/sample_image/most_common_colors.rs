use crate::color::Color;
use crate::input::{Input, InputSettings};
use crate::node_settings::NodeSettings;
use crate::operations::{OperationResponse, OperationError, OutputResponse};
use crate::output::Output;
use crate::value::{Value, ValueType};
use image::{DynamicImage, Pixel};
use serde::{Deserialize, Serialize};
use std::time::Instant;
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpColorSampleMostCommonColors {}

impl OpColorSampleMostCommonColors {
    pub fn settings() -> NodeSettings {
        NodeSettings {
            name: "most common colors".to_string(),
            description: "Finds the most common colors in an image.".to_string(),
        }
    }

    pub fn create_inputs() -> Vec<Input> {
        vec![
            Input::new("image".to_string(), Value::DynamicImage{data:crate::operations::default_image(), change_id:crate::get_id()}, None, None),
            Input::new("hue quantization".to_string(), Value::Decimal(10.0), Some(InputSettings::Slider { range: (1.0, 100.0), step_by: Some(1.0), clamp_to_range: true}), None),
            Input::new("saturation quantization".to_string(), Value::Decimal(10.0), Some(InputSettings::Slider { range: (1.0, 100.0), step_by: Some(1.0), clamp_to_range: true}), None),
            Input::new("lightness quantization".to_string(), Value::Decimal(10.0), Some(InputSettings::Slider { range: (1.0, 100.0), step_by: Some(1.0), clamp_to_range: true}), None),
        ]
    }

    pub fn create_outputs() -> Vec<Output> {
        vec![
            Output::new("1".to_string(), Value::Color(Color::default()), None),
            Output::new("2".to_string(), Value::Color(Color::default()), None),
            Output::new("3".to_string(), Value::Color(Color::default()), None),
            Output::new("4".to_string(), Value::Color(Color::default()), None),
            Output::new("5".to_string(), Value::Color(Color::default()), None),
        ]
    }

    pub async fn run(inputs: &Vec<Input>) -> Result<OperationResponse, OperationError> {
        let start_time = Instant::now();

        let Value::DynamicImage{data:image, change_id:_} = inputs[0].value.clone() else { return Err(OperationError { message: "Error getting image.".to_string() }); };
        let Ok(Value::Decimal(hue_precision)) = inputs[1].value.try_convert_to(ValueType::Decimal) else { return Err(OperationError { message: "Unable to convert to integer.".to_string() })};
        let Ok(Value::Decimal(saturation_precision)) = inputs[2].value.try_convert_to(ValueType::Decimal) else { return Err(OperationError { message: "Unable to convert to integer.".to_string() })};
        let Ok(Value::Decimal(lightness_precision)) = inputs[3].value.try_convert_to(ValueType::Decimal) else { return Err(OperationError { message: "Unable to convert to integer.".to_string() })};

        let mut color_counts: HashMap<[i32; 3], u32> = HashMap::new();

        for rgb in image::Rgb32FImage::pixels(&image.to_rgb32f()) {
            let color = Color::from_srgb_float(rgb[0], rgb[1], rgb[2], 1.0);
            let hsl = color.to_hsl();
            let h = ((hsl.0 / 360.0) * hue_precision).round() as i32;
            let s = (hsl.1 * saturation_precision).round() as i32;
            let l = (hsl.2 * lightness_precision).round() as i32;
            *color_counts.entry([h, s, l]).or_insert(0) += 1;
        }

        let mut sorted_colors: Vec<(&[i32; 3], &u32)> = color_counts.iter().collect();
        sorted_colors.sort_by(|a, b| b.1.cmp(a.1));

        let mut responses: Vec<OutputResponse> = Vec::new();

        for (hsl, _count) in sorted_colors.iter().take(5) {
            let h = ((hsl[0] as f32) / hue_precision) * 360.0;
            let s = (hsl[1] as f32) / saturation_precision;
            let l = (hsl[2] as f32) / lightness_precision;
            responses.push(OutputResponse {
                value: Value::Color(Color::from_hsl(h, s, l, 1.0)),
            });
        }

        Ok(OperationResponse {
            time: Instant::now().duration_since(start_time),
            responses,
        })
    }
}
