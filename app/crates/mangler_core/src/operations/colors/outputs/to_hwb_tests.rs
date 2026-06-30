use super::*;
use crate::color::Color;
use crate::input::Input;
use crate::value::Value;

fn color_input(r: f32, g: f32, b: f32, a: f32) -> Vec<Input> {
    vec![Input::new("input".to_string(), Value::Color(Color::from_srgb_float(r, g, b, a)), None, None)]
}

#[tokio::test]
async fn settings_and_ports() {
    assert_eq!(OpColorOutputHwb::settings().name, "to hwb");
    assert_eq!(OpColorOutputHwb::create_inputs().len(), 1);
    assert_eq!(OpColorOutputHwb::create_outputs().len(), 4);
}

#[tokio::test]
async fn white_is_full_whiteness() {
    let mut inputs = color_input(1.0, 1.0, 1.0, 1.0);
    let result = OpColorOutputHwb::run(&mut inputs).await.unwrap();
    // whiteness = 1, blackness = 0 for white
    match (&result.responses[1].value, &result.responses[2].value) {
        (Value::Decimal(w), Value::Decimal(b)) => {
            assert!((*w - 1.0).abs() < 1e-4, "whiteness should be 1, got {}", w);
            assert!(b.abs() < 1e-4, "blackness should be 0, got {}", b);
        }
        other => panic!("Expected Decimals, got {:?}", other),
    }
}
