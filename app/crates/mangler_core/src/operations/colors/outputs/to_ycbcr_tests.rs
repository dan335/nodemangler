use super::*;
use crate::color::Color;
use crate::input::Input;
use crate::value::Value;

fn color_input(r: f32, g: f32, b: f32, a: f32) -> Vec<Input> {
    vec![Input::new("input".to_string(), Value::Color(Color::from_srgb_float(r, g, b, a)), None, None)]
}

#[tokio::test]
async fn settings_and_ports() {
    assert_eq!(OpColorOutputYcbcr::settings().name, "to ycbcr");
    assert_eq!(OpColorOutputYcbcr::create_inputs().len(), 1);
    assert_eq!(OpColorOutputYcbcr::create_outputs().len(), 4);
}

#[tokio::test]
async fn gray_is_neutral_chroma() {
    let mut inputs = color_input(0.5, 0.5, 0.5, 1.0);
    let result = OpColorOutputYcbcr::run(&mut inputs).await.unwrap();
    // Cb and Cr of a neutral gray should be ~0
    match (&result.responses[1].value, &result.responses[2].value) {
        (Value::Decimal(cb), Value::Decimal(cr)) => {
            assert!(cb.abs() < 1e-4, "Cb should be ~0, got {}", cb);
            assert!(cr.abs() < 1e-4, "Cr should be ~0, got {}", cr);
        }
        other => panic!("Expected Decimals, got {:?}", other),
    }
}
