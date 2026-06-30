use super::*;
use crate::color::Color;
use crate::input::Input;
use crate::value::Value;

fn color_input(r: f32, g: f32, b: f32, a: f32) -> Vec<Input> {
    vec![Input::new("input".to_string(), Value::Color(Color::from_srgb_float(r, g, b, a)), None, None)]
}

#[tokio::test]
async fn settings_and_ports() {
    assert_eq!(OpColorOutputXyy::settings().name, "to xyy");
    assert_eq!(OpColorOutputXyy::create_inputs().len(), 1);
    assert_eq!(OpColorOutputXyy::create_outputs().len(), 4);
}

#[tokio::test]
async fn white_is_d65_chromaticity() {
    let mut inputs = color_input(1.0, 1.0, 1.0, 1.0);
    let result = OpColorOutputXyy::run(&mut inputs).await.unwrap();
    // white chromaticity ~ D65 (0.3127, 0.3290)
    match (&result.responses[0].value, &result.responses[1].value) {
        (Value::Decimal(x), Value::Decimal(y)) => {
            assert!((*x - 0.3127).abs() < 2e-3, "x should be ~0.3127, got {}", x);
            assert!((*y - 0.3290).abs() < 2e-3, "y should be ~0.3290, got {}", y);
        }
        other => panic!("Expected Decimals, got {:?}", other),
    }
}
