use super::*;
use crate::input::Input;
use crate::value::Value;

#[tokio::test]
async fn test_format_number_settings() {
    let s = OpTextFormatNumber::settings();
    assert_eq!(s.name, "format number");
    assert_eq!(OpTextFormatNumber::create_inputs().len(), 4);
    assert_eq!(OpTextFormatNumber::create_outputs().len(), 1);
}

#[tokio::test]
async fn test_format_number_decimals() {
    let mut inputs = vec![
        Input::new("value".to_string(), Value::Decimal(3.14159), None, None),
        Input::new("decimals".to_string(), Value::Integer(2), None, None),
        Input::new("min width".to_string(), Value::Integer(0), None, None),
        Input::new("pad zeros".to_string(), Value::Bool(false), None, None),
    ];
    let result = OpTextFormatNumber::run(&mut inputs).await.unwrap();
    match &result.responses[0].value {
        Value::Text(v) => assert_eq!(v, "3.14"),
        other => panic!("Expected Text, got {:?}", other),
    }
}

#[tokio::test]
async fn test_format_number_zero_pad() {
    let mut inputs = vec![
        Input::new("value".to_string(), Value::Decimal(5.0), None, None),
        Input::new("decimals".to_string(), Value::Integer(1), None, None),
        Input::new("min width".to_string(), Value::Integer(6), None, None),
        Input::new("pad zeros".to_string(), Value::Bool(true), None, None),
    ];
    let result = OpTextFormatNumber::run(&mut inputs).await.unwrap();
    match &result.responses[0].value {
        Value::Text(v) => assert_eq!(v, "0005.0"),
        other => panic!("Expected Text, got {:?}", other),
    }
}

#[tokio::test]
async fn test_format_number_space_pad() {
    let mut inputs = vec![
        Input::new("value".to_string(), Value::Decimal(5.0), None, None),
        Input::new("decimals".to_string(), Value::Integer(1), None, None),
        Input::new("min width".to_string(), Value::Integer(6), None, None),
        Input::new("pad zeros".to_string(), Value::Bool(false), None, None),
    ];
    let result = OpTextFormatNumber::run(&mut inputs).await.unwrap();
    match &result.responses[0].value {
        Value::Text(v) => assert_eq!(v, "   5.0"),
        other => panic!("Expected Text, got {:?}", other),
    }
}
