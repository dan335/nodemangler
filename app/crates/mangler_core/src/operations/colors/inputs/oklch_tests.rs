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
    assert_eq!(OpColorInputOklch::settings().name, "oklch");
    assert_eq!(OpColorInputOklch::create_inputs().len(), 4);
    assert_eq!(OpColorInputOklch::create_outputs().len(), 1);
}

#[tokio::test]
async fn produces_color() {
    let mut inputs = decimal_inputs(&[0.7, 0.1, 120.0, 1.0]);
    let result = OpColorInputOklch::run(&mut inputs).await.unwrap();
    assert!(matches!(result.responses[0].value, Value::Color(_)));
}
