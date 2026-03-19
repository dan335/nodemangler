use super::*;
use crate::color::Color;
use crate::input::Input;
use crate::value::Value;

fn pair_inputs(a: Color, b: Color) -> Vec<Input> {
    vec![
        Input::new("a".to_string(), Value::Color(a), None, None),
        Input::new("b".to_string(), Value::Color(b), None, None),
    ]
}

#[tokio::test]
async fn test_same_hue_monochromatic_peak() {
    // Two colors with the same hue → 0° delta → monochromatic peak ~0.9.
    let color_a = Color::from_hsl(120.0, 0.8, 0.3, 1.0);
    let color_b = Color::from_hsl(120.0, 0.6, 0.7, 1.0);
    let mut inputs = pair_inputs(color_a, color_b);
    let result = OpColorAnalysisHarmonyScore::run(&mut inputs).await.unwrap();

    let Value::Decimal(score) = result.responses[0].value else { panic!("Expected Decimal") };
    assert!(
        (score - 0.9).abs() < 0.05,
        "Same hue should score near monochromatic peak 0.9, got {}",
        score
    );
}

#[tokio::test]
async fn test_complementary_180_peak() {
    // Colors 180° apart → complementary peak ~0.95.
    let color_a = Color::from_hsl(0.0, 1.0, 0.5, 1.0);
    let color_b = Color::from_hsl(180.0, 1.0, 0.5, 1.0);
    let mut inputs = pair_inputs(color_a, color_b);
    let result = OpColorAnalysisHarmonyScore::run(&mut inputs).await.unwrap();

    let Value::Decimal(score) = result.responses[0].value else { panic!("Expected Decimal") };
    assert!(
        (score - 0.95).abs() < 0.05,
        "180° apart should score near complementary peak 0.95, got {}",
        score
    );
}

#[tokio::test]
async fn test_90_degrees_low_score() {
    // Colors ~90° apart sit between peaks and should have a noticeably lower score.
    let color_a = Color::from_hsl(0.0, 1.0, 0.5, 1.0);
    let color_b = Color::from_hsl(90.0, 1.0, 0.5, 1.0);
    let mut inputs = pair_inputs(color_a, color_b);
    let result = OpColorAnalysisHarmonyScore::run(&mut inputs).await.unwrap();

    let Value::Decimal(score) = result.responses[0].value else { panic!("Expected Decimal") };
    // 90° is equidistant from triadic (120°) and analogous (30°); score should be relatively low.
    assert!(score < 0.5, "90° apart should have a low harmony score, got {}", score);
}

#[tokio::test]
async fn test_score_clamped_0_to_1() {
    // Score must always be within 0–1.
    for (h_a, h_b) in [(0.0, 0.0), (0.0, 45.0), (0.0, 90.0), (0.0, 135.0), (0.0, 180.0)] {
        let ca = Color::from_hsl(h_a, 1.0, 0.5, 1.0);
        let cb = Color::from_hsl(h_b, 1.0, 0.5, 1.0);
        let mut inputs = pair_inputs(ca, cb);
        let result = OpColorAnalysisHarmonyScore::run(&mut inputs).await.unwrap();
        let Value::Decimal(score) = result.responses[0].value else { panic!("Expected Decimal") };
        assert!(score >= 0.0 && score <= 1.0, "Score out of range: {}", score);
    }
}

#[tokio::test]
async fn test_settings() {
    let s = OpColorAnalysisHarmonyScore::settings();
    assert_eq!(s.name, "harmony score");
    assert_eq!(OpColorAnalysisHarmonyScore::create_inputs().len(), 2);
    assert_eq!(OpColorAnalysisHarmonyScore::create_outputs().len(), 1);
}
