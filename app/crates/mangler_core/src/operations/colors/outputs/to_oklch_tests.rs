use super::*;
use crate::color::Color;
use crate::input::Input;
use crate::value::Value;

fn color_input(r: f32, g: f32, b: f32, a: f32) -> Vec<Input> {
    vec![Input::new("input".to_string(), Value::Color(Color::from_srgb_float(r, g, b, a)), None, None)]
}

#[tokio::test]
async fn settings_and_ports() {
    assert_eq!(OpColorOutputOklch::settings().name, "to oklch");
    assert_eq!(OpColorOutputOklch::create_inputs().len(), 1);
    assert_eq!(OpColorOutputOklch::create_outputs().len(), 4);
}

#[tokio::test]
async fn gray_is_achromatic() {
    let mut inputs = color_input(0.5, 0.5, 0.5, 1.0);
    let result = OpColorOutputOklch::run(&mut inputs).await.unwrap();
    // chroma of a neutral gray should be ~0
    match &result.responses[1].value {
        Value::Decimal(c) => assert!(c.abs() < 1e-3, "gray chroma should be ~0, got {}", c),
        other => panic!("Expected Decimal, got {:?}", other),
    }
}
