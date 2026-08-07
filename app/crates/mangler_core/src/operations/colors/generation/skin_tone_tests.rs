use super::*;
use crate::input::Input;
use crate::value::Value;

fn skin_tone_inputs(
    random: bool,
    seed: i32,
    r_square: f32,
    t: f32,
    u: f32,
    v: f32,
    alpha: f32,
) -> Vec<Input> {
    vec![
        Input::new("random".to_string(), Value::Bool(random), None, None),
        Input::new("seed".to_string(), Value::Integer(seed), None, None),
        Input::new("r²".to_string(), Value::Decimal(r_square), None, None),
        Input::new("deep/fair".to_string(), Value::Decimal(t), None, None),
        Input::new("flushed/ochre".to_string(), Value::Decimal(u), None, None),
        Input::new("cool/warm".to_string(), Value::Decimal(v), None, None),
        Input::new("alpha".to_string(), Value::Decimal(alpha), None, None),
    ]
}

#[test]
fn test_settings() {
    let s = OpColorGenerationSkinTone::settings();
    assert_eq!(s.name, "skin tone");
    assert_eq!(OpColorGenerationSkinTone::create_inputs().len(), 7);
    assert_eq!(OpColorGenerationSkinTone::create_outputs().len(), 4);
}

#[test]
fn test_origin_is_neutral_mid_tone() {
    // R² = 0 and TUV = 0 both map to the PCA-space origin — a neutral
    // ambiguous mid tone used by the space as a bias check.
    let (r0, g0, b0) = tuv_to_srgb(0.0, 0.0, 0.0);
    assert!((0.2..0.9).contains(&r0), "r={r0}");
    assert!((0.2..0.9).contains(&g0), "g={g0}");
    assert!((0.2..0.9).contains(&b0), "b={b0}");
    // Skin tones lean warm: red channel should dominate blue at the origin.
    assert!(r0 > b0, "origin should be warmer than pure grey (r={r0}, b={b0})");
    assert!(g0 > b0, "origin should have g > b (g={g0}, b={b0})");
}

#[test]
fn test_sample_sphere_at_zero_radius_is_origin() {
    let (t, u, v) = sample_sphere(42, 0.0);
    assert_eq!((t, u, v), (0.0, 0.0, 0.0));
}

#[test]
fn test_sample_sphere_stays_inside_radius() {
    let r_square = 2.0_f32;
    let radius = r_square.sqrt();
    for seed in 0..200 {
        let (t, u, v) = sample_sphere(seed, r_square);
        let r = (t * t + u * u + v * v).sqrt();
        assert!(
            r <= radius + 1e-5,
            "seed {seed}: point ({t},{u},{v}) has r={r} > radius={radius}"
        );
    }
}

#[test]
fn test_sample_sphere_is_deterministic() {
    let a = sample_sphere(7, 2.0);
    let b = sample_sphere(7, 2.0);
    assert_eq!(a, b);
    let c = sample_sphere(8, 2.0);
    assert_ne!(a, c);
}

#[test]
fn test_tuv_to_srgb_channels_in_unit_interval() {
    // Sweep a grid covering and slightly beyond the R²=2.5 picker range.
    for ti in -5..=5 {
        for ui in -5..=5 {
            for vi in -5..=5 {
                let t = ti as f32 * 0.5;
                let u = ui as f32 * 0.5;
                let v = vi as f32 * 0.5;
                let (r, g, b) = tuv_to_srgb(t, u, v);
                assert!((0.0..=1.0).contains(&r), "r={r} at ({t},{u},{v})");
                assert!((0.0..=1.0).contains(&g), "g={g} at ({t},{u},{v})");
                assert!((0.0..=1.0).contains(&b), "b={b} at ({t},{u},{v})");
            }
        }
    }
}

#[tokio::test]
async fn test_random_mode_outputs_color_and_coords() {
    let mut inputs = skin_tone_inputs(true, 1, 2.0, 0.0, 0.0, 0.0, 1.0);
    let result = OpColorGenerationSkinTone::run(&mut inputs).await.unwrap();
    assert_eq!(result.responses.len(), 4);
    match &result.responses[0].value {
        Value::Color(c) => {
            let (r, g, b, a) = c.to_srgb_float();
            assert!((0.0..=1.0).contains(&r));
            assert!((0.0..=1.0).contains(&g));
            assert!((0.0..=1.0).contains(&b));
            assert!((a - 1.0).abs() < 1e-5);
        }
        other => panic!("Expected Color, got {other:?}"),
    }
    // Coords should match a fresh sample with the same seed.
    let (et, eu, ev) = sample_sphere(1, 2.0);
    let Value::Decimal(t) = &result.responses[1].value else { panic!("t") };
    let Value::Decimal(u) = &result.responses[2].value else { panic!("u") };
    let Value::Decimal(v) = &result.responses[3].value else { panic!("v") };
    assert!((t - et).abs() < 1e-6);
    assert!((u - eu).abs() < 1e-6);
    assert!((v - ev).abs() < 1e-6);
}

#[tokio::test]
async fn test_manual_mode_uses_tuv_sliders() {
    let mut inputs = skin_tone_inputs(false, 99, 2.0, 0.5, -0.3, 0.2, 0.75);
    let result = OpColorGenerationSkinTone::run(&mut inputs).await.unwrap();

    let Value::Decimal(t) = &result.responses[1].value else { panic!("t") };
    let Value::Decimal(u) = &result.responses[2].value else { panic!("u") };
    let Value::Decimal(v) = &result.responses[3].value else { panic!("v") };
    assert!((t - 0.5).abs() < 1e-6);
    assert!((u - -0.3).abs() < 1e-6);
    assert!((v - 0.2).abs() < 1e-6);

    let (er, eg, eb) = tuv_to_srgb(0.5, -0.3, 0.2);
    let Value::Color(c) = &result.responses[0].value else { panic!("color") };
    let (r, g, b, a) = c.to_srgb_float();
    assert!((r - er).abs() < 1e-5);
    assert!((g - eg).abs() < 1e-5);
    assert!((b - eb).abs() < 1e-5);
    assert!((a - 0.75).abs() < 1e-5);
}

#[tokio::test]
async fn test_random_mode_is_seed_deterministic() {
    let mut a = skin_tone_inputs(true, 123, 1.5, 0.0, 0.0, 0.0, 1.0);
    let mut b = skin_tone_inputs(true, 123, 1.5, 0.0, 0.0, 0.0, 1.0);
    let mut c = skin_tone_inputs(true, 124, 1.5, 0.0, 0.0, 0.0, 1.0);

    let ra = OpColorGenerationSkinTone::run(&mut a).await.unwrap();
    let rb = OpColorGenerationSkinTone::run(&mut b).await.unwrap();
    let rc = OpColorGenerationSkinTone::run(&mut c).await.unwrap();

    let Value::Color(ca) = &ra.responses[0].value else { panic!() };
    let Value::Color(cb) = &rb.responses[0].value else { panic!() };
    let Value::Color(cc) = &rc.responses[0].value else { panic!() };
    assert_eq!(ca.to_srgb_float(), cb.to_srgb_float());
    assert_ne!(ca.to_srgb_float(), cc.to_srgb_float());
}

#[tokio::test]
async fn test_zero_r_square_yields_origin_tone() {
    let mut inputs = skin_tone_inputs(true, 999, 0.0, 1.0, 1.0, 1.0, 1.0);
    let result = OpColorGenerationSkinTone::run(&mut inputs).await.unwrap();
    let (er, eg, eb) = tuv_to_srgb(0.0, 0.0, 0.0);
    let Value::Color(c) = &result.responses[0].value else { panic!() };
    let (r, g, b, _) = c.to_srgb_float();
    assert!((r - er).abs() < 1e-5);
    assert!((g - eg).abs() < 1e-5);
    assert!((b - eb).abs() < 1e-5);
}
