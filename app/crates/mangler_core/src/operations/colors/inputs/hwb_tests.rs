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
    assert_eq!(OpColorInputHwb::settings().name, "hwb");
    assert_eq!(OpColorInputHwb::create_inputs().len(), 4);
    assert_eq!(OpColorInputHwb::create_outputs().len(), 1);
}

#[tokio::test]
async fn produces_color() {
    let mut inputs = decimal_inputs(&[120.0, 0.2, 0.2, 1.0]);
    let result = OpColorInputHwb::run(&mut inputs).await.unwrap();
    assert!(matches!(result.responses[0].value, Value::Color(_)));
}
