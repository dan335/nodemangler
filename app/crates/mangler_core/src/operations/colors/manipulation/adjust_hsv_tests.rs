use super::*;
use crate::color::Color;
use crate::input::Input;
use crate::value::Value;

#[tokio::test]
async fn test_hue_offset_180() {
    // Offsetting hue by 180 should produce the complementary hue
    let mut inputs = vec![
        Input::new("color".to_string(), Value::Color(Color::from_srgb_float(1.0, 0.0, 0.0, 1.0)), None, None),
        Input::new("hue offset".to_string(), Value::Decimal(180.0), None, None),
        Input::new("saturation offset".to_string(), Value::Decimal(0.0), None, None),
        Input::new("value offset".to_string(), Value::Decimal(0.0), None, None),
    ];
    let result = OpColorManipulationAdjustHsv::run(&mut inputs).await.unwrap();
    match &result.responses[0].value {
        Value::Color(_) => {}
        other => panic!("Expected Color, got {:?}", other),
    }
}

#[tokio::test]
async fn test_saturation_offset() {
    // Reducing saturation to zero should produce a neutral gray
    let mut inputs = vec![
        Input::new("color".to_string(), Value::Color(Color::from_srgb_float(1.0, 0.0, 0.0, 1.0)), None, None),
        Input::new("hue offset".to_string(), Value::Decimal(0.0), None, None),
        Input::new("saturation offset".to_string(), Value::Decimal(-1.0), None, None),
        Input::new("value offset".to_string(), Value::Decimal(0.0), None, None),
    ];
    let result = OpColorManipulationAdjustHsv::run(&mut inputs).await.unwrap();
    match &result.responses[0].value {
        Value::Color(c) => {
            // With saturation 0 the color should be a shade of gray (r≈g≈b)
            assert!((c.r - c.g).abs() < 1e-3, "Expected gray, r={} g={}", c.r, c.g);
            assert!((c.g - c.b).abs() < 1e-3, "Expected gray, g={} b={}", c.g, c.b);
        }
        other => panic!("Expected Color, got {:?}", other),
    }
}

#[tokio::test]
async fn test_value_offset() {
    // Reducing value to zero should produce black
    let mut inputs = vec![
        Input::new("color".to_string(), Value::Color(Color::from_srgb_float(1.0, 0.0, 0.0, 1.0)), None, None),
        Input::new("hue offset".to_string(), Value::Decimal(0.0), None, None),
        Input::new("saturation offset".to_string(), Value::Decimal(0.0), None, None),
        Input::new("value offset".to_string(), Value::Decimal(-1.0), None, None),
    ];
    let result = OpColorManipulationAdjustHsv::run(&mut inputs).await.unwrap();
    match &result.responses[0].value {
        Value::Color(c) => {
            assert!(c.r.abs() < 1e-5, "Expected r≈0.0, got {}", c.r);
            assert!(c.g.abs() < 1e-5, "Expected g≈0.0, got {}", c.g);
            assert!(c.b.abs() < 1e-5, "Expected b≈0.0, got {}", c.b);
        }
        other => panic!("Expected Color, got {:?}", other),
    }
}

#[tokio::test]
async fn test_settings() {
    let s = OpColorManipulationAdjustHsv::settings();
    assert_eq!(s.name, "adjust hsv");
    assert_eq!(OpColorManipulationAdjustHsv::create_inputs().len(), 4);
    assert_eq!(OpColorManipulationAdjustHsv::create_outputs().len(), 1);
}
