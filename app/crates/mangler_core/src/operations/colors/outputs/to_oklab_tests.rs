use super::*;
use crate::color::Color;
use crate::input::Input;
use crate::value::Value;

fn color_input(r: f32, g: f32, b: f32, a: f32) -> Vec<Input> {
    vec![Input::new("input".to_string(), Value::Color(Color::from_srgb_float(r, g, b, a)), None, None)]
}

#[tokio::test]
async fn settings_and_ports() {
    assert_eq!(OpColorOutputOklab::settings().name, "to oklab");
    assert_eq!(OpColorOutputOklab::create_inputs().len(), 1);
    assert_eq!(OpColorOutputOklab::create_outputs().len(), 4);
}

#[tokio::test]
async fn white_is_lightness_one() {
    let mut inputs = color_input(1.0, 1.0, 1.0, 1.0);
    let result = OpColorOutputOklab::run(&mut inputs).await.unwrap();
    assert_eq!(result.responses.len(), 4);
    match &result.responses[0].value {
        Value::Decimal(l) => assert!((*l - 1.0).abs() < 1e-3, "white L should be ~1, got {}", l),
        other => panic!("Expected Decimal, got {:?}", other),
    }
}
