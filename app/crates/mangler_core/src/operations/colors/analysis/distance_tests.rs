use super::*;
use crate::color::Color;
use crate::input::Input;
use crate::value::Value;

fn distance_inputs(a: Color, b: Color) -> Vec<Input> {
    vec![
        Input::new("a".to_string(), Value::Color(a), None, None),
        Input::new("b".to_string(), Value::Color(b), None, None),
    ]
}

#[tokio::test]
async fn test_same_color_distance_is_zero() {
    // Comparing a color with itself should yield zero for both distance metrics.
    let color = Color::from_srgb_float(0.5, 0.3, 0.8, 1.0);
    let mut inputs = distance_inputs(color, color);
    let result = OpColorAnalysisDistance::run(&mut inputs).await.unwrap();

    let Value::Decimal(delta_e) = result.responses[0].value else { panic!("Expected Decimal") };
    let Value::Decimal(rgb_dist) = result.responses[1].value else { panic!("Expected Decimal") };

    assert!(delta_e.abs() < 0.001, "same color delta_e should be zero, got {}", delta_e);
    assert!(rgb_dist.abs() < 0.001, "same color rgb_distance should be zero, got {}", rgb_dist);
}

#[tokio::test]
async fn test_black_white_distance() {
    // Black and white are maximally different in both Lab and RGB spaces.
    let black = Color::from_srgb_float(0.0, 0.0, 0.0, 1.0);
    let white = Color::from_srgb_float(1.0, 1.0, 1.0, 1.0);
    let mut inputs = distance_inputs(black, white);
    let result = OpColorAnalysisDistance::run(&mut inputs).await.unwrap();

    let Value::Decimal(delta_e) = result.responses[0].value else { panic!("Expected Decimal") };
    let Value::Decimal(rgb_dist) = result.responses[1].value else { panic!("Expected Decimal") };

    // Delta E between black and white in Lab is ~100 (L goes from 0 to 100).
    assert!(delta_e > 50.0, "black-white delta_e should be large, got {}", delta_e);

    // RGB distance between (0,0,0) and (1,1,1) is sqrt(3) ≈ 1.732.
    assert!((rgb_dist - 3.0_f32.sqrt()).abs() < 0.01, "black-white rgb_distance should be ~1.732, got {}", rgb_dist);
}

#[tokio::test]
async fn test_settings() {
    let s = OpColorAnalysisDistance::settings();
    assert_eq!(s.name, "distance");
    assert_eq!(OpColorAnalysisDistance::create_inputs().len(), 2);
    assert_eq!(OpColorAnalysisDistance::create_outputs().len(), 2);
}
