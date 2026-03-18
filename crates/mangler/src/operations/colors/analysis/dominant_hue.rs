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
        }
    }

    /// Creates the five input definitions: color_1 through color_5.
    pub fn create_inputs() -> Vec<Input> {
        vec![
            Input::new("color_1".to_string(), Value::Color(Color::default()), None, None),
            Input::new("color_2".to_string(), Value::Color(Color::default()), None, None),
            Input::new("color_3".to_string(), Value::Color(Color::default()), None, None),
            Input::new("color_4".to_string(), Value::Color(Color::default()), None, None),
            Input::new("color_5".to_string(), Value::Color(Color::default()), None, None),
        ]
    }

    /// Creates the two output definitions: dominant_color and dominant_index (1-based).
    pub fn create_outputs() -> Vec<Output> {
        vec![
            Output::new("dominant_color".to_string(), Value::Color(Color::default()), None),
            Output::new("dominant_index".to_string(), Value::Integer(1), None),
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
mod tests {
    use super::*;
    use crate::color::Color;
    use crate::input::Input;
    use crate::value::Value;

    fn five_inputs(colors: [Color; 5]) -> Vec<Input> {
        colors.iter().enumerate().map(|(i, c)| {
            Input::new(format!("color_{}", i + 1), Value::Color(*c), None, None)
        }).collect()
    }

    #[tokio::test]
    async fn test_all_same_color_returns_index_1() {
        // When all colors are identical, the first (index 1) should win due to tie-breaking.
        let same = Color::from_hsl(120.0, 0.5, 0.5, 1.0);
        let mut inputs = five_inputs([same; 5]);
        let result = OpColorAnalysisDominantHue::run(&mut inputs).await.unwrap();

        let Value::Integer(idx) = result.responses[1].value else { panic!("Expected Integer") };
        assert_eq!(idx, 1, "All-same colors should return index 1 (tie-break by lowest)");
    }

    #[tokio::test]
    async fn test_highly_saturated_wins() {
        // Place a highly saturated color at index 3; it should be selected as dominant.
        let dull = Color::from_hsl(0.0, 0.1, 0.5, 1.0);
        let vivid = Color::from_hsl(200.0, 1.0, 0.5, 1.0);
        let mut inputs = five_inputs([dull, dull, vivid, dull, dull]);
        let result = OpColorAnalysisDominantHue::run(&mut inputs).await.unwrap();

        let Value::Integer(idx) = result.responses[1].value else { panic!("Expected Integer") };
        assert_eq!(idx, 3, "Highly saturated color at position 3 should win, got index {}", idx);

        // The dominant color should equal the vivid color.
        let Value::Color(dom) = result.responses[0].value else { panic!("Expected Color") };
        assert!(
            (dom.r - vivid.r).abs() < 0.01 && (dom.g - vivid.g).abs() < 0.01 && (dom.b - vivid.b).abs() < 0.01,
            "Dominant color should be the vivid color"
        );
    }

    #[tokio::test]
    async fn test_last_position_wins() {
        // Dominant at position 5 should return index 5.
        let dull = Color::from_hsl(0.0, 0.05, 0.5, 1.0);
        let vivid = Color::from_hsl(30.0, 0.95, 0.8, 1.0);
        let mut inputs = five_inputs([dull, dull, dull, dull, vivid]);
        let result = OpColorAnalysisDominantHue::run(&mut inputs).await.unwrap();

        let Value::Integer(idx) = result.responses[1].value else { panic!("Expected Integer") };
        assert_eq!(idx, 5, "Dominant at position 5 should return index 5, got {}", idx);
    }

    #[tokio::test]
    async fn test_settings() {
        let s = OpColorAnalysisDominantHue::settings();
        assert_eq!(s.name, "dominant hue");
        assert_eq!(OpColorAnalysisDominantHue::create_inputs().len(), 5);
        assert_eq!(OpColorAnalysisDominantHue::create_outputs().len(), 2);
    }
}
