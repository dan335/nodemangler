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
async fn test_nand_true_true() {
    let mut inputs = make_inputs(Value::Bool(true), Value::Bool(true));
    let result = OpLogicBoolNand::run(&mut inputs).await.unwrap();
    assert!(matches!(result.responses[0].value, Value::Bool(false)));
}

#[tokio::test]
async fn test_nand_true_false() {
    let mut inputs = make_inputs(Value::Bool(true), Value::Bool(false));
    let result = OpLogicBoolNand::run(&mut inputs).await.unwrap();
    assert!(matches!(result.responses[0].value, Value::Bool(true)));
}

#[tokio::test]
async fn test_nand_false_false() {
    let mut inputs = make_inputs(Value::Bool(false), Value::Bool(false));
    let result = OpLogicBoolNand::run(&mut inputs).await.unwrap();
    assert!(matches!(result.responses[0].value, Value::Bool(true)));
}

#[tokio::test]
async fn test_nand_settings() {
    let s = OpLogicBoolNand::settings();
    assert_eq!(s.name, "nand");
}
