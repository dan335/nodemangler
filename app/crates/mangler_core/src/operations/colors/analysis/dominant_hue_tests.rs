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
