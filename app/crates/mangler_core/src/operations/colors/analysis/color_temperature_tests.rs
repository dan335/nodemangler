use super::*;
use crate::color::Color;
use crate::input::Input;
use crate::value::Value;

fn single_input(color: Color) -> Vec<Input> {
    vec![Input::new("color".to_string(), Value::Color(color), None, None)]
}

#[tokio::test]
async fn test_white_neutral_temperature() {
    // Pure white should be close to the D65 standard illuminant (~6504K).
    let white = Color::from_srgb_float(1.0, 1.0, 1.0, 1.0);
    let mut inputs = single_input(white);
    let result = OpColorAnalysisColorTemperature::run(&mut inputs).await.unwrap();

    let Value::Decimal(kelvin) = result.responses[0].value else { panic!("Expected Decimal") };
    // Allow a generous tolerance since we use an approximation formula.
    assert!(
        (kelvin - 6504.0).abs() < 500.0,
        "White should be near D65 (~6504K), got {}K",
        kelvin
    );

    let Value::Decimal(warm_cool) = result.responses[1].value else { panic!("Expected Decimal") };
    // D65 should be roughly in the middle (not fully warm, not fully cool).
    assert!(warm_cool > 0.2 && warm_cool < 0.8, "White warm_cool should be mid-range, got {}", warm_cool);
}

#[tokio::test]
async fn test_pure_red_is_warm() {
    // Pure red has a high warm component and should yield a low (warm-side) Kelvin estimate.
    let red = Color::from_srgb_float(1.0, 0.0, 0.0, 1.0);
    let mut inputs = single_input(red);
    let result = OpColorAnalysisColorTemperature::run(&mut inputs).await.unwrap();

    let Value::Decimal(warm_cool) = result.responses[1].value else { panic!("Expected Decimal") };
    // Red is a warm color and should have a high warm_cool value.
    assert!(warm_cool > 0.5, "Pure red should be warm (warm_cool > 0.5), got {}", warm_cool);
}

#[tokio::test]
async fn test_kelvin_clamped_range() {
    // Output Kelvin values must always be within the clamped 1000–20000 range.
    for color in [
        Color::from_srgb_float(0.0, 0.0, 0.0, 1.0),
        Color::from_srgb_float(1.0, 1.0, 1.0, 1.0),
        Color::from_srgb_float(0.0, 0.0, 1.0, 1.0),
        Color::from_srgb_float(1.0, 0.5, 0.0, 1.0),
    ] {
        let mut inputs = single_input(color);
        let result = OpColorAnalysisColorTemperature::run(&mut inputs).await.unwrap();
        let Value::Decimal(k) = result.responses[0].value else { panic!("Expected Decimal") };
        assert!((1000.0..=20000.0).contains(&k), "Kelvin out of range: {}", k);
    }
}

#[tokio::test]
async fn test_settings() {
    let s = OpColorAnalysisColorTemperature::settings();
    assert_eq!(s.name, "color temperature");
    assert_eq!(OpColorAnalysisColorTemperature::create_inputs().len(), 1);
    assert_eq!(OpColorAnalysisColorTemperature::create_outputs().len(), 2);
}
