use super::*;
use crate::input::Input;
use crate::value::Value;

fn make_inputs(condition: Value, if_true: Value, if_false: Value) -> Vec<Input> {
    vec![
        Input::new("condition".to_string(), condition, None, None),
        Input::new("if true".to_string(), if_true, None, None),
        Input::new("if false".to_string(), if_false, None, None),
    ]
}

#[tokio::test]
async fn test_select_true() {
    let mut inputs = make_inputs(Value::Bool(true), Value::Decimal(10.0), Value::Decimal(20.0));
    let result = OpLogicFlowSelect::run(&mut inputs).await.unwrap();
    match &result.responses[0].value {
        Value::Decimal(v) => assert!((*v - 10.0).abs() < 1e-6),
        other => panic!("Expected Decimal(10.0), got {:?}", other),
    }
}

#[tokio::test]
async fn test_select_false() {
    let mut inputs = make_inputs(Value::Bool(false), Value::Decimal(10.0), Value::Decimal(20.0));
    let result = OpLogicFlowSelect::run(&mut inputs).await.unwrap();
    match &result.responses[0].value {
        Value::Decimal(v) => assert!((*v - 20.0).abs() < 1e-6),
        other => panic!("Expected Decimal(20.0), got {:?}", other),
    }
}

#[tokio::test]
async fn test_select_integers() {
    let mut inputs = make_inputs(Value::Bool(true), Value::Integer(42), Value::Integer(0));
    let result = OpLogicFlowSelect::run(&mut inputs).await.unwrap();
    match &result.responses[0].value {
        Value::Integer(v) => assert_eq!(*v, 42),
        other => panic!("Expected Integer(42), got {:?}", other),
    }
}

#[tokio::test]
async fn test_select_text() {
    let mut inputs = make_inputs(
        Value::Bool(false),
        Value::Text("yes".to_string()),
        Value::Text("no".to_string()),
    );
    let result = OpLogicFlowSelect::run(&mut inputs).await.unwrap();
    match &result.responses[0].value {
        Value::Text(v) => assert_eq!(v, "no"),
        other => panic!("Expected Text(\"no\"), got {:?}", other),
    }
}

#[tokio::test]
async fn test_select_bools() {
    let mut inputs = make_inputs(Value::Bool(true), Value::Bool(true), Value::Bool(false));
    let result = OpLogicFlowSelect::run(&mut inputs).await.unwrap();
    assert!(matches!(result.responses[0].value, Value::Bool(true)));
}

#[tokio::test]
async fn test_select_condition_from_integer() {
    let mut inputs = make_inputs(Value::Integer(1), Value::Decimal(10.0), Value::Decimal(20.0));
    let result = OpLogicFlowSelect::run(&mut inputs).await.unwrap();
    match &result.responses[0].value {
        Value::Decimal(v) => assert!((*v - 10.0).abs() < 1e-6),
        other => panic!("Expected Decimal(10.0), got {:?}", other),
    }
}

#[tokio::test]
async fn test_select_settings() {
    let s = OpLogicFlowSelect::settings();
    assert_eq!(s.name, "select");
    assert_eq!(OpLogicFlowSelect::create_inputs().len(), 3);
    assert_eq!(OpLogicFlowSelect::create_outputs().len(), 1);
}

#[test]
fn test_select_branch_inputs_accept_any_type() {
    let inputs = OpLogicFlowSelect::create_inputs();
    assert!(!inputs[0].accepts_any_type, "condition input should not accept any type");
    assert!(inputs[1].accepts_any_type, "if true input should accept any type");
    assert!(inputs[2].accepts_any_type, "if false input should accept any type");
}

#[tokio::test]
async fn test_select_with_images() {
    use std::sync::Arc;
    use crate::float_image::FloatImage;
    use crate::get_id;

    let img_true = Value::Image {
        data: Arc::new(FloatImage::from_pixel(1, 1, 4, &[1.0, 0.0, 0.0, 1.0])),
        change_id: get_id(),
    };
    let img_false = Value::Image {
        data: Arc::new(FloatImage::from_pixel(1, 1, 4, &[0.0, 1.0, 0.0, 1.0])),
        change_id: get_id(),
    };

    let mut inputs = make_inputs(Value::Bool(true), img_true.clone(), img_false.clone());
    let result = OpLogicFlowSelect::run(&mut inputs).await.unwrap();
    assert!(matches!(result.responses[0].value, Value::Image { .. }));

    let mut inputs = make_inputs(Value::Bool(false), img_true, img_false);
    let result = OpLogicFlowSelect::run(&mut inputs).await.unwrap();
    assert!(matches!(result.responses[0].value, Value::Image { .. }));
}
