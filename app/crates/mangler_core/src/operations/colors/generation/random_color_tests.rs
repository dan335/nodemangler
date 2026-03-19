use super::*;
use crate::input::Input;
use crate::value::Value;

fn random_color_inputs(min_sat: f32, max_sat: f32, min_light: f32, max_light: f32) -> Vec<Input> {
    vec![
        Input::new("generate".to_string(), Value::Trigger, None, None),
        Input::new("min saturation".to_string(), Value::Decimal(min_sat), None, None),
        Input::new("max saturation".to_string(), Value::Decimal(max_sat), None, None),
        Input::new("min lightness".to_string(), Value::Decimal(min_light), None, None),
        Input::new("max lightness".to_string(), Value::Decimal(max_light), None, None),
    ]
}

#[tokio::test]
async fn test_random_color_output_is_color() {
    let mut inputs = random_color_inputs(0.5, 1.0, 0.3, 0.7);
    let result = OpColorGenerationRandomColor::run(&mut inputs).await.unwrap();
    match &result.responses[0].value {
        Value::Color(_) => {}
        other => panic!("Expected Color, got {:?}", other),
    }
}

#[tokio::test]
async fn test_settings() {
    let s = OpColorGenerationRandomColor::settings();
    assert_eq!(s.name, "random color");
    assert_eq!(OpColorGenerationRandomColor::create_inputs().len(), 5);
    assert_eq!(OpColorGenerationRandomColor::create_outputs().len(), 1);
}
