use super::*;
use crate::input::Input;
use crate::value::Value;

fn decimal_inputs(vals: &[f32]) -> Vec<Input> {
    vals.iter()
        .enumerate()
        .map(|(i, v)| Input::new(format!("v{}", i), Value::Decimal(*v), None, None))
        .collect()
}

#[tokio::test]
async fn settings_and_ports() {
    assert_eq!(OpColorInputXyy::settings().name, "xyy");
    assert_eq!(OpColorInputXyy::create_inputs().len(), 4);
    assert_eq!(OpColorInputXyy::create_outputs().len(), 1);
}

#[tokio::test]
async fn produces_color() {
    let mut inputs = decimal_inputs(&[0.3127, 0.329, 0.5, 1.0]);
    let result = OpColorInputXyy::run(&mut inputs).await.unwrap();
    assert!(matches!(result.responses[0].value, Value::Color(_)));
}
