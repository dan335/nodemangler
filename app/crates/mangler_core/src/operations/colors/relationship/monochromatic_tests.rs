use super::*;
use crate::color::Color;
use crate::input::Input;
use crate::value::Value;

fn mono_inputs(color: Color, lmin: f32, lmax: f32) -> Vec<Input> {
    vec![
        Input::new("color".to_string(), Value::Color(color), None, None),
        Input::new("lightness_min".to_string(), Value::Decimal(lmin), None, None),
        Input::new("lightness_max".to_string(), Value::Decimal(lmax), None, None),
    ]
}

#[tokio::test]
async fn test_monochromatic_hue_saturation_preserved() {
    // Hue and saturation must be identical across all five shades.
    let color = Color::from_hsl(120.0, 0.8, 0.5, 1.0);
    let mut inputs = mono_inputs(color, 0.1, 0.9);
    let result = OpColorHarmonyMonochromatic::run(&mut inputs).await.unwrap();
    assert_eq!(result.responses.len(), 5, "Expected 5 output responses");

    for (i, resp) in result.responses.iter().enumerate() {
        let Value::Color(c) = &resp.value else { panic!("Expected Color at index {}", i) };
        let (h, s, _, _) = c.to_hsl();
        assert!((h - 120.0).abs() < 1.0, "Shade {} hue should be ~120°, got {}", i + 1, h);
        assert!((s - 0.8).abs() < 0.02, "Shade {} saturation should be ~0.8, got {}", i + 1, s);
    }
}

#[tokio::test]
async fn test_monochromatic_lightness_range() {
    // shade_1 lightness ≈ lmin, shade_5 lightness ≈ lmax.
    let color = Color::from_hsl(60.0, 1.0, 0.5, 1.0);
    let mut inputs = mono_inputs(color, 0.1, 0.9);
    let result = OpColorHarmonyMonochromatic::run(&mut inputs).await.unwrap();

    let Value::Color(shade1) = &result.responses[0].value else { panic!("Expected Color") };
    let (_, _, l1, _) = shade1.to_hsl();
    assert!((l1 - 0.1).abs() < 0.02, "shade_1 lightness should be ~0.1, got {}", l1);

    let Value::Color(shade5) = &result.responses[4].value else { panic!("Expected Color") };
    let (_, _, l5, _) = shade5.to_hsl();
    assert!((l5 - 0.9).abs() < 0.02, "shade_5 lightness should be ~0.9, got {}", l5);

    // Intermediate shades should be evenly distributed.
    let Value::Color(shade3) = &result.responses[2].value else { panic!("Expected Color") };
    let (_, _, l3, _) = shade3.to_hsl();
    assert!((l3 - 0.5).abs() < 0.02, "shade_3 lightness should be ~0.5 (midpoint), got {}", l3);
}

#[tokio::test]
async fn test_settings() {
    let s = OpColorHarmonyMonochromatic::settings();
    assert_eq!(s.name, "monochromatic");
    assert_eq!(OpColorHarmonyMonochromatic::create_inputs().len(), 3);
    assert_eq!(OpColorHarmonyMonochromatic::create_outputs().len(), 5);
}
