use super::*;
use crate::color::Color;
use crate::input::Input;
use crate::value::Value;

#[tokio::test]
async fn test_invert_black_gives_white() {
    // Black (r=0, g=0, b=0, a=1) should become white (r=1, g=1, b=1, a=1)
    let mut inputs = vec![
        Input::new("color".to_string(), Value::Color(Color::from_srgb_float(0.0, 0.0, 0.0, 1.0)), None, None),
        Input::new("invert alpha".to_string(), Value::Bool(false), None, None),
    ];
    let result = OpColorManipulationInvert::run(&mut inputs).await.unwrap();
    match &result.responses[0].value {
        Value::Color(c) => {
            assert!((c.r - 1.0).abs() < 1e-5, "Expected r=1.0, got {}", c.r);
            assert!((c.g - 1.0).abs() < 1e-5, "Expected g=1.0, got {}", c.g);
            assert!((c.b - 1.0).abs() < 1e-5, "Expected b=1.0, got {}", c.b);
            assert!((c.a - 1.0).abs() < 1e-5, "Expected a=1.0, got {}", c.a);
        }
        other => panic!("Expected Color, got {:?}", other),
    }
}

#[tokio::test]
async fn test_invert_alpha() {
    // With invert_alpha=true, alpha 0.5 should remain 0.5 (1.0 - 0.5 = 0.5).
    // RGB: r=0 → 1, g=0 → 1, b=0 → 1. Alpha: 0.5 → 0.5.
    let mut inputs = vec![
        Input::new("color".to_string(), Value::Color(Color::from_srgb_float(0.0, 0.0, 0.0, 0.5)), None, None),
        Input::new("invert alpha".to_string(), Value::Bool(true), None, None),
    ];
    let result = OpColorManipulationInvert::run(&mut inputs).await.unwrap();
    match &result.responses[0].value {
        Value::Color(c) => {
            assert!((c.r - 1.0).abs() < 1e-5, "Expected r=1.0, got {}", c.r);
            assert!((c.g - 1.0).abs() < 1e-5, "Expected g=1.0, got {}", c.g);
            assert!((c.b - 1.0).abs() < 1e-5, "Expected b=1.0, got {}", c.b);
            // 1.0 - 0.5 = 0.5
            assert!((c.a - 0.5).abs() < 1e-5, "Expected a=0.5, got {}", c.a);
        }
        other => panic!("Expected Color, got {:?}", other),
    }
}

#[tokio::test]
async fn test_settings() {
    let s = OpColorManipulationInvert::settings();
    assert_eq!(s.name, "invert");
    assert_eq!(OpColorManipulationInvert::create_inputs().len(), 2);
    assert_eq!(OpColorManipulationInvert::create_outputs().len(), 1);
}
