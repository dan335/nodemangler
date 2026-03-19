use super::*;

macro_rules! assert_value {
    ($val:expr, Integer($expected:expr)) => {
        match &$val { Value::Integer(v) => assert_eq!(*v, $expected), other => panic!("Expected Integer({}), got {:?}", $expected, other) }
    };
    ($val:expr, Decimal($expected:expr)) => {
        match &$val { Value::Decimal(v) => assert!((*v - $expected).abs() < 1e-6, "Expected Decimal({}), got Decimal({})", $expected, v), other => panic!("Expected Decimal({}), got {:?}", $expected, other) }
    };
    ($val:expr, Bool($expected:expr)) => {
        match &$val { Value::Bool(v) => assert_eq!(*v, $expected), other => panic!("Expected Bool({}), got {:?}", $expected, other) }
    };
    ($val:expr, Text($expected:expr)) => {
        match &$val { Value::Text(v) => assert_eq!(v, $expected), other => panic!("Expected Text, got {:?}", other) }
    };
}

fn make_inputs(a: Value, b: Value) -> Vec<Input> {
    vec![
        Input::new("a".to_string(), a, None, None),
        Input::new("b".to_string(), b, None, None),
    ]
}

#[tokio::test]
async fn test_add_decimal_decimal() {
    let mut inputs = make_inputs(
        Value::Decimal(5.0),
        Value::Decimal(10.0),
    );
    let result = OpNumberMathAdd::run(&mut inputs).await.unwrap();
    assert_value!(result.responses[0].value, Decimal(15.0));
}

#[tokio::test]
async fn test_add_integer_integer() {
    let mut inputs = make_inputs(
        Value::Integer(5),
        Value::Integer(10),
    );
    let result = OpNumberMathAdd::run(&mut inputs).await.unwrap();
    assert_value!(result.responses[0].value, Integer(15));
}

#[tokio::test]
async fn test_add_integer_decimal() {
    let mut inputs = make_inputs(
        Value::Integer(5),
        Value::Decimal(2.5),
    );
    let result = OpNumberMathAdd::run(&mut inputs).await.unwrap();
    assert_value!(result.responses[0].value, Decimal(7.5));
}

#[tokio::test]
async fn test_add_decimal_integer() {
    let mut inputs = make_inputs(
        Value::Decimal(2.5),
        Value::Integer(5),
    );
    let result = OpNumberMathAdd::run(&mut inputs).await.unwrap();
    assert_value!(result.responses[0].value, Decimal(7.5));
}

#[tokio::test]
async fn test_add_bool_true_integer() {
    let mut inputs = make_inputs(
        Value::Bool(true),
        Value::Integer(5),
    );
    let result = OpNumberMathAdd::run(&mut inputs).await.unwrap();
    assert_value!(result.responses[0].value, Integer(6));
}

#[tokio::test]
async fn test_add_bool_false_integer() {
    let mut inputs = make_inputs(
        Value::Bool(false),
        Value::Integer(5),
    );
    let result = OpNumberMathAdd::run(&mut inputs).await.unwrap();
    assert_value!(result.responses[0].value, Integer(5));
}

#[tokio::test]
async fn test_add_bool_bool() {
    let mut inputs = make_inputs(
        Value::Bool(true),
        Value::Bool(false),
    );
    let result = OpNumberMathAdd::run(&mut inputs).await.unwrap();
    assert_value!(result.responses[0].value, Bool(true));
}

#[tokio::test]
async fn test_add_bool_decimal() {
    let mut inputs = make_inputs(
        Value::Bool(true),
        Value::Decimal(5.5),
    );
    let result = OpNumberMathAdd::run(&mut inputs).await.unwrap();
    assert_value!(result.responses[0].value, Decimal(6.5));
}

#[tokio::test]
async fn test_add_integer_bool_true() {
    let mut inputs = make_inputs(
        Value::Integer(10),
        Value::Bool(true),
    );
    let result = OpNumberMathAdd::run(&mut inputs).await.unwrap();
    assert_value!(result.responses[0].value, Integer(11));
}

#[tokio::test]
async fn test_add_decimal_bool_true() {
    let mut inputs = make_inputs(
        Value::Decimal(10.0),
        Value::Bool(true),
    );
    let result = OpNumberMathAdd::run(&mut inputs).await.unwrap();
    assert_value!(result.responses[0].value, Decimal(11.0));
}

#[tokio::test]
async fn test_add_decimal_zero() {
    let mut inputs = make_inputs(
        Value::Decimal(0.0),
        Value::Decimal(0.0),
    );
    let result = OpNumberMathAdd::run(&mut inputs).await.unwrap();
    assert_value!(result.responses[0].value, Decimal(0.0));
}

#[tokio::test]
async fn test_add_negative_numbers() {
    let mut inputs = make_inputs(
        Value::Integer(-5),
        Value::Integer(-10),
    );
    let result = OpNumberMathAdd::run(&mut inputs).await.unwrap();
    assert_value!(result.responses[0].value, Integer(-15));
}

#[tokio::test]
async fn test_add_text_concat() {
    let mut inputs = make_inputs(
        Value::Bool(true),
        Value::Text("hello".to_string()),
    );
    let result = OpNumberMathAdd::run(&mut inputs).await.unwrap();
    assert_value!(result.responses[0].value, Text("truehello"));
}

#[tokio::test]
async fn test_add_integer_text_concat() {
    let mut inputs = make_inputs(
        Value::Integer(42),
        Value::Text("hello".to_string()),
    );
    let result = OpNumberMathAdd::run(&mut inputs).await.unwrap();
    assert_value!(result.responses[0].value, Text("42hello"));
}

#[tokio::test]
async fn test_add_settings() {
    let settings = OpNumberMathAdd::settings();
    assert_eq!(settings.name, "add");
}

#[tokio::test]
async fn test_add_create_inputs_count() {
    let inputs = OpNumberMathAdd::create_inputs();
    assert_eq!(inputs.len(), 2);
}

#[tokio::test]
async fn test_add_create_outputs_count() {
    let outputs = OpNumberMathAdd::create_outputs();
    assert_eq!(outputs.len(), 1);
}

#[tokio::test]
async fn test_add_large_integers() {
    let mut inputs = make_inputs(
        Value::Integer(i32::MAX / 2),
        Value::Integer(i32::MAX / 2),
    );
    let result = OpNumberMathAdd::run(&mut inputs).await.unwrap();
    assert_value!(result.responses[0].value, Integer(i32::MAX - 1));
}

#[tokio::test]
async fn test_add_large_decimals() {
    let mut inputs = make_inputs(
        Value::Decimal(1e15_f32),
        Value::Decimal(1e15_f32),
    );
    let result = OpNumberMathAdd::run(&mut inputs).await.unwrap();
    match &result.responses[0].value {
        Value::Decimal(v) => assert!(*v > 0.0),
        other => panic!("Expected Decimal, got {:?}", other),
    }
}

#[tokio::test]
async fn test_add_tiny_decimals() {
    let mut inputs = make_inputs(
        Value::Decimal(0.0001),
        Value::Decimal(0.0001),
    );
    let result = OpNumberMathAdd::run(&mut inputs).await.unwrap();
    assert_value!(result.responses[0].value, Decimal(0.0002));
}

#[tokio::test]
async fn test_add_mixed_sign() {
    let mut inputs = make_inputs(
        Value::Integer(100),
        Value::Integer(-100),
    );
    let result = OpNumberMathAdd::run(&mut inputs).await.unwrap();
    assert_value!(result.responses[0].value, Integer(0));
}

#[tokio::test]
async fn test_add_decimal_negative() {
    let mut inputs = make_inputs(
        Value::Decimal(-3.5),
        Value::Decimal(-1.5),
    );
    let result = OpNumberMathAdd::run(&mut inputs).await.unwrap();
    assert_value!(result.responses[0].value, Decimal(-5.0));
}

#[tokio::test]
async fn test_add_integer_zero() {
    let mut inputs = make_inputs(
        Value::Integer(0),
        Value::Integer(0),
    );
    let result = OpNumberMathAdd::run(&mut inputs).await.unwrap();
    assert_value!(result.responses[0].value, Integer(0));
}

#[tokio::test]
async fn test_add_invalid_type_returns_error() {
    let mut inputs = vec![
        Input::new("a".to_string(), Value::Bool(true), None, None),
        Input::new("b".to_string(), Value::Trigger, None, None),
    ];
    let result = OpNumberMathAdd::run(&mut inputs).await;
    assert!(result.is_err(), "Expected error for unsupported type combination");
}

#[tokio::test]
async fn test_add_bool_false_decimal() {
    let mut inputs = make_inputs(
        Value::Bool(false),
        Value::Decimal(5.5),
    );
    let result = OpNumberMathAdd::run(&mut inputs).await.unwrap();
    assert_value!(result.responses[0].value, Decimal(5.5));
}

#[tokio::test]
async fn test_add_integer_decimal_fractional_result() {
    let mut inputs = make_inputs(
        Value::Integer(3),
        Value::Decimal(0.14159),
    );
    let result = OpNumberMathAdd::run(&mut inputs).await.unwrap();
    match &result.responses[0].value {
        Value::Decimal(v) => assert!((*v - 3.14159).abs() < 1e-4),
        other => panic!("Expected Decimal, got {:?}", other),
    }
}
