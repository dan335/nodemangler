use super::*;
use crate::input::Input;
use crate::value::Value;

fn decimal_inputs(vals: &[f32]) -> Vec<Input> {
    vals.iter()
        .enumerate()
        .map(|(i, v)| Input::new(format!("v{}",  i), Value::Decimal(*v), None, None))
        .collect()
}

#[tokio::test]
async fn test_hsv_input() {
    let mut inputs = decimal_inputs(&[120.0, 1.0, 1.0, 1.0]);
    let result = OpColorInputHsva::run(&mut inputs).await.unwrap();
    match &result.responses[0].value {
        Value::Color(_) => {}
        other => panic!("Expected Color, got {:?}", other),
    }
}

#[tokio::test]
async fn test_hsv_settings() {
    let s = OpColorInputHsva::settings();
    assert_eq!(s.name, "hsv");
    assert_eq!(OpColorInputHsva::create_inputs().len(), 4);
    assert_eq!(OpColorInputHsva::create_outputs().len(), 1);
}

#[tokio::test]
async fn test_hsv_black() {
    // V=0 should give black
    let mut inputs = decimal_inputs(&[0.0, 0.0, 0.0, 1.0]);
    let result = OpColorInputHsva::run(&mut inputs).await.unwrap();
    match &result.responses[0].value {
        Value::Color(c) => {
            let (r, g, b, _) = c.to_srgb_float();
            assert!(r.abs() < 0.01, "black R should be ~0, got {}", r);
            assert!(g.abs() < 0.01, "black G should be ~0, got {}", g);
            assert!(b.abs() < 0.01, "black B should be ~0, got {}", b);
        }
        other => panic!("Expected Color, got {:?}", other),
    }
}

#[tokio::test]
async fn test_hsv_white() {
    // S=0, V=1 should give white
    let mut inputs = decimal_inputs(&[0.0, 0.0, 1.0, 1.0]);
    let result = OpColorInputHsva::run(&mut inputs).await.unwrap();
    match &result.responses[0].value {
        Value::Color(c) => {
            let (r, g, b, _) = c.to_srgb_float();
            assert!((r - 1.0).abs() < 0.01, "white R should be ~1, got {}", r);
            assert!((g - 1.0).abs() < 0.01, "white G should be ~1, got {}", g);
            assert!((b - 1.0).abs() < 0.01, "white B should be ~1, got {}", b);
        }
        other => panic!("Expected Color, got {:?}", other),
    }
}

#[tokio::test]
async fn test_hsv_zero_alpha() {
    let mut inputs = decimal_inputs(&[120.0, 1.0, 1.0, 0.0]);
    let result = OpColorInputHsva::run(&mut inputs).await.unwrap();
    match &result.responses[0].value {
        Value::Color(c) => {
            let (_, _, _, a) = c.to_srgb_float();
            assert!(a.abs() < 0.01, "alpha 0 should round trip, got {}", a);
        }
        other => panic!("Expected Color, got {:?}", other),
    }
}

#[tokio::test]
async fn test_hsv_pure_blue() {
    // Hue=240, S=1, V=1 should give blue
    let mut inputs = decimal_inputs(&[240.0, 1.0, 1.0, 1.0]);
    let result = OpColorInputHsva::run(&mut inputs).await.unwrap();
    match &result.responses[0].value {
        Value::Color(c) => {
            let (r, g, b, _) = c.to_srgb_float();
            assert!(r.abs() < 0.01, "blue R should be ~0, got {}", r);
            assert!(g.abs() < 0.01, "blue G should be ~0, got {}", g);
            assert!((b - 1.0).abs() < 0.01, "blue B should be ~1, got {}", b);
        }
        other => panic!("Expected Color, got {:?}", other),
    }
}
