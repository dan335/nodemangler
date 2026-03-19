use super::*;
use crate::color::Color;
use crate::input::Input;
use crate::value::Value;

fn luminance_inputs(color: Color) -> Vec<Input> {
    vec![
        Input::new("color".to_string(), Value::Color(color), None, None),
    ]
}

#[tokio::test]
async fn test_black_luminance_is_zero() {
    // Pure black has zero luminance.
    let black = Color::from_srgb_float(0.0, 0.0, 0.0, 1.0);
    let mut inputs = luminance_inputs(black);
    let result = OpColorAnalysisLuminance::run(&mut inputs).await.unwrap();

    let Value::Decimal(luminance) = result.responses[0].value else { panic!("Expected Decimal") };
    assert_eq!(luminance, 0.0, "black luminance should be 0.0, got {}", luminance);
}

#[tokio::test]
async fn test_white_luminance_is_one() {
    // Pure white has relative luminance of 1.0.
    let white = Color::from_srgb_float(1.0, 1.0, 1.0, 1.0);
    let mut inputs = luminance_inputs(white);
    let result = OpColorAnalysisLuminance::run(&mut inputs).await.unwrap();

    let Value::Decimal(luminance) = result.responses[0].value else { panic!("Expected Decimal") };
    assert!(
        (luminance - 1.0).abs() < 0.001,
        "white luminance should be ~1.0, got {}",
        luminance
    );
}

#[tokio::test]
async fn test_settings() {
    let s = OpColorAnalysisLuminance::settings();
    assert_eq!(s.name, "luminance");
    assert_eq!(OpColorAnalysisLuminance::create_inputs().len(), 1);
    assert_eq!(OpColorAnalysisLuminance::create_outputs().len(), 1);
}
