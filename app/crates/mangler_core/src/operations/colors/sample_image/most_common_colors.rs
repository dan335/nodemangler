//! Most common colors sampling operation.
//!
//! Analyzes an image to find the most frequently occurring colors by
//! quantizing each pixel's HSL representation and counting occurrences.
//! Returns the top 5 most common colors.

use crate::color::Color;
use crate::input::{Input, InputSettings};
use crate::node_settings::NodeSettings;
use crate::operations::{OperationResponse, OperationError, OutputResponse, convert_input};
use crate::output::Output;
use crate::value::{Value, ValueType};
use serde::{Deserialize, Serialize};
use std::time::Instant;
use std::collections::HashMap;

/// Operation that extracts the top 5 most common colors from an image.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpColorSampleMostCommonColors {}

impl OpColorSampleMostCommonColors {
    /// Returns the node metadata (name and description) for this operation.
    pub fn settings() -> NodeSettings {
        NodeSettings {
            name: "most common colors".to_string(),
            description: "Finds the most common colors in an image.".to_string(),
            help: "Converts every pixel to HSL, quantizes hue/saturation/lightness into integer buckets using the provided precision values, and counts how many pixels fall into each bucket. The five most populous buckets are converted back to a representative color (bucket center) and returned.\n\nHigher quantization values give finer color distinctions but can scatter visually identical pixels across multiple buckets, so start low and increase gradually. Single-channel images are broadcast to gray. If fewer than five distinct buckets exist the remaining slots are padded with the default color, and pixel alpha is ignored during counting.".to_string(),
        }
    }

    /// Creates the input definitions: an image and quantization precision for hue, saturation, and lightness.
    pub fn create_inputs() -> Vec<Input> {
        vec![
            Input::new("image".to_string(), Value::Image{data:crate::operations::default_image(), change_id:crate::get_id()}, None, None)
                .with_description("Image whose pixels are scanned to find the most common quantized HSL colors."),
            Input::new("hue quantization".to_string(), Value::Decimal(10.0), Some(InputSettings::Slider { range: (1.0, 100.0), step_by: Some(1.0), clamp_to_range: true}), None)
                .with_description("Number of hue buckets; higher values distinguish more hues."),
            Input::new("saturation quantization".to_string(), Value::Decimal(10.0), Some(InputSettings::Slider { range: (1.0, 100.0), step_by: Some(1.0), clamp_to_range: true}), None)
                .with_description("Number of saturation buckets; higher values distinguish more saturations."),
            Input::new("lightness quantization".to_string(), Value::Decimal(10.0), Some(InputSettings::Slider { range: (1.0, 100.0), step_by: Some(1.0), clamp_to_range: true}), None)
                .with_description("Number of lightness buckets; higher values distinguish more shades."),
        ]
    }

    /// Creates 5 color output slots, one for each of the top most common colors.
    pub fn create_outputs() -> Vec<Output> {
        vec![
            Output::new("1".to_string(), Value::Color(Color::default()), None)
                .with_description("Most common color in the image (from the largest HSL bucket)."),
            Output::new("2".to_string(), Value::Color(Color::default()), None)
                .with_description("Second most common color in the image."),
            Output::new("3".to_string(), Value::Color(Color::default()), None)
                .with_description("Third most common color in the image."),
            Output::new("4".to_string(), Value::Color(Color::default()), None)
                .with_description("Fourth most common color in the image."),
            Output::new("5".to_string(), Value::Color(Color::default()), None)
                .with_description("Fifth most common color in the image."),
        ]
    }

    /// Executes the operation, scanning all pixels and returning the 5 most common quantized colors.
    pub async fn run(inputs: &mut [Input]) -> Result<OperationResponse, OperationError> {
        let start_time = Instant::now();
        let mut input_errors: Vec<(usize, String)> = vec![];

        // convert inputs
        let image_converted = convert_input(inputs, 0, ValueType::Image, &mut input_errors);
        let hue_precision_converted = convert_input(inputs, 1, ValueType::Decimal, &mut input_errors);
        let saturation_precision_converted = convert_input(inputs, 2, ValueType::Decimal, &mut input_errors);
        let lightness_precision_converted = convert_input(inputs, 3, ValueType::Decimal, &mut input_errors);


        // return if error
        if !input_errors.is_empty() { return Err(OperationError { input_errors, node_error: None }); }

        // get values
        let Value::Image{data:image, change_id:_} = image_converted.unwrap() else { unreachable!() };
        let Value::Decimal(hue_precision) = hue_precision_converted.unwrap() else { unreachable!() };
        let Value::Decimal(saturation_precision) = saturation_precision_converted.unwrap() else { unreachable!() };
        let Value::Decimal(lightness_precision) = lightness_precision_converted.unwrap() else { unreachable!() };

        // Quantize each pixel's HSL values into buckets and count occurrences.
        // Higher precision values produce more buckets (finer color distinction).
        //
        // The common case uses a flat Vec<u32> histogram indexed by
        // (h * s_dim + s) * l_dim + l, which avoids per-pixel hashing. Bucket
        // indices that fall outside the flat range (out-of-gamut pixels can
        // produce HSL components outside [0,1]) spill into a HashMap so the
        // counts stay identical to the hashing implementation.
        //
        // In-range hue is [0, 360] and saturation/lightness are [0, 1], so
        // each in-range index is at most precision.round(); +2 leaves margin.
        let dims: Option<[usize; 3]> = {
            let dim = |p: f32| -> Option<usize> {
                if p.is_finite() && (0.0..=100_000.0).contains(&p) { Some(p.ceil() as usize + 2) } else { None }
            };
            match (dim(hue_precision), dim(saturation_precision), dim(lightness_precision)) {
                (Some(hd), Some(sd), Some(ld)) if hd.saturating_mul(sd).saturating_mul(ld) <= (1 << 24) => Some([hd, sd, ld]),
                _ => None,
            }
        };
        let mut flat: Vec<u32> = dims.map(|[hd, sd, ld]| vec![0u32; hd * sd * ld]).unwrap_or_default();
        let mut color_counts: HashMap<[i32; 3], u32> = HashMap::new();

        let ch = image.channels() as usize;
        for pixel in image.pixels() {
            // Extract RGB from any channel count
            let (r, g, b) = if ch >= 3 {
                (pixel[0], pixel[1], pixel[2])
            } else {
                (pixel[0], pixel[0], pixel[0])
            };
            let color = Color::from_srgb_float(r, g, b, 1.0);
            let hsl = color.to_hsl();
            // Round each channel to its quantized bucket index
            let h = ((hsl.0 / 360.0) * hue_precision).round() as i32;
            let s = (hsl.1 * saturation_precision).round() as i32;
            let l = (hsl.2 * lightness_precision).round() as i32;
            match dims {
                Some([hd, sd, ld])
                    if (0..hd as i32).contains(&h)
                        && (0..sd as i32).contains(&s)
                        && (0..ld as i32).contains(&l) =>
                {
                    flat[(h as usize * sd + s as usize) * ld + l as usize] += 1;
                }
                _ => *color_counts.entry([h, s, l]).or_insert(0) += 1,
            }
        }

        // Gather non-empty buckets from the flat histogram plus any spilled entries.
        let mut sorted_colors: Vec<([i32; 3], u32)> = color_counts.iter().map(|(k, &c)| (*k, c)).collect();
        if let Some([_, sd, ld]) = dims {
            sorted_colors.extend(flat.iter().enumerate().filter(|(_, &c)| c > 0).map(|(i, &c)| {
                let l = i % ld;
                let s = (i / ld) % sd;
                let h = i / (ld * sd);
                ([h as i32, s as i32, l as i32], c)
            }));
        }

        // Sort buckets by pixel count (most frequent first); ties break on the
        // bucket key so the ordering is deterministic.
        sorted_colors.sort_by(|a, b| b.1.cmp(&a.1).then_with(|| a.0.cmp(&b.0)));

        let mut responses: Vec<OutputResponse> = Vec::new();

        // Convert the top 5 quantized HSL buckets back to colors
        for (hsl, _count) in sorted_colors.iter().take(5) {
            let h = ((hsl[0] as f32) / hue_precision) * 360.0;
            let s = (hsl[1] as f32) / saturation_precision;
            let l = (hsl[2] as f32) / lightness_precision;
            responses.push(OutputResponse {
                value: Value::Color(Color::from_hsl(h, s, l, 1.0)),
            });
        }

        // Pad with default colors if fewer than 5 distinct buckets exist
        while responses.len() < 5 {
            responses.push(OutputResponse {
                value: Value::Color(Color::default()),
            });
        }

        Ok(OperationResponse { 
            time: Instant::now().duration_since(start_time),
            responses,
        })
    }
}

#[cfg(test)]
#[path = "most_common_colors_tests.rs"]
mod tests;
