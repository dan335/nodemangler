use super::*;
use crate::color::Color;
use crate::input::Input;
use crate::value::Value;

#[tokio::test]
async fn test_clamp_high_values() {
    // Values above max should be pulled down to max
    let mut inputs = vec![
        Input::new("color".to_string(), Value::Color(Color::from_srgb_float(1.0, 1.0, 1.0, 1.0)), None, None),
        Input::new("min".to_string(), Value::Decimal(0.0), None, None),
        Input::new("max".to_string(), Value::Decimal(0.5), None, None),
    ];
    let result = OpColorManipulationClamp::run(&mut inputs).await.unwrap();
    match &result.responses[0].value {
        Value::Color(c) => {
            assert!((c.r - 0.5).abs() < 1e-5, "Expected r=0.5, got {}", c.r);
            assert!((c.g - 0.5).abs() < 1e-5, "Expected g=0.5, got {}", c.g);
            assert!((c.b - 0.5).abs() < 1e-5, "Expected b=0.5, got {}", c.b);
            // Alpha should be unchanged
            assert!((c.a - 1.0).abs() < 1e-5, "Expected a=1.0, got {}", c.a);
        }
        other => panic!("Expected Color, got {:?}", other),
    }
}

#[tokio::test]
async fn test_clamp_low_values() {
    // Values below min should be pulled up to min
    let mut inputs = vec![
        Input::new("color".to_string(), Value::Color(Color::from_srgb_float(0.0, 0.0, 0.0, 1.0)), None, None),
        Input::new("min".to_string(), Value::Decimal(0.3), None, None),
        Input::new("max".to_string(), Value::Decimal(1.0), None, None),
    ];
    let result = OpColorManipulationClamp::run(&mut inputs).await.unwrap();
    match &result.responses[0].value {
        Value::Color(c) => {
            assert!((c.r - 0.3).abs() < 1e-5, "Expected r=0.3, got {}", c.r);
            assert!((c.g - 0.3).abs() < 1e-5, "Expected g=0.3, got {}", c.g);
            assert!((c.b - 0.3).abs() < 1e-5, "Expected b=0.3, got {}", c.b);
        }
        other => panic!("Expected Color, got {:?}", other),
    }
}

#[tokio::test]
async fn test_settings() {
    let s = OpColorManipulationClamp::settings();
    assert_eq!(s.name, "clamp");
    assert_eq!(OpColorManipulationClamp::create_inputs().len(), 3);
    assert_eq!(OpColorManipulationClamp::create_outputs().len(), 1);
}
