//! Tests for the shared luma weightings.

use super::*;

/// The coefficient sets are the published ones and each sums to 1, so a pure
/// grey maps to itself. A mistyped digit breaks this.
#[test]
fn coefficients_are_normalised() {
    for set in [REC709, REC601] {
        let sum: f32 = set.iter().sum();
        assert!((sum - 1.0).abs() < 1e-6, "coefficients must sum to 1, got {sum} for {set:?}");
    }
    assert_eq!(REC709, [0.2126, 0.7152, 0.0722]);
    assert_eq!(REC601, [0.299, 0.587, 0.114]);
}

#[test]
fn grey_maps_to_itself() {
    for v in [0.0f32, 0.25, 0.5, 1.0] {
        assert!((rec709(v, v, v) - v).abs() < 1e-6);
        assert!((rec601(v, v, v) - v).abs() < 1e-6);
    }
}

/// The two conventions are deliberately different — a test that asserted they
/// agreed would be asserting the bug this module exists to make visible.
#[test]
fn the_two_conventions_differ() {
    let (r, g, b) = (1.0, 0.0, 0.0);
    assert!((rec709(r, g, b) - rec601(r, g, b)).abs() > 0.05);
}

/// Fewer than three channels is already luminance: channel 0 passes through
/// unweighted rather than being treated as a red channel.
#[test]
fn low_channel_pixels_pass_channel_zero_through() {
    assert_eq!(rec709_px(&[0.4]), 0.4);
    assert_eq!(rec709_px(&[0.4, 0.9]), 0.4);
    assert_eq!(rec601_px(&[0.4]), 0.4);
    assert_eq!(rec601_px(&[0.4, 0.9]), 0.4);
    assert_eq!(rec709_px(&[]), 0.0);
    assert_eq!(rec601_px(&[]), 0.0);
}

#[test]
fn three_and_four_channel_pixels_use_the_weights() {
    let px = [0.2f32, 0.5, 0.8, 0.3];
    assert!((rec709_px(&px[..3]) - rec709(0.2, 0.5, 0.8)).abs() < 1e-7);
    assert!((rec709_px(&px) - rec709(0.2, 0.5, 0.8)).abs() < 1e-7);
    assert!((rec601_px(&px) - rec601(0.2, 0.5, 0.8)).abs() < 1e-7);
}

/// The exact expression the operations used to write out longhand, so a future
/// change to `rec709` that altered results would be caught here.
#[test]
fn matches_the_longhand_expression() {
    for (r, g, b) in [(0.1f32, 0.2f32, 0.3f32), (1.0, 0.0, 0.5), (0.7, 0.7, 0.7)] {
        assert_eq!(rec709(r, g, b), 0.2126 * r + 0.7152 * g + 0.0722 * b);
        assert_eq!(rec601(r, g, b), 0.299 * r + 0.587 * g + 0.114 * b);
    }
}
