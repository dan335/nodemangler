use super::*;
use crate::input::Input;
use crate::value::Value;

#[tokio::test]
async fn test_to_color_settings() {
    let s = OpColorCastToColor::settings();
    assert_eq!(s.name, "to color");
    assert_eq!(OpColorCastToColor::create_inputs().len(), 1);
    assert_eq!(OpColorCastToColor::create_outputs().len(), 1);
}

#[tokio::test]
async fn test_to_color_from_decimal() {
    let mut inputs = vec![Input::new("input".to_string(), Value::Decimal(0.5), None, None)];
    let result = OpColorCastToColor::run(&mut inputs).await.unwrap();
    match &result.responses[0].value {
        Value::Color(c) => {
            let (r, g, b, a) = c.to_srgb_float();
            assert!((r - 0.5).abs() < 0.01);
            assert!((g - 0.5).abs() < 0.01);
            assert!((b - 0.5).abs() < 0.01);
            assert!((a - 1.0).abs() < 0.01);
        }
        other => panic!("Expected Color, got {:?}", other),
    }
}

#[tokio::test]
async fn test_to_color_from_integer() {
    let mut inputs = vec![Input::new("input".to_string(), Value::Integer(255), None, None)];
    let result = OpColorCastToColor::run(&mut inputs).await.unwrap();
    match &result.responses[0].value {
        Value::Color(c) => {
            let (r, _, _, _) = c.to_srgb_float();
            assert!((r - 1.0).abs() < 0.01);
        }
        other => panic!("Expected Color, got {:?}", other),
    }
}

#[tokio::test]
async fn test_to_color_from_integer_zero() {
    let mut inputs = vec![Input::new("input".to_string(), Value::Integer(0), None, None)];
    let result = OpColorCastToColor::run(&mut inputs).await.unwrap();
    match &result.responses[0].value {
        Value::Color(c) => {
            let (r, g, b, _) = c.to_srgb_float();
            assert!(r.abs() < 0.01);
            assert!(g.abs() < 0.01);
            assert!(b.abs() < 0.01);
        }
        other => panic!("Expected Color, got {:?}", other),
    }
}

#[tokio::test]
async fn test_to_color_from_bool_true() {
    let mut inputs = vec![Input::new("input".to_string(), Value::Bool(true), None, None)];
    let result = OpColorCastToColor::run(&mut inputs).await.unwrap();
    match &result.responses[0].value {
        Value::Color(c) => {
            let (r, g, b, _) = c.to_srgb_float();
            assert!((r - 1.0).abs() < 0.01);
            assert!((g - 1.0).abs() < 0.01);
            assert!((b - 1.0).abs() < 0.01);
        }
        other => panic!("Expected Color, got {:?}", other),
    }
}

#[tokio::test]
async fn test_to_color_from_bool_false() {
    let mut inputs = vec![Input::new("input".to_string(), Value::Bool(false), None, None)];
    let result = OpColorCastToColor::run(&mut inputs).await.unwrap();
    match &result.responses[0].value {
        Value::Color(c) => {
            let (r, g, b, _) = c.to_srgb_float();
            assert!(r.abs() < 0.01);
            assert!(g.abs() < 0.01);
            assert!(b.abs() < 0.01);
        }
        other => panic!("Expected Color, got {:?}", other),
    }
}

#[tokio::test]
async fn test_to_color_from_decimal_zero() {
    let mut inputs = vec![Input::new("input".to_string(), Value::Decimal(0.0), None, None)];
    let result = OpColorCastToColor::run(&mut inputs).await.unwrap();
    match &result.responses[0].value {
        Value::Color(c) => {
            let (r, g, b, _) = c.to_srgb_float();
            assert!(r.abs() < 0.01);
            assert!(g.abs() < 0.01);
            assert!(b.abs() < 0.01);
        }
        other => panic!("Expected Color, got {:?}", other),
    }
}

#[tokio::test]
async fn test_to_color_from_decimal_one() {
    let mut inputs = vec![Input::new("input".to_string(), Value::Decimal(1.0), None, None)];
    let result = OpColorCastToColor::run(&mut inputs).await.unwrap();
    match &result.responses[0].value {
        Value::Color(c) => {
            let (r, g, b, _) = c.to_srgb_float();
            assert!((r - 1.0).abs() < 0.01);
            assert!((g - 1.0).abs() < 0.01);
            assert!((b - 1.0).abs() < 0.01);
        }
        other => panic!("Expected Color, got {:?}", other),
    }
}

#[tokio::test]
async fn test_to_color_passthrough() {
    let color = Color::from_srgb_float(0.2, 0.4, 0.6, 0.8);
    let mut inputs = vec![Input::new("input".to_string(), Value::Color(color), None, None)];
    let result = OpColorCastToColor::run(&mut inputs).await.unwrap();
    match &result.responses[0].value {
        Value::Color(c) => {
            let (r, g, b, a) = c.to_srgb_float();
            assert!((r - 0.2).abs() < 0.01);
            assert!((g - 0.4).abs() < 0.01);
            assert!((b - 0.6).abs() < 0.01);
            assert!((a - 0.8).abs() < 0.01);
        }
        other => panic!("Expected Color, got {:?}", other),
    }
}
