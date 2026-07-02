use super::*;
use crate::color::Color;
use crate::input::Input;
use crate::value::Value;

fn mix_inputs(source: Color, target: Color, mixed: Color) -> Vec<Input> {
    vec![
        Input::new("source".to_string(), Value::Color(source), None, None),
        Input::new("target".to_string(), Value::Color(target), None, None),
        Input::new("mixed".to_string(), Value::Color(mixed), None, None),
    ]
}

#[tokio::test]
async fn test_mixed_equals_source_returns_zero() {
    // If mixed == source, then t should be 0.0.
    let source = Color::from_srgb_float(0.2, 0.4, 0.6, 1.0);
    let target = Color::from_srgb_float(0.8, 0.6, 0.4, 1.0);
    let mut inputs = mix_inputs(source, target, source);
    let result = OpColorAnalysisMixRatio::run(&mut inputs).await.unwrap();

    let Value::Decimal(ratio) = result.responses[0].value else { panic!("Expected Decimal") };
    assert!((ratio - 0.0).abs() < 0.001, "mixed=source should give ratio 0.0, got {}", ratio);
}

#[tokio::test]
async fn test_mixed_equals_target_returns_one() {
    // If mixed == target, then t should be 1.0.
    let source = Color::from_srgb_float(0.2, 0.4, 0.6, 1.0);
    let target = Color::from_srgb_float(0.8, 0.6, 0.4, 1.0);
    let mut inputs = mix_inputs(source, target, target);
    let result = OpColorAnalysisMixRatio::run(&mut inputs).await.unwrap();

    let Value::Decimal(ratio) = result.responses[0].value else { panic!("Expected Decimal") };
    assert!((ratio - 1.0).abs() < 0.001, "mixed=target should give ratio 1.0, got {}", ratio);
}

#[tokio::test]
async fn test_midpoint_returns_half() {
    // If mixed is the exact midpoint, t should be 0.5.
    let source = Color::from_srgb_float(0.0, 0.0, 0.0, 1.0);
    let target = Color::from_srgb_float(1.0, 1.0, 1.0, 1.0);
    let midpoint = Color::from_srgb_float(0.5, 0.5, 0.5, 1.0);
    let mut inputs = mix_inputs(source, target, midpoint);
    let result = OpColorAnalysisMixRatio::run(&mut inputs).await.unwrap();

    let Value::Decimal(ratio) = result.responses[0].value else { panic!("Expected Decimal") };
    assert!((ratio - 0.5).abs() < 0.001, "midpoint should give ratio 0.5, got {}", ratio);
}

#[tokio::test]
async fn test_degenerate_all_channels_same() {
    // If source == target, all channels are degenerate; ratio defaults to 0.0.
    let same = Color::from_srgb_float(0.5, 0.5, 0.5, 1.0);
    let mut inputs = mix_inputs(same, same, same);
    let result = OpColorAnalysisMixRatio::run(&mut inputs).await.unwrap();

    let Value::Decimal(ratio) = result.responses[0].value else { panic!("Expected Decimal") };
    assert!((ratio - 0.0).abs() < 0.001, "Degenerate source==target should give ratio 0.0, got {}", ratio);
}

#[tokio::test]
async fn test_ratio_clamped_to_0_1() {
    // Even if mixed is outside the source–target range, ratio is clamped.
    let source = Color::from_srgb_float(0.3, 0.3, 0.3, 1.0);
    let target = Color::from_srgb_float(0.7, 0.7, 0.7, 1.0);
    // mixed far beyond target
    let beyond = Color::from_srgb_float(1.0, 1.0, 1.0, 1.0);
    let mut inputs = mix_inputs(source, target, beyond);
    let result = OpColorAnalysisMixRatio::run(&mut inputs).await.unwrap();

    let Value::Decimal(ratio) = result.responses[0].value else { panic!("Expected Decimal") };
    assert!((0.0..=1.0).contains(&ratio), "Ratio out of range: {}", ratio);
}

#[tokio::test]
async fn test_settings() {
    let s = OpColorAnalysisMixRatio::settings();
    assert_eq!(s.name, "mix ratio");
    assert_eq!(OpColorAnalysisMixRatio::create_inputs().len(), 3);
    assert_eq!(OpColorAnalysisMixRatio::create_outputs().len(), 1);
}
