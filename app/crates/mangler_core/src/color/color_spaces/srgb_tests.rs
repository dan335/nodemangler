use super::*;

#[test]
fn test_srgb_float_roundtrip() {
    let color = Color::from_srgb_float(0.75, 0.5, 0.25, 1.0);
    let (r, g, b, a) = color.to_srgb_float();
    let color2 = Color::from_srgb_float(r, g, b, a);
    assert_eq!(color, color2);
}

#[test]
fn test_srgb_u8_roundtrip() {
    let color = Color::from_srgb_u8(200, 128, 64, 255);
    let (r, g, b, a) = color.to_srgb_u8();
    assert_eq!((r, g, b, a), (200, 128, 64, 255));
}

#[test]
fn test_srgb_u8_black() {
    let color = Color::from_srgb_u8(0, 0, 0, 255);
    let (r, g, b, a) = color.to_srgb_u8();
    assert_eq!((r, g, b, a), (0, 0, 0, 255));
}

#[test]
fn test_srgb_u8_white() {
    let color = Color::from_srgb_u8(255, 255, 255, 255);
    let (r, g, b, a) = color.to_srgb_u8();
    assert_eq!((r, g, b, a), (255, 255, 255, 255));
}

#[test]
fn test_srgb_float_clamp() {
    let color = Color::from_srgb_float(1.5, -0.5, 0.5, 2.0);
    let (r, g, b, a) = color.to_srgb_float();
    assert_eq!(r, 1.0);
    assert_eq!(g, 0.0);
    assert_eq!(b, 0.5);
    assert_eq!(a, 1.0);
}

#[test]
fn test_default_is_black() {
    let color = Color::default();
    assert_eq!(color.r, 0.0);
    assert_eq!(color.g, 0.0);
    assert_eq!(color.b, 0.0);
    assert_eq!(color.a, 1.0);
}
