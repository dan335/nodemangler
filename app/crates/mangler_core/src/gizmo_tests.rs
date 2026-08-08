//! Integrity gate for the spatial-gizmo table.
//!
//! A mis-declared index is invisible at runtime — the overlay just drags the
//! wrong slider — so these tests cross-check every declaration against the
//! operations' actual `create_inputs()`. Inserting or reordering an op's inputs
//! fails here rather than shipping a silently broken gizmo.

use super::*;
use crate::value::Value;

/// Every `(operation, input index, expected name)` a gizmo declares.
///
/// The index check alone would not catch a *reordering* — swapping `width` and
/// `height` keeps both indices valid — so the names are pinned too. Add a row
/// whenever a gizmo is added to [`gizmos`].
const EXPECTED_NAMES: &[(Operation, usize, &str)] = &[
    (Operation::OpImageTransformCrop, 1, "x"),
    (Operation::OpImageTransformCrop, 2, "y"),
    (Operation::OpImageTransformCrop, 3, "width"),
    (Operation::OpImageTransformCrop, 4, "height"),
    (Operation::OpColorSampleSamplePixel, 1, "x"),
    (Operation::OpColorSampleSamplePixel, 2, "y"),
    // Display-only: sample disk size painted on the crosshair, not drag-driven.
    (Operation::OpColorSampleSamplePixel, 3, "diameter"),
];

#[test]
fn every_declared_index_exists_and_is_numeric() {
    for op in Operation::all_variants() {
        let inputs = op.create_inputs();
        for spec in gizmos(&op) {
            for idx in spec.kind.referenced_inputs() {
                let input = inputs.get(idx).unwrap_or_else(|| {
                    panic!(
                        "{:?} gizmo {:?} references input {} but the op only has {}",
                        op,
                        spec.label,
                        idx,
                        inputs.len()
                    )
                });
                assert!(
                    matches!(input.value, Value::Decimal(_) | Value::Integer(_)),
                    "{:?} gizmo {:?} input {} ({}) is {:?}, not a number",
                    op,
                    spec.label,
                    idx,
                    input.name,
                    input.value
                );
            }
        }
    }
}

#[test]
fn declared_inputs_have_the_expected_names() {
    for (op, idx, expected) in EXPECTED_NAMES {
        let inputs = op.create_inputs();
        let actual = &inputs[*idx].name;
        assert_eq!(
            actual, expected,
            "{op:?} input {idx} is now {actual:?}; the gizmo table expects {expected:?}"
        );
    }
}

#[test]
fn every_gizmo_input_is_covered_by_the_name_table() {
    // Keeps the two checks from drifting apart: a new gizmo must also pin its
    // input names, or the reordering check silently stops covering it.
    for op in Operation::all_variants() {
        for spec in gizmos(&op) {
            for idx in spec.kind.referenced_inputs() {
                assert!(
                    EXPECTED_NAMES
                        .iter()
                        .any(|(o, i, _)| std::mem::discriminant(o) == std::mem::discriminant(&op)
                            && *i == idx),
                    "{:?} gizmo {:?} input {} has no row in EXPECTED_NAMES",
                    op,
                    spec.label,
                    idx
                );
            }
        }
    }
}

#[test]
fn gizmo_indices_are_unique_within_a_spec() {
    for op in Operation::all_variants() {
        for spec in gizmos(&op) {
            let mut seen = spec.kind.referenced_inputs();
            let before = seen.len();
            seen.sort_unstable();
            seen.dedup();
            assert_eq!(seen.len(), before, "{:?} gizmo {:?} repeats an input", op, spec.label);
        }
    }
}

#[test]
fn no_input_is_claimed_by_two_gizmos_on_one_op() {
    for op in Operation::all_variants() {
        let mut claimed: Vec<usize> =
            gizmos(&op).iter().flat_map(|s| s.kind.referenced_inputs()).collect();
        let before = claimed.len();
        claimed.sort_unstable();
        claimed.dedup();
        assert_eq!(claimed.len(), before, "{op:?} has two gizmos fighting over one input");
    }
}

#[test]
fn sample_pixel_diameter_is_display_only() {
    // Diameter paints the sample disk but must not join the drag/editable set —
    // wiring diameter upstream must not freeze the crosshair.
    let Some(spec) = gizmos(&Operation::OpColorSampleSamplePixel).first() else {
        panic!("sample pixel should declare a gizmo");
    };
    match spec.kind {
        Gizmo::Point { diameter, .. } => {
            assert_eq!(diameter, Some(3));
            assert_eq!(spec.kind.inputs(), vec![1, 2]);
            assert_eq!(spec.kind.referenced_inputs(), vec![1, 2, 3]);
        }
        other => panic!("sample pixel should be a Point, got {other:?}"),
    }
}

#[test]
fn crop_declares_origin_size_not_two_corner() {
    // Pins the semantic that `width` is a SIZE measured from `x`, matching
    // crop.rs's `x1 = round((x + width) * iw)`. Reading it as a far edge would
    // put the box in the right place only when x == 0.
    let Some(spec) = gizmos(&Operation::OpImageTransformCrop).first() else {
        panic!("crop should declare a gizmo");
    };
    match spec.kind {
        Gizmo::Rect { extent, space, .. } => {
            assert_eq!(extent, RectExtent::OriginSize);
            assert_eq!(space, SpatialSpace::Norm01 { basis: PixelBasis::Extent });
        }
        other => panic!("crop should be a Rect, got {other:?}"),
    }
}

#[test]
fn sample_pixel_declares_pixel_centres() {
    // Pins sample_pixel.rs's `px = x * (w - 1)`, which differs from crop's
    // `x * w` by half a pixel at the extremes.
    let Some(spec) = gizmos(&Operation::OpColorSampleSamplePixel).first() else {
        panic!("sample pixel should declare a gizmo");
    };
    match spec.kind {
        Gizmo::Point { space, .. } => {
            assert_eq!(space, SpatialSpace::Norm01 { basis: PixelBasis::Centres });
        }
        other => panic!("sample pixel should be a Point, got {other:?}"),
    }
}

#[test]
fn gizmos_never_panics_and_is_empty_for_ops_without_one() {
    let mut with_gizmos = 0;
    for op in Operation::all_variants() {
        if !gizmos(&op).is_empty() {
            with_gizmos += 1;
        }
    }
    assert_eq!(with_gizmos, 2, "update this count when a gizmo is added");
    // Spot-check a few unrelated ops across categories.
    for op in [
        Operation::OpNumberMathAdd,
        Operation::OpImageAdjustmentBlur,
        Operation::OpTextTrim,
        Operation::OpImageTransformResize,
    ] {
        assert!(gizmos(&op).is_empty(), "{op:?} should have no gizmo");
    }
}

#[test]
fn extent_basis_is_the_identity_mapping() {
    let space = SpatialSpace::Norm01 { basis: PixelBasis::Extent };
    for v in [[0.0, 0.0], [0.5, 0.25], [1.0, 1.0]] {
        assert_eq!(space.to_unit(Some((512, 256)), v), v);
        assert_eq!(space.to_unit(None, v), v);
        assert_eq!(space.from_unit(Some((512, 256)), v), v);
    }
}

#[test]
fn centres_basis_places_endpoints_at_pixel_centres() {
    // On a 4-wide image the addressable pixels are 0..=3, whose centres sit at
    // 0.125, 0.375, 0.625, 0.875 across the image.
    let space = SpatialSpace::Norm01 { basis: PixelBasis::Centres };
    let dims = Some((4, 4));
    let at = |v: f32| space.to_unit(dims, [v, v])[0];
    assert!((at(0.0) - 0.125).abs() < 1e-6, "{}", at(0.0));
    assert!((at(1.0) - 0.875).abs() < 1e-6, "{}", at(1.0));
    assert!((at(0.5) - 0.5).abs() < 1e-6, "{}", at(0.5));
}

#[test]
fn centres_basis_round_trips() {
    let space = SpatialSpace::Norm01 { basis: PixelBasis::Centres };
    // Excludes 1x1: with a single addressable pixel the mapping is genuinely
    // many-to-one, covered by `centres_basis_collapses_on_a_single_pixel`.
    for dims in [Some((2, 2)), Some((4, 4)), Some((1920, 1080)), None] {
        for v in [0.0, 0.25, 0.5, 1.0] {
            let round = space.from_unit(dims, space.to_unit(dims, [v, v]));
            assert!((round[0] - v).abs() < 1e-5, "dims {dims:?} v {v} -> {round:?}");
        }
    }
}

#[test]
fn centres_basis_collapses_on_a_single_pixel() {
    // A 1x1 image has exactly one addressable pixel, so every value maps to its
    // centre and the mapping is deliberately not invertible. Asserting the
    // collapse documents that rather than letting a round-trip test imply an
    // injectivity that cannot exist.
    let space = SpatialSpace::Norm01 { basis: PixelBasis::Centres };
    for v in [0.0, 0.25, 1.0] {
        assert_eq!(space.to_unit(Some((1, 1)), [v, v]), [0.5, 0.5]);
    }
}

#[test]
fn centres_basis_degrades_to_the_plain_fraction_without_dims() {
    // No backdrop means no half-pixel correction is knowable; the plain
    // fraction is the honest fallback and must not produce NaN.
    let space = SpatialSpace::Norm01 { basis: PixelBasis::Centres };
    assert_eq!(space.to_unit(None, [0.0, 1.0]), [0.0, 1.0]);
    assert_eq!(space.from_unit(None, [0.0, 1.0]), [0.0, 1.0]);
}

#[test]
fn centres_basis_handles_degenerate_image_sizes() {
    // A 1x1 image has no span between pixel centres; a 0-size one shouldn't
    // divide by zero. Both must stay finite.
    let space = SpatialSpace::Norm01 { basis: PixelBasis::Centres };
    for dims in [Some((1, 1)), Some((0, 0))] {
        let u = space.to_unit(dims, [0.5, 0.5]);
        let v = space.from_unit(dims, [0.5, 0.5]);
        assert!(u.iter().all(|c| c.is_finite()), "dims {dims:?} -> {u:?}");
        assert!(v.iter().all(|c| c.is_finite()), "dims {dims:?} -> {v:?}");
    }
}
