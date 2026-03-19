use super::*;
use crate::color::Color;
use crate::input::Input;
use crate::value::Value;

#[tokio::test]
async fn test_set_alpha_replace() {
    // Replace mode: alpha should be exactly the provided value regardless of original
    let mut inputs = vec![
        Input::new("color".to_string(), Value::Color(Color::from_srgb_float(1.0, 0.5, 0.25, 0.8)), None, None),
        Input::new("alpha".to_string(), Value::Decimal(0.4), None, None),
        Input::new("multiply".to_string(), Value::Bool(false), None, None),
    ];
    let result = OpColorManipulationSetAlpha::run(&mut inputs).await.unwrap();
    match &result.responses[0].value {
        Value::Color(c) => {
            assert!((c.a - 0.4).abs() < 1e-5, "Expected a=0.4, got {}", c.a);
            // RGB channels should be unchanged
            assert!((c.r - 1.0).abs() < 1e-5, "Expected r=1.0, got {}", c.r);
            assert!((c.g - 0.5).abs() < 1e-5, "Expected g=0.5, got {}", c.g);
            assert!((c.b - 0.25).abs() < 1e-5, "Expected b=0.25, got {}", c.b);
        }
        other => panic!("Expected Color, got {:?}", other),
    }
}

#[tokio::test]
async fn test_set_alpha_multiply() {
    // Multiply mode: new alpha = original_alpha * alpha_input
    // 0.8 * 0.5 = 0.4
    let mut inputs = vec![
        Input::new("color".to_string(), Value::Color(Color::from_srgb_float(1.0, 0.5, 0.25, 0.8)), None, None),
        Input::new("alpha".to_string(), Value::Decimal(0.5), None, None),
        Input::new("multiply".to_string(), Value::Bool(true), None, None),
    ];
    let result = OpColorManipulationSetAlpha::run(&mut inputs).await.unwrap();
    match &result.responses[0].value {
        Value::Color(c) => {
            assert!((c.a - 0.4).abs() < 1e-5, "Expected a=0.4 (0.8*0.5), got {}", c.a);
        }
        other => panic!("Expected Color, got {:?}", other),
    }
}

#[tokio::test]
async fn test_settings() {
    let s = OpColorManipulationSetAlpha::settings();
    assert_eq!(s.name, "set alpha");
    assert_eq!(OpColorManipulationSetAlpha::create_inputs().len(), 3);
    assert_eq!(OpColorManipulationSetAlpha::create_outputs().len(), 1);
}
