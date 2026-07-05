use super::*;
use crate::input::Input;
use crate::value::Value;

fn inputs(s: &str) -> Vec<Input> {
    vec![Input::new("input".to_string(), Value::Text(s.to_string()), None, None)]
}

#[tokio::test]
async fn test_parse_decimal_settings() {
    let s = OpNumberTextParseDecimal::settings();
    assert_eq!(s.name, "parse decimal");
    assert_eq!(OpNumberTextParseDecimal::create_outputs().len(), 1);
}

#[tokio::test]
async fn test_parse_decimal_ok() {
    let mut i = inputs("  3.5 ");
    let r = OpNumberTextParseDecimal::run(&mut i).await.unwrap();
    assert!(matches!(r.responses[0].value, Value::Decimal(v) if (v - 3.5).abs() < 1e-6));
}

#[tokio::test]
async fn test_parse_decimal_negative() {
    let mut i = inputs("-2");
    let r = OpNumberTextParseDecimal::run(&mut i).await.unwrap();
    assert!(matches!(r.responses[0].value, Value::Decimal(v) if (v + 2.0).abs() < 1e-6));
}

#[tokio::test]
async fn test_parse_decimal_error() {
    let mut i = inputs("hello");
    let r = OpNumberTextParseDecimal::run(&mut i).await;
    assert!(r.is_err());
}
