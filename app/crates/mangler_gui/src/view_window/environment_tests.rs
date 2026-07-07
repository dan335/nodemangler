//! Tests for the procedural sky and the CPU-prefiltered IBL LUT builders.

use super::*;

/// Component-wise minimum of the three sky colours — a lower bound for any
/// integral/average of `sky_radiance`.
fn sky_min() -> Vec3 {
    ZENITH.min(HORIZON).min(GROUND)
}

/// Component-wise maximum of the three sky colours — an upper bound for any
/// integral/average of `sky_radiance`.
fn sky_max() -> Vec3 {
    ZENITH.max(HORIZON).max(GROUND)
}

/// Looking straight up must return the zenith colour.
#[test]
fn sky_up_is_zenith() {
    let c = sky_radiance(Vec3::Y);
    assert!((c - ZENITH).length() < 1e-5, "expected {:?}, got {:?}", ZENITH, c);
}

/// Looking straight down must return the ground colour.
#[test]
fn sky_down_is_ground() {
    let c = sky_radiance(Vec3::NEG_Y);
    assert!((c - GROUND).length() < 1e-5, "expected {:?}, got {:?}", GROUND, c);
}

/// A horizontal direction must return the horizon colour, from either side.
#[test]
fn sky_horizontal_is_horizon() {
    for dir in [Vec3::X, Vec3::NEG_X, Vec3::Z, Vec3::NEG_Z] {
        let c = sky_radiance(dir);
        assert!(
            (c - HORIZON).length() < 1e-5,
            "expected {:?}, got {:?} for {:?}",
            HORIZON,
            c,
            dir
        );
    }
}

/// The gradient must be continuous across the horizon: sweeping the direction's
/// y through small steps produces proportionally small colour deltas (no jump
/// where the above/below-horizon branches meet).
#[test]
fn sky_continuous_across_horizon() {
    let steps = 400;
    let mut prev: Option<Vec3> = None;
    for i in 0..=steps {
        // y sweeps -0.2 .. 0.2 through the horizon band.
        let y = -0.2 + 0.4 * i as f32 / steps as f32;
        let x = (1.0 - y * y).max(0.0).sqrt();
        let c = sky_radiance(Vec3::new(x, y, 0.0));
        if let Some(p) = prev {
            // With a step of 0.001 in y, any per-step delta above this bound
            // would indicate a discontinuity rather than a smooth gradient.
            assert!(
                (c - p).length() < 0.01,
                "colour jumped by {} at y={}",
                (c - p).length(),
                y
            );
        }
        prev = Some(c);
    }
}

/// The irradiance LUT has the documented size and every value is finite,
/// non-negative, and bounded by the min/max of the three sky colours (a
/// cosine-weighted average of sky colours cannot exceed its inputs).
#[test]
fn irradiance_lut_bounded() {
    let lut = build_irradiance_lut();
    assert_eq!(lut.len(), IRRADIANCE_SIZE * 4);

    let lo = sky_min();
    let hi = sky_max();
    for texel in lut.chunks_exact(4) {
        let c = Vec3::new(texel[0], texel[1], texel[2]);
        assert!(c.is_finite(), "non-finite irradiance texel {:?}", c);
        for (v, lo, hi) in [(c.x, lo.x, hi.x), (c.y, lo.y, hi.y), (c.z, lo.z, hi.z)] {
            assert!(v >= 0.0, "negative irradiance {}", v);
            // Small epsilon for Monte-Carlo/floating-point slack.
            assert!(
                v >= lo - 1e-4 && v <= hi + 1e-4,
                "irradiance {} outside sky bounds [{}, {}]",
                v,
                lo,
                hi
            );
        }
        assert_eq!(texel[3], 1.0, "alpha must be 1.0");
    }
}

/// The specular LUT has the documented size and every value is finite,
/// non-negative, and bounded by the sky colour range.
#[test]
fn specular_lut_bounded_and_finite() {
    let lut = build_specular_lut();
    assert_eq!(lut.len(), SPECULAR_SIZE * SPECULAR_SIZE * 4);

    let lo = sky_min();
    let hi = sky_max();
    for texel in lut.chunks_exact(4) {
        let c = Vec3::new(texel[0], texel[1], texel[2]);
        assert!(c.is_finite(), "non-finite specular texel {:?}", c);
        for (v, lo, hi) in [(c.x, lo.x, hi.x), (c.y, lo.y, hi.y), (c.z, lo.z, hi.z)] {
            assert!(v >= 0.0, "negative specular value {}", v);
            assert!(
                v >= lo - 1e-4 && v <= hi + 1e-4,
                "specular {} outside sky bounds [{}, {}]",
                v,
                lo,
                hi
            );
        }
        assert_eq!(texel[3], 1.0, "alpha must be 1.0");
    }
}

/// The lowest-roughness row of the specular LUT is mirror-like: each texel must
/// closely match `sky_radiance` evaluated at that texel's reflection direction
/// (at roughness 0 the GGX sampler collapses every sample onto R).
#[test]
fn specular_lut_roughness_zero_is_mirror() {
    let lut = build_specular_lut();
    for col in 0..SPECULAR_SIZE {
        // Same texel-centre → axis mapping the builder uses.
        let ry = (col as f32 + 0.5) / SPECULAR_SIZE as f32 * 2.0 - 1.0;
        let r = Vec3::new((1.0 - ry * ry).max(0.0).sqrt(), ry, 0.0).normalize();
        let expected = sky_radiance(r);
        // Row 0 = roughness 0.
        let base = col * 4;
        let got = Vec3::new(lut[base], lut[base + 1], lut[base + 2]);
        assert!(
            (got - expected).length() < 1e-3,
            "mirror row texel {} = {:?}, expected {:?}",
            col,
            got,
            expected
        );
    }
}

/// Irradiance around an upward normal must be cooler/bluer (closer to zenith)
/// than around a downward normal (closer to ground) — a sanity check that the
/// hemisphere orientation is not flipped.
#[test]
fn irradiance_up_bluer_than_down() {
    let lut = build_irradiance_lut();
    let first = &lut[0..4]; // N.y ≈ -1 (downward normal)
    let last = &lut[(IRRADIANCE_SIZE - 1) * 4..IRRADIANCE_SIZE * 4]; // N.y ≈ +1
    // The zenith-facing texel must have more blue than the ground-facing one.
    assert!(
        last[2] > first[2],
        "upward blue {} should exceed downward blue {}",
        last[2],
        first[2]
    );
    // And the ground-facing texel must be darker overall.
    let sum_down = first[0] + first[1] + first[2];
    let sum_up = last[0] + last[1] + last[2];
    assert!(
        sum_up > sum_down,
        "upward irradiance {} should exceed downward {}",
        sum_up,
        sum_down
    );
}

/// One-time LUT build must stay well under the plan's ~100 ms budget; assert a
/// generous 1 s ceiling as a smoke test and print the measured duration.
#[test]
fn specular_lut_build_time_smoke() {
    let start = std::time::Instant::now();
    let lut = build_specular_lut();
    let elapsed = start.elapsed();
    println!("build_specular_lut: {:?} ({} texels)", elapsed, lut.len() / 4);
    assert!(
        elapsed < std::time::Duration::from_secs(1),
        "specular LUT build took {:?}, expected < 1s",
        elapsed
    );
}
