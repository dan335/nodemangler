use super::*;
use crate::input::Input;
use crate::value::Value;

fn make_inputs(a: Value, b: Value) -> Vec<Input> {
    vec![
        Input::new("a".to_string(), a, None, None),
        Input::new("b".to_string(), b, None, None),
    ]
}

#[tokio::test]
async fn test_equal_integers_true() {
    let mut inputs = make_inputs(Value::Integer(5), Value::Integer(5));
    let result = OpLogicCompareEqual::run(&mut inputs).await.unwrap();
    assert!(matches!(result.responses[0].value, Value::Bool(true)));
}

#[tokio::test]
async fn test_equal_integers_false() {
    let mut inputs = make_inputs(Value::Integer(5), Value::Integer(10));
    let result = OpLogicCompareEqual::run(&mut inputs).await.unwrap();
    assert!(matches!(result.responses[0].value, Value::Bool(false)));
}

#[tokio::test]
async fn test_equal_decimals_true() {
    let mut inputs = make_inputs(Value::Decimal(3.14), Value::Decimal(3.14));
    let result = OpLogicCompareEqual::run(&mut inputs).await.unwrap();
    assert!(matches!(result.responses[0].value, Value::Bool(true)));
}

#[tokio::test]
async fn test_equal_decimals_false() {
    let mut inputs = make_inputs(Value::Decimal(3.14), Value::Decimal(2.71));
    let result = OpLogicCompareEqual::run(&mut inputs).await.unwrap();
    assert!(matches!(result.responses[0].value, Value::Bool(false)));
}

#[tokio::test]
async fn test_equal_mixed_int_decimal() {
    let mut inputs = make_inputs(Value::Integer(5), Value::Decimal(5.0));
    let result = OpLogicCompareEqual::run(&mut inputs).await.unwrap();
    assert!(matches!(result.responses[0].value, Value::Bool(true)));
}

#[tokio::test]
async fn test_equal_bools() {
    let mut inputs = make_inputs(Value::Bool(true), Value::Bool(true));
    let result = OpLogicCompareEqual::run(&mut inputs).await.unwrap();
    assert!(matches!(result.responses[0].value, Value::Bool(true)));
}

#[tokio::test]
async fn test_equal_text() {
    let mut inputs = make_inputs(Value::Text("hello".to_string()), Value::Text("hello".to_string()));
    let result = OpLogicCompareEqual::run(&mut inputs).await.unwrap();
    assert!(matches!(result.responses[0].value, Value::Bool(true)));
}

#[tokio::test]
async fn test_equal_text_false() {
    let mut inputs = make_inputs(Value::Text("hello".to_string()), Value::Text("world".to_string()));
    let result = OpLogicCompareEqual::run(&mut inputs).await.unwrap();
    assert!(matches!(result.responses[0].value, Value::Bool(false)));
}

// Bool/Numeric mixed: true converts to 1.0, false to 0.0 (JS/Rust semantics)
#[tokio::test]
async fn test_equal_bool_true_decimal_one() {
    let mut inputs = make_inputs(Value::Bool(true), Value::Decimal(1.0));
    let result = OpLogicCompareEqual::run(&mut inputs).await.unwrap();
    assert!(matches!(result.responses[0].value, Value::Bool(true)));
}

#[tokio::test]
async fn test_equal_bool_true_decimal_point_one() {
    // 0.1 != true  (true == 1.0, not 0.1)
    let mut inputs = make_inputs(Value::Bool(true), Value::Decimal(0.1));
    let result = OpLogicCompareEqual::run(&mut inputs).await.unwrap();
    assert!(matches!(result.responses[0].value, Value::Bool(false)));
}

#[tokio::test]
async fn test_equal_bool_true_decimal_neg_point_one() {
    // -0.1 != true
    let mut inputs = make_inputs(Value::Bool(true), Value::Decimal(-0.1));
    let result = OpLogicCompareEqual::run(&mut inputs).await.unwrap();
    assert!(matches!(result.responses[0].value, Value::Bool(false)));
}

#[tokio::test]
async fn test_equal_decimal_point_one_bool_true() {
    // symmetric: 0.1 != true
    let mut inputs = make_inputs(Value::Decimal(0.1), Value::Bool(true));
    let result = OpLogicCompareEqual::run(&mut inputs).await.unwrap();
    assert!(matches!(result.responses[0].value, Value::Bool(false)));
}

#[tokio::test]
async fn test_equal_decimal_neg_point_one_bool_true() {
    // symmetric: -0.1 != true
    let mut inputs = make_inputs(Value::Decimal(-0.1), Value::Bool(true));
    let result = OpLogicCompareEqual::run(&mut inputs).await.unwrap();
    assert!(matches!(result.responses[0].value, Value::Bool(false)));
}

#[tokio::test]
async fn test_equal_bool_false_decimal_zero() {
    // false == 0.0
    let mut inputs = make_inputs(Value::Bool(false), Value::Decimal(0.0));
    let result = OpLogicCompareEqual::run(&mut inputs).await.unwrap();
    assert!(matches!(result.responses[0].value, Value::Bool(true)));
}

#[tokio::test]
async fn test_equal_bool_true_integer_one() {
    let mut inputs = make_inputs(Value::Bool(true), Value::Integer(1));
    let result = OpLogicCompareEqual::run(&mut inputs).await.unwrap();
    assert!(matches!(result.responses[0].value, Value::Bool(true)));
}

#[tokio::test]
async fn test_equal_bool_true_integer_zero() {
    let mut inputs = make_inputs(Value::Bool(true), Value::Integer(0));
    let result = OpLogicCompareEqual::run(&mut inputs).await.unwrap();
    assert!(matches!(result.responses[0].value, Value::Bool(false)));
}

#[tokio::test]
async fn test_equal_bool_false_integer_zero() {
    let mut inputs = make_inputs(Value::Bool(false), Value::Integer(0));
    let result = OpLogicCompareEqual::run(&mut inputs).await.unwrap();
    assert!(matches!(result.responses[0].value, Value::Bool(true)));
}

#[tokio::test]
async fn test_equal_settings() {
    let s = OpLogicCompareEqual::settings();
    assert_eq!(s.name, "equal");
    assert_eq!(OpLogicCompareEqual::create_inputs().len(), 2);
    assert_eq!(OpLogicCompareEqual::create_outputs().len(), 1);
}
