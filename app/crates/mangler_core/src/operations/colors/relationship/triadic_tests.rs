use super::*;
use crate::color::Color;
use crate::input::Input;
use crate::value::Value;

#[tokio::test]
async fn test_triadic_red() {
    // Red hue ~0° should produce triadic_a ~120° and triadic_b ~240°
    let mut inputs = vec![
        Input::new("color".to_string(), Value::Color(Color::from_hsl(0.0, 1.0, 0.5, 1.0)), None, None),
    ];
    let result = OpColorHarmonyTriadic::run(&mut inputs).await.unwrap();
    assert_eq!(result.responses.len(), 2, "Expected 2 output responses");

    // Check triadic_a hue (~120°)
    let Value::Color(ta) = &result.responses[0].value else { panic!("Expected Color") };
    let (h_ta, _, _, _) = ta.to_hsl();
    assert!((h_ta - 120.0).abs() < 1.0, "Expected triadic_a hue ~120°, got {}", h_ta);

    // Check triadic_b hue (~240°)
    let Value::Color(tb) = &result.responses[1].value else { panic!("Expected Color") };
    let (h_tb, _, _, _) = tb.to_hsl();
    assert!((h_tb - 240.0).abs() < 1.0, "Expected triadic_b hue ~240°, got {}", h_tb);
}

#[tokio::test]
async fn test_settings() {
    let s = OpColorHarmonyTriadic::settings();
    assert_eq!(s.name, "triadic");
    assert_eq!(OpColorHarmonyTriadic::create_inputs().len(), 1);
    assert_eq!(OpColorHarmonyTriadic::create_outputs().len(), 2);
}
