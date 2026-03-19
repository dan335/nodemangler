use super::*;
use crate::color::Color;
use crate::input::Input;
use crate::value::Value;

#[tokio::test]
async fn test_analogous_red_30deg() {
    // Red hue ~0° with angle 30° → analogous_a ~30°, analogous_b ~330°
    let mut inputs = vec![
        Input::new("color".to_string(), Value::Color(Color::from_hsl(0.0, 1.0, 0.5, 1.0)), None, None),
        Input::new("angle".to_string(), Value::Decimal(30.0), None, None),
    ];
    let result = OpColorHarmonyAnalogous::run(&mut inputs).await.unwrap();
    assert_eq!(result.responses.len(), 2, "Expected 2 output responses");

    // Check analogous_a hue (~30°)
    let Value::Color(aa) = &result.responses[0].value else { panic!("Expected Color") };
    let (h_aa, _, _, _) = aa.to_hsl();
    assert!((h_aa - 30.0).abs() < 1.0, "Expected analogous_a hue ~30°, got {}", h_aa);

    // Check analogous_b hue (~330°)
    let Value::Color(ab) = &result.responses[1].value else { panic!("Expected Color") };
    let (h_ab, _, _, _) = ab.to_hsl();
    assert!((h_ab - 330.0).abs() < 1.0, "Expected analogous_b hue ~330°, got {}", h_ab);
}

#[tokio::test]
async fn test_settings() {
    let s = OpColorHarmonyAnalogous::settings();
    assert_eq!(s.name, "analogous");
    assert_eq!(OpColorHarmonyAnalogous::create_inputs().len(), 2);
    assert_eq!(OpColorHarmonyAnalogous::create_outputs().len(), 2);
}
