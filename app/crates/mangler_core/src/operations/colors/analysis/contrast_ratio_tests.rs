use super::*;
use crate::color::Color;
use crate::input::Input;
use crate::value::Value;

fn contrast_inputs(a: Color, b: Color) -> Vec<Input> {
    vec![
        Input::new("a".to_string(), Value::Color(a), None, None),
        Input::new("b".to_string(), Value::Color(b), None, None),
    ]
}

#[tokio::test]
async fn test_black_white_contrast() {
    // Black on white is the maximum possible contrast, approximately 21:1.
    let black = Color::from_srgb_float(0.0, 0.0, 0.0, 1.0);
    let white = Color::from_srgb_float(1.0, 1.0, 1.0, 1.0);
    let mut inputs = contrast_inputs(black, white);
    let result = OpColorAnalysisContrastRatio::run(&mut inputs).await.unwrap();

    let Value::Decimal(ratio) = result.responses[0].value else { panic!("Expected Decimal") };
    let Value::Bool(passes_aa) = result.responses[1].value else { panic!("Expected Bool") };
    let Value::Bool(passes_aaa) = result.responses[2].value else { panic!("Expected Bool") };

    assert!(
        (ratio - 21.0).abs() < 0.1,
        "black-white contrast ratio should be ~21.0, got {}",
        ratio
    );
    assert!(passes_aa, "black-white should pass AA");
    assert!(passes_aaa, "black-white should pass AAA");
}

#[tokio::test]
async fn test_same_color_contrast() {
    // Comparing a color with itself gives a ratio of exactly 1.0, failing all thresholds.
    let color = Color::from_srgb_float(0.4, 0.6, 0.2, 1.0);
    let mut inputs = contrast_inputs(color, color);
    let result = OpColorAnalysisContrastRatio::run(&mut inputs).await.unwrap();

    let Value::Decimal(ratio) = result.responses[0].value else { panic!("Expected Decimal") };
    let Value::Bool(passes_aa) = result.responses[1].value else { panic!("Expected Bool") };
    let Value::Bool(passes_aaa) = result.responses[2].value else { panic!("Expected Bool") };

    assert!(
        (ratio - 1.0).abs() < 0.001,
        "same color contrast ratio should be 1.0, got {}",
        ratio
    );
    assert!(!passes_aa, "same color should not pass AA");
    assert!(!passes_aaa, "same color should not pass AAA");
}

#[tokio::test]
async fn test_settings() {
    let s = OpColorAnalysisContrastRatio::settings();
    assert_eq!(s.name, "contrast ratio");
    assert_eq!(OpColorAnalysisContrastRatio::create_inputs().len(), 2);
    assert_eq!(OpColorAnalysisContrastRatio::create_outputs().len(), 3);
}
