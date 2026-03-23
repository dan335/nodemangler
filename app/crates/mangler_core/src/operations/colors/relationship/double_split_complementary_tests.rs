use super::*;
use crate::color::Color;
use crate::input::Input;
use crate::value::Value;

#[tokio::test]
async fn test_double_split_complementary_red() {
    // Red hue ~0° → split_base_a ~30°, split_base_b ~330°, split_comp_a ~150°, split_comp_b ~210°.
    let mut inputs = vec![
        Input::new("color".to_string(), Value::Color(Color::from_hsl(0.0, 1.0, 0.5, 1.0)), None, None),
    ];
    let result = OpColorHarmonyDoubleSplitComplementary::run(&mut inputs).await.unwrap();
    assert_eq!(result.responses.len(), 4, "Expected 4 output responses");

    // Check split_base_a hue (~30°).
    let Value::Color(sba) = &result.responses[0].value else { panic!("Expected Color") };
    let (h_sba, _, _, _) = sba.to_hsl();
    assert!((h_sba - 30.0).abs() < 1.0, "Expected split_base_a hue ~30°, got {}", h_sba);

    // Check split_base_b hue (~330°).
    let Value::Color(sbb) = &result.responses[1].value else { panic!("Expected Color") };
    let (h_sbb, _, _, _) = sbb.to_hsl();
    assert!((h_sbb - 330.0).abs() < 1.0, "Expected split_base_b hue ~330°, got {}", h_sbb);

    // Check split_comp_a hue (~150°).
    let Value::Color(sca) = &result.responses[2].value else { panic!("Expected Color") };
    let (h_sca, _, _, _) = sca.to_hsl();
    assert!((h_sca - 150.0).abs() < 1.0, "Expected split_comp_a hue ~150°, got {}", h_sca);

    // Check split_comp_b hue (~210°).
    let Value::Color(scb) = &result.responses[3].value else { panic!("Expected Color") };
    let (h_scb, _, _, _) = scb.to_hsl();
    assert!((h_scb - 210.0).abs() < 1.0, "Expected split_comp_b hue ~210°, got {}", h_scb);
}

#[tokio::test]
async fn test_settings() {
    let s = OpColorHarmonyDoubleSplitComplementary::settings();
    assert_eq!(s.name, "double split comp");
    assert_eq!(OpColorHarmonyDoubleSplitComplementary::create_inputs().len(), 1);
    assert_eq!(OpColorHarmonyDoubleSplitComplementary::create_outputs().len(), 4);
}
