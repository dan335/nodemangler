use super::*;

#[test]
fn test_identity_curve_is_identity() {
    assert!(is_identity(&identity_tone_curve()));
    assert!(optional_lut(&identity_tone_curve()).is_none());
}

#[test]
fn test_moved_curve_is_not_identity() {
    let mut curve = identity_tone_curve();
    curve.points[0] = [0.0, 0.5];
    assert!(!is_identity(&curve));
    assert!(optional_lut(&curve).is_some());
}

#[test]
fn test_anti_diagonal_is_exact_descending_ramp() {
    let lut = tone_curve_lut(&anti_diagonal_tone_curve(), TONE_LUT_SIZE);
    for (i, v) in lut.iter().enumerate() {
        let expected = 1.0 - i as f32 / (TONE_LUT_SIZE - 1) as f32;
        assert!(
            (v - expected).abs() < 1e-5,
            "bin {i}: got {v}, expected {expected}"
        );
    }
}

#[test]
fn test_identity_lut_endpoints_exact() {
    let lut = tone_curve_lut(&identity_tone_curve(), TONE_LUT_SIZE);
    assert_eq!(lut[0], 0.0);
    assert_eq!(lut[TONE_LUT_SIZE - 1], 1.0);
}

#[test]
fn test_tone_curve_input_builder() {
    let input = tone_curve_input("falloff", "test description");
    assert_eq!(input.name, "falloff");
    assert!(matches!(input.settings, Some(InputSettings::ToneCurve)));
    let Value::Curve(curve) = &input.default_value else {
        panic!("expected a curve default");
    };
    assert!(is_identity(curve));
}

#[test]
fn test_sample_lut_interpolates() {
    let lut = vec![0.0, 1.0];
    assert!((sample_lut(&lut, 0.5) - 0.5).abs() < 1e-6);
    assert_eq!(sample_lut(&lut, -1.0), 0.0);
    assert_eq!(sample_lut(&lut, 2.0), 1.0);
}
