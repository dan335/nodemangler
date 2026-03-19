use super::*;
use crate::color::Color;
use crate::input::Input;
use crate::value::Value;

fn to_hex_inputs(color: Color, include_alpha: bool) -> Vec<Input> {
    vec![
        Input::new("color".to_string(), Value::Color(color), None, None),
        Input::new("include alpha".to_string(), Value::Bool(include_alpha), None, None),
    ]
}

#[tokio::test]
async fn test_to_hex_white() {
    let mut inputs = to_hex_inputs(Color::from_srgb_float(1.0, 1.0, 1.0, 1.0), false);
    let result = OpColorGenerationToHex::run(&mut inputs).await.unwrap();
    match &result.responses[0].value {
        Value::Text(s) => assert_eq!(s, "#FFFFFF"),
        other => panic!("Expected Text, got {:?}", other),
    }
}

#[tokio::test]
async fn test_to_hex_black() {
    let mut inputs = to_hex_inputs(Color::from_srgb_float(0.0, 0.0, 0.0, 1.0), false);
    let result = OpColorGenerationToHex::run(&mut inputs).await.unwrap();
    match &result.responses[0].value {
        Value::Text(s) => assert_eq!(s, "#000000"),
        other => panic!("Expected Text, got {:?}", other),
    }
}

#[tokio::test]
async fn test_to_hex_with_alpha() {
    let mut inputs = to_hex_inputs(Color::from_srgb_float(1.0, 0.0, 0.0, 1.0), true);
    let result = OpColorGenerationToHex::run(&mut inputs).await.unwrap();
    match &result.responses[0].value {
        Value::Text(s) => assert_eq!(s, "#FF0000FF"),
        other => panic!("Expected Text, got {:?}", other),
    }
}

#[tokio::test]
async fn test_settings() {
    let s = OpColorGenerationToHex::settings();
    assert_eq!(s.name, "to hex");
    assert_eq!(OpColorGenerationToHex::create_inputs().len(), 2);
    assert_eq!(OpColorGenerationToHex::create_outputs().len(), 1);
}
