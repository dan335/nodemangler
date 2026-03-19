use super::*;
use crate::color::Color;
use crate::input::Input;
use crate::value::Value;

#[tokio::test]
async fn test_grayscale_white_is_white() {
    // White in sRGB should remain white after grayscale conversion
    let mut inputs = vec![
        Input::new("color".to_string(), Value::Color(Color::from_srgb_float(1.0, 1.0, 1.0, 1.0)), None, None),
    ];
    let result = OpColorManipulationGrayscale::run(&mut inputs).await.unwrap();
    match &result.responses[0].value {
        Value::Color(c) => {
            assert!((c.r - 1.0).abs() < 1e-3, "Expected r≈1.0, got {}", c.r);
            assert!((c.g - 1.0).abs() < 1e-3, "Expected g≈1.0, got {}", c.g);
            assert!((c.b - 1.0).abs() < 1e-3, "Expected b≈1.0, got {}", c.b);
        }
        other => panic!("Expected Color, got {:?}", other),
    }
}

#[tokio::test]
async fn test_grayscale_black_is_black() {
    // Black should remain black after grayscale conversion
    let mut inputs = vec![
        Input::new("color".to_string(), Value::Color(Color::from_srgb_float(0.0, 0.0, 0.0, 1.0)), None, None),
    ];
    let result = OpColorManipulationGrayscale::run(&mut inputs).await.unwrap();
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
    let s = OpColorManipulationGrayscale::settings();
    assert_eq!(s.name, "grayscale");
    assert_eq!(OpColorManipulationGrayscale::create_inputs().len(), 1);
    assert_eq!(OpColorManipulationGrayscale::create_outputs().len(), 2);
}

#[tokio::test]
async fn test_grayscale_outputs_luminance() {
    // Verify that two output responses are produced (color + luminance)
    let mut inputs = vec![
        Input::new("color".to_string(), Value::Color(Color::from_srgb_float(0.5, 0.5, 0.5, 1.0)), None, None),
    ];
    let result = OpColorManipulationGrayscale::run(&mut inputs).await.unwrap();
    assert_eq!(result.responses.len(), 2, "Expected 2 output responses");
    match &result.responses[1].value {
        Value::Decimal(_) => {}
        other => panic!("Expected Decimal for luminance output, got {:?}", other),
    }
}
