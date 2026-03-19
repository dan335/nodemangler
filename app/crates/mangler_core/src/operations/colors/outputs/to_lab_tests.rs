use super::*;
use crate::color::Color;
use crate::input::Input;
use crate::value::Value;

fn color_input(r: f32, g: f32, b: f32, a: f32) -> Vec<Input> {
    vec![Input::new("input".to_string(), Value::Color(Color::from_srgb_float(r, g, b, a)), None, None)]
}

#[tokio::test]
async fn test_to_lab() {
    let mut inputs = color_input(0.5, 0.5, 0.5, 1.0);
    let result = OpColorOutputLab::run(&mut inputs).await.unwrap();
    assert_eq!(result.responses.len(), 4);
}

#[tokio::test]
async fn test_to_lab_settings() {
    let s = OpColorOutputLab::settings();
    assert_eq!(s.name, "to lab");
    assert_eq!(OpColorOutputLab::create_inputs().len(), 1);
    assert_eq!(OpColorOutputLab::create_outputs().len(), 4);
}

#[tokio::test]
async fn test_to_lab_black_lightness() {
    let mut inputs = color_input(0.0, 0.0, 0.0, 1.0);
    let result = OpColorOutputLab::run(&mut inputs).await.unwrap();
    assert_eq!(result.responses.len(), 4);
    // L* of black should be ~0
    match &result.responses[0].value {
        Value::Decimal(l) => assert!((*l).abs() < 0.5, "black L* should be ~0, got {}", l),
        other => panic!("Expected Decimal, got {:?}", other),
    }
}

#[tokio::test]
async fn test_to_lab_white_lightness() {
    let mut inputs = color_input(1.0, 1.0, 1.0, 1.0);
    let result = OpColorOutputLab::run(&mut inputs).await.unwrap();
    // L* of white should be ~100 (allow slight overshoot from float precision)
    match &result.responses[0].value {
        Value::Decimal(l) => assert!((*l - 100.0).abs() < 2.0, "white L* should be ~100, got {}", l),
        other => panic!("Expected Decimal, got {:?}", other),
    }
}

#[tokio::test]
async fn test_to_lab_alpha_passthrough() {
    let mut inputs = color_input(0.5, 0.5, 0.5, 0.4);
    let result = OpColorOutputLab::run(&mut inputs).await.unwrap();
    match &result.responses[3].value {
        Value::Decimal(a) => assert!((*a - 0.4).abs() < 0.01, "alpha should round trip, got {}", a),
        other => panic!("Expected Decimal for alpha, got {:?}", other),
    }
}

#[tokio::test]
async fn test_to_lab_grey_is_achromatic() {
    // Grey should have a*≈0 and b*≈0
    let mut inputs = color_input(0.5, 0.5, 0.5, 1.0);
    let result = OpColorOutputLab::run(&mut inputs).await.unwrap();
    match (&result.responses[1].value, &result.responses[2].value) {
        (Value::Decimal(a), Value::Decimal(b)) => {
            assert!((*a).abs() < 5.0, "grey a* should be near 0, got {}", a);
            assert!((*b).abs() < 5.0, "grey b* should be near 0, got {}", b);
        }
        other => panic!("Expected Decimals, got {:?}", other),
    }
}
