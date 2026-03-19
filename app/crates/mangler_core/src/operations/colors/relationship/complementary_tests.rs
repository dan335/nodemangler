use super::*;
use crate::color::Color;
use crate::input::Input;
use crate::value::Value;

#[tokio::test]
async fn test_complementary_red() {
    // Red hue ~0° should produce complementary ~180°, split_a ~150°, split_b ~210°
    let mut inputs = vec![
        Input::new("color".to_string(), Value::Color(Color::from_hsl(0.0, 1.0, 0.5, 1.0)), None, None),
    ];
    let result = OpColorHarmonyComplementary::run(&mut inputs).await.unwrap();
    assert_eq!(result.responses.len(), 3, "Expected 3 output responses");

    // Check complementary hue (~180°)
    let Value::Color(comp) = &result.responses[0].value else { panic!("Expected Color") };
    let (h_comp, _, _, _) = comp.to_hsl();
    assert!((h_comp - 180.0).abs() < 1.0, "Expected complementary hue ~180°, got {}", h_comp);

    // Check split_a hue (~150°)
    let Value::Color(sa) = &result.responses[1].value else { panic!("Expected Color") };
    let (h_sa, _, _, _) = sa.to_hsl();
    assert!((h_sa - 150.0).abs() < 1.0, "Expected split_a hue ~150°, got {}", h_sa);

    // Check split_b hue (~210°)
    let Value::Color(sb) = &result.responses[2].value else { panic!("Expected Color") };
    let (h_sb, _, _, _) = sb.to_hsl();
    assert!((h_sb - 210.0).abs() < 1.0, "Expected split_b hue ~210°, got {}", h_sb);
}

#[tokio::test]
async fn test_settings() {
    let s = OpColorHarmonyComplementary::settings();
    assert_eq!(s.name, "complementary");
}

#[tokio::test]
async fn test_output_count() {
    assert_eq!(OpColorHarmonyComplementary::create_inputs().len(), 1);
    assert_eq!(OpColorHarmonyComplementary::create_outputs().len(), 3);
}
