use super::*;
use crate::input::Input;
use crate::value::Value;

fn hex_inputs(hex: &str) -> Vec<Input> {
    vec![
        Input::new("hex".to_string(), Value::Text(hex.to_string()), None, None),
    ]
}

#[tokio::test]
async fn test_from_hex_white() {
    let mut inputs = hex_inputs("#FFFFFF");
    let result = OpColorGenerationFromHex::run(&mut inputs).await.unwrap();
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
async fn test_from_hex_black() {
    let mut inputs = hex_inputs("#000000");
    let result = OpColorGenerationFromHex::run(&mut inputs).await.unwrap();
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
async fn test_from_hex_with_alpha() {
    // #FF000080 → r=1.0, g=0.0, b=0.0, a≈0x80/0xFF = 128/255 ≈ 0.502
    let mut inputs = hex_inputs("#FF000080");
    let result = OpColorGenerationFromHex::run(&mut inputs).await.unwrap();
    match &result.responses[0].value {
        Value::Color(c) => {
            let (r, _g, _b, a) = c.to_srgb_float();
            assert!((r - 1.0).abs() < 0.01, "R should be ~1.0, got {}", r);
            assert!((a - 0.502).abs() < 0.01, "A should be ~0.502, got {}", a);
        }
        other => panic!("Expected Color, got {:?}", other),
    }
}

#[tokio::test]
async fn test_from_hex_without_hash() {
    // Input without leading '#' should still parse correctly
    let mut inputs = hex_inputs("FF0000");
    let result = OpColorGenerationFromHex::run(&mut inputs).await.unwrap();
    match &result.responses[0].value {
        Value::Color(c) => {
            let (r, g, b, _) = c.to_srgb_float();
            assert!((r - 1.0).abs() < 0.01, "R should be ~1.0, got {}", r);
            assert!(g.abs() < 0.01, "G should be ~0, got {}", g);
            assert!(b.abs() < 0.01, "B should be ~0, got {}", b);
        }
        other => panic!("Expected Color, got {:?}", other),
    }
}

#[tokio::test]
async fn test_settings() {
    let s = OpColorGenerationFromHex::settings();
    assert_eq!(s.name, "from hex");
    assert_eq!(OpColorGenerationFromHex::create_inputs().len(), 1);
    assert_eq!(OpColorGenerationFromHex::create_outputs().len(), 1);
}
