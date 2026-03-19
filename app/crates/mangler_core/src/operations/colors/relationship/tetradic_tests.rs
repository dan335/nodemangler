use super::*;
use crate::color::Color;
use crate::input::Input;
use crate::value::Value;

#[tokio::test]
async fn test_tetradic_red() {
    // Red hue ~0° should produce tetradic_b ~90°, tetradic_c ~180°, tetradic_d ~270°.
    let mut inputs = vec![
        Input::new("color".to_string(), Value::Color(Color::from_hsl(0.0, 1.0, 0.5, 1.0)), None, None),
    ];
    let result = OpColorHarmonyTetradic::run(&mut inputs).await.unwrap();
    assert_eq!(result.responses.len(), 3, "Expected 3 output responses");

    // Check tetradic_b hue (~90°).
    let Value::Color(tb) = &result.responses[0].value else { panic!("Expected Color") };
    let (h_tb, _, _, _) = tb.to_hsl();
    assert!((h_tb - 90.0).abs() < 1.0, "Expected tetradic_b hue ~90°, got {}", h_tb);

    // Check tetradic_c hue (~180°).
    let Value::Color(tc) = &result.responses[1].value else { panic!("Expected Color") };
    let (h_tc, _, _, _) = tc.to_hsl();
    assert!((h_tc - 180.0).abs() < 1.0, "Expected tetradic_c hue ~180°, got {}", h_tc);

    // Check tetradic_d hue (~270°).
    let Value::Color(td) = &result.responses[2].value else { panic!("Expected Color") };
    let (h_td, _, _, _) = td.to_hsl();
    assert!((h_td - 270.0).abs() < 1.0, "Expected tetradic_d hue ~270°, got {}", h_td);
}

#[tokio::test]
async fn test_settings() {
    let s = OpColorHarmonyTetradic::settings();
    assert_eq!(s.name, "tetradic");
    assert_eq!(OpColorHarmonyTetradic::create_inputs().len(), 1);
    assert_eq!(OpColorHarmonyTetradic::create_outputs().len(), 3);
}
