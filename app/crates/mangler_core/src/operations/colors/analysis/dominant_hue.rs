//! Dominant hue identification operation.
//!
//! Given five color inputs, identifies which has the most "dominant" hue by computing
//! the product of HSV saturation × HSV value as a dominance weight. Returns the
//! dominant color and its 1-based index.

use crate::color::Color;
use crate::input::Input;
use crate::node_settings::NodeSettings;
use crate::operations::{OperationResponse, OperationError, OutputResponse, convert_input};
use crate::output::Output;
use crate::value::{Value, ValueType};
use serde::{Deserialize, Serialize};
use std::time::Instant;

/// Operation that identifies the most hue-dominant color from five inputs.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpColorAnalysisDominantHue {}

impl OpColorAnalysisDominantHue {
    /// Returns the node metadata (name and description) for this operation.
    pub fn settings() -> NodeSettings {
        NodeSettings {
            name: "dominant hue".to_string(),
            description: "Identifies which of the five input colors has the most dominant hue (highest HSV saturation × value). Returns the dominant color and its 1-based index.".to_string(),
            help: "For each input, converts to HSV and computes the weight s * v. A fully saturated, fully bright color scores 1.0; grays, blacks, and whites tend toward 0.0.\n\nReturns the color with the highest weight and its 1-based position (1-5). Ties are resolved in favor of the lowest index, so equal inputs produce a deterministic pick. The actual hue angle is not used as a tiebreaker, only the saturation/value product, which means two different vivid hues at the same s*v pick the earlier slot.".to_string(),
        }
    }

    /// Creates the five input definitions: color_1 through color_5.
    pub fn create_inputs() -> Vec<Input> {
        vec![
            Input::new("color_1".to_string(), Value::Color(Color::default()), None, None)
                .with_description("First candidate color to evaluate for hue dominance."),
            Input::new("color_2".to_string(), Value::Color(Color::default()), None, None)
                .with_description("Second candidate color to evaluate for hue dominance."),
            Input::new("color_3".to_string(), Value::Color(Color::default()), None, None)
                .with_description("Third candidate color to evaluate for hue dominance."),
            Input::new("color_4".to_string(), Value::Color(Color::default()), None, None)
                .with_description("Fourth candidate color to evaluate for hue dominance."),
            Input::new("color_5".to_string(), Value::Color(Color::default()), None, None)
                .with_description("Fifth candidate color to evaluate for hue dominance."),
        ]
    }

    /// Creates the two output definitions: dominant_color and dominant_index (1-based).
    pub fn create_outputs() -> Vec<Output> {
        vec![
            Output::new("dominant_color".to_string(), Value::Color(Color::default()), None)
                .with_description("The input color with the highest HSV saturation × value weight."),
            Output::new("dominant_index".to_string(), Value::Integer(1), None)
                .with_description("One-based index (1–5) identifying which input was dominant."),
        ]
    }

    /// Executes the dominant hue identification.
    ///
    /// Computes HSV saturation × HSV value for each color and returns the
    /// color and 1-based index of the maximum. Ties are broken by lowest index.
    pub async fn run(inputs: &mut [Input]) -> Result<OperationResponse, OperationError> {
        let start_time = Instant::now();
        let mut input_errors: Vec<(usize, String)> = vec![];

        // Convert all five color inputs.
        let c1 = convert_input(inputs, 0, ValueType::Color, &mut input_errors);
        let c2 = convert_input(inputs, 1, ValueType::Color, &mut input_errors);
        let c3 = convert_input(inputs, 2, ValueType::Color, &mut input_errors);
        let c4 = convert_input(inputs, 3, ValueType::Color, &mut input_errors);
        let c5 = convert_input(inputs, 4, ValueType::Color, &mut input_errors);

        // Return early on conversion errors.
        if !input_errors.is_empty() { return Err(OperationError { input_errors, node_error: None }); }

        // Unwrap all five colors.
        let Value::Color(color1) = c1.unwrap() else { unreachable!() };
        let Value::Color(color2) = c2.unwrap() else { unreachable!() };
        let Value::Color(color3) = c3.unwrap() else { unreachable!() };
        let Value::Color(color4) = c4.unwrap() else { unreachable!() };
        let Value::Color(color5) = c5.unwrap() else { unreachable!() };

        let colors = [color1, color2, color3, color4, color5];

        // Compute dominance weight for each color: saturation × value in HSV space.
        // A fully saturated, fully bright color has weight 1.0; gray/black/white approach 0.0.
        let weights: Vec<f32> = colors.iter().map(|c| {
            let (_h, s, v, _a) = c.to_hsv();
            s * v
        }).collect();

        // Find the index of the maximum weight; ties are broken by lowest index (natural iteration order).
        let (dominant_index, _) = weights.iter()
            .enumerate()
            .fold((0, -1.0_f32), |(best_i, best_w), (i, &w)| {
                if w > best_w { (i, w) } else { (best_i, best_w) }
            });

        let dominant_color = colors[dominant_index];
        // Convert to 1-based index for output (Value::Integer stores i32).
        let dominant_index_1based = (dominant_index + 1) as i32;

        Ok(OperationResponse { 
            time: Instant::now().duration_since(start_time),
            responses: vec![
                OutputResponse { value: Value::Color(dominant_color) },
                OutputResponse { value: Value::Integer(dominant_index_1based) },
            ],
        })
    }
}

#[cfg(test)]
#[path = "dominant_hue_tests.rs"]
mod tests;
