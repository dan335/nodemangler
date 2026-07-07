//! Tests for the pure light-direction math and default settings.

use super::*;
use std::f32::consts::{FRAC_PI_2, PI};

/// Elevation π/2 must point straight up (+Y), independent of azimuth.
#[test]
fn elevation_up_is_plus_y() {
    for azimuth in [0.0, 1.0, PI, -2.0] {
        let dir = light_direction(azimuth, FRAC_PI_2);
        assert!((dir.x).abs() < 1e-5, "x should be ~0, got {}", dir.x);
        assert!((dir.y - 1.0).abs() < 1e-5, "y should be ~1, got {}", dir.y);
        assert!((dir.z).abs() < 1e-5, "z should be ~0, got {}", dir.z);
    }
}

/// Elevation 0 must lie in the horizontal plane (y ≈ 0).
#[test]
fn elevation_zero_is_horizontal() {
    for azimuth in [0.0, 0.5, PI, -1.3] {
        let dir = light_direction(azimuth, 0.0);
        assert!((dir.y).abs() < 1e-5, "y should be ~0, got {}", dir.y);
    }
}

/// The returned direction is always a unit vector.
#[test]
fn direction_is_unit_length() {
    for &(az, el) in &[
        (0.0, 0.0),
        (1.0, 0.5),
        (PI, FRAC_PI_2),
        (-2.0, -0.3),
        (3.0, 1.2),
    ] {
        let dir = light_direction(az, el);
        assert!(
            (dir.length() - 1.0).abs() < 1e-5,
            "length should be 1, got {}",
            dir.length()
        );
    }
}

/// Default settings must reproduce the old hard-coded light direction
/// `Vec3::new(0.8, 1.0, 0.6).normalize()` within epsilon.
#[test]
fn default_reproduces_legacy_direction() {
    let settings = Viewer3dSettings::default();
    let dir = light_direction(settings.light_azimuth, settings.light_elevation);
    let expected = glam::Vec3::new(0.8, 1.0, 0.6).normalize();
    assert!(
        (dir - expected).length() < 1e-4,
        "expected {:?}, got {:?}",
        expected,
        dir
    );
}

/// Default settings match the previously hard-coded white light / 45° FOV.
#[test]
fn default_values_match_legacy_behavior() {
    let settings = Viewer3dSettings::default();
    assert_eq!(settings.light_color, [1.0, 1.0, 1.0]);
    assert_eq!(settings.light_intensity, 3.0);
    assert_eq!(settings.fov_y_degrees, 45.0);
}

/// Phase 4 defaults: one texture copy, ACES tone map, wireframe off.
#[test]
fn phase4_defaults() {
    let settings = Viewer3dSettings::default();
    assert_eq!(settings.uv_tiling, 1.0);
    assert_eq!(settings.tone_map, ToneMap::Aces);
    assert!(!settings.wireframe);
}

/// Directional shadows default on (the expected lit-preview look).
#[test]
fn shadows_default_on() {
    let settings = Viewer3dSettings::default();
    assert!(settings.shadows, "shadows should default to true");
}

/// ToneMap ALL lists every variant once with distinct labels, and to_int matches
/// the shader branch numbering (0=None, 1=Reinhard, 2=ACES, 3=Filmic).
#[test]
fn tone_map_all_labels_and_ints() {
    assert_eq!(ToneMap::ALL.len(), 4);
    assert_eq!(ToneMap::None.label(), "None");
    assert_eq!(ToneMap::Reinhard.label(), "Reinhard");
    assert_eq!(ToneMap::Aces.label(), "ACES");
    assert_eq!(ToneMap::Filmic.label(), "Filmic");

    assert_eq!(ToneMap::None.to_int(), 0);
    assert_eq!(ToneMap::Reinhard.to_int(), 1);
    assert_eq!(ToneMap::Aces.to_int(), 2);
    assert_eq!(ToneMap::Filmic.to_int(), 3);

    // Default is ACES.
    assert_eq!(ToneMap::default(), ToneMap::Aces);
}
