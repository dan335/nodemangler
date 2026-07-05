use super::*;
use crate::input::Input;
use crate::value::Value;

fn inputs(s: &str) -> Vec<Input> {
    vec![Input::new("input".to_string(), Value::Text(s.to_string()), None, None)]
}

#[tokio::test]
async fn test_parse_integer_settings() {
    let s = OpNumberTextParseInteger::settings();
    assert_eq!(s.name, "parse integer");
    assert_eq!(OpNumberTextParseInteger::create_outputs().len(), 1);
}

#[tokio::test]
async fn test_parse_integer_ok() {
    let mut i = inputs("  42 ");
    let r = OpNumberTextParseInteger::run(&mut i).await.unwrap();
    assert!(matches!(r.responses[0].value, Value::Integer(42)));
}

#[tokio::test]
async fn test_parse_integer_negative() {
    let mut i = inputs("-7");
    let r = OpNumberTextParseInteger::run(&mut i).await.unwrap();
    assert!(matches!(r.responses[0].value, Value::Integer(-7)));
}

#[tokio::test]
async fn test_parse_integer_decimal_errors() {
    let mut i = inputs("3.5");
    let r = OpNumberTextParseInteger::run(&mut i).await;
    assert!(r.is_err());
}
