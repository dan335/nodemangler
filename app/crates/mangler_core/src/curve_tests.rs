use super::*;

fn approx(a: f32, b: f32, tol: f32) -> bool {
    (a - b).abs() <= tol
}

// ── flatten: linear ───────────────────────────────────────────────────────

#[test]
fn flatten_linear_open_returns_points() {
    let c = Curve {
        points: vec![[0.0, 0.0], [0.5, 0.5], [1.0, 0.0]],
        closed: false,
        interpolation: CurveInterpolation::Linear,
        handles: vec![],
    };
    let poly = c.flatten(16);
    assert_eq!(poly.len(), 3);
    assert_eq!(poly[0], [0.0, 0.0]);
    assert_eq!(poly[2], [1.0, 0.0]);
}

#[test]
fn flatten_linear_closed_wraps_first_point() {
    let c = Curve {
        points: vec![[0.0, 0.0], [1.0, 0.0], [0.5, 1.0]],
        closed: true,
        interpolation: CurveInterpolation::Linear,
        handles: vec![],
    };
    let poly = c.flatten(16);
    assert_eq!(poly.len(), 4);
    assert_eq!(*poly.last().unwrap(), [0.0, 0.0]);
}

// ── flatten: smooth ───────────────────────────────────────────────────────

#[test]
fn flatten_smooth_open_exact_endpoints() {
    let c = Curve {
        points: vec![[0.15, 0.65], [0.5, 0.35], [0.85, 0.65]],
        closed: false,
        interpolation: CurveInterpolation::Smooth,
        handles: vec![],
    };
    let poly = c.flatten(16);
    // seg_count (2) * samples (16) + final endpoint.
    assert_eq!(poly.len(), 2 * 16 + 1);
    assert_eq!(poly[0], [0.15, 0.65]);
    assert_eq!(*poly.last().unwrap(), [0.85, 0.65]);
}

#[test]
fn flatten_smooth_open_end_spans_actually_curve() {
    // Regression: clamped (duplicated) endpoint neighbors collapsed the
    // centripetal knot spacing, dropping *end* spans to straight lines — a
    // default 3-point curve is nothing but end spans, so "smooth" rendered
    // linear. With reflected phantom neighbors the first span's midpoint must
    // deviate from the straight chord between points 0 and 1.
    let c = Curve::default();
    let poly = c.flatten(16);
    let a = c.points[0];
    let b = c.points[1];
    let mid = poly[8]; // halfway through the first span's 16 samples
    let chord_mid = [(a[0] + b[0]) * 0.5, (a[1] + b[1]) * 0.5];
    let deviation =
        ((mid[0] - chord_mid[0]).powi(2) + (mid[1] - chord_mid[1]).powi(2)).sqrt();
    assert!(
        deviation > 0.005,
        "first span midpoint {:?} sits on the straight chord (deviation {})",
        mid,
        deviation
    );
}

#[test]
fn flatten_smooth_closed_wraps() {
    let c = Curve {
        points: vec![[0.2, 0.2], [0.8, 0.2], [0.5, 0.8]],
        closed: true,
        interpolation: CurveInterpolation::Smooth,
        handles: vec![],
    };
    let poly = c.flatten(16);
    // seg_count (3) * samples (16) + closing point back to start.
    assert_eq!(poly.len(), 3 * 16 + 1);
    assert_eq!(poly[0], [0.2, 0.2]);
    assert_eq!(*poly.last().unwrap(), [0.2, 0.2]);
}

// ── flatten: bezier ───────────────────────────────────────────────────────

#[test]
fn flatten_bezier_counts_and_endpoints() {
    let c = Curve {
        points: vec![[0.15, 0.65], [0.5, 0.35], [0.85, 0.65]],
        closed: false,
        interpolation: CurveInterpolation::Bezier,
        handles: vec![[0.1, 0.0], [0.1, 0.0], [0.1, 0.0]],
    };
    let poly = c.flatten(16);
    assert_eq!(poly.len(), 2 * 16 + 1);
    assert_eq!(poly[0], [0.15, 0.65]);
    assert_eq!(*poly.last().unwrap(), [0.85, 0.65]);
}

#[test]
fn flatten_bezier_handle_bends_the_span() {
    // A 2-point bezier with a perpendicular handle must bow away from the
    // chord (unlike Linear/Smooth, where 2 points are always a straight line).
    let straight = Curve {
        points: vec![[0.1, 0.5], [0.9, 0.5]],
        closed: false,
        interpolation: CurveInterpolation::Bezier,
        handles: vec![[0.0, 0.0], [0.0, 0.0]],
    };
    let bent = Curve {
        handles: vec![[0.0, -0.3], [0.0, 0.3]],
        ..straight.clone()
    };
    let mid_straight = straight.flatten(16)[8];
    let mid_bent = bent.flatten(16)[8];
    assert!(approx(mid_straight[1], 0.5, 1e-4), "zero handles should stay on the chord");
    assert!(
        mid_bent[1] < 0.4,
        "perpendicular handles should bow the span; midpoint y was {}",
        mid_bent[1]
    );
}

#[test]
fn flatten_bezier_mirrored_handles_are_tangent_continuous() {
    // The polyline direction entering an interior anchor must match the
    // direction leaving it — mirrored handles make anchors C¹ by construction.
    let c = Curve {
        points: vec![[0.1, 0.8], [0.5, 0.2], [0.9, 0.8]],
        closed: false,
        interpolation: CurveInterpolation::Bezier,
        handles: vec![[0.05, 0.0], [0.2, 0.1], [0.05, 0.0]],
    };
    // Fine sampling: the check compares secants, whose deviation from the true
    // (exactly continuous) tangent shrinks linearly with the step size.
    let poly = c.flatten(512);
    let k = 512; // index of the interior anchor in the flattened polyline
    let incoming = [poly[k][0] - poly[k - 1][0], poly[k][1] - poly[k - 1][1]];
    let outgoing = [poly[k + 1][0] - poly[k][0], poly[k + 1][1] - poly[k][1]];
    let cross = incoming[0] * outgoing[1] - incoming[1] * outgoing[0];
    let dot = incoming[0] * outgoing[0] + incoming[1] * outgoing[1];
    assert!(dot > 0.0, "directions should agree across the anchor");
    let mag = (incoming[0].powi(2) + incoming[1].powi(2)).sqrt()
        * (outgoing[0].powi(2) + outgoing[1].powi(2)).sqrt();
    assert!(
        (cross / mag).abs() < 0.05,
        "tangent should be continuous across the anchor; normalized cross was {}",
        cross / mag
    );
}

#[test]
fn flatten_bezier_missing_handles_fall_back_to_auto() {
    // With no stored handles, bezier uses auto tangents and still curves.
    let c = Curve {
        points: vec![[0.15, 0.65], [0.5, 0.35], [0.85, 0.65]],
        closed: false,
        interpolation: CurveInterpolation::Bezier,
        handles: vec![],
    };
    let poly = c.flatten(16);
    let a = c.points[0];
    let b = c.points[1];
    let mid = poly[8];
    let chord_mid = [(a[0] + b[0]) * 0.5, (a[1] + b[1]) * 0.5];
    let deviation =
        ((mid[0] - chord_mid[0]).powi(2) + (mid[1] - chord_mid[1]).powi(2)).sqrt();
    assert!(deviation > 0.005, "auto-handle bezier should curve; deviation {}", deviation);
}

#[test]
fn flatten_bezier_closed_wraps() {
    let c = Curve {
        points: vec![[0.2, 0.2], [0.8, 0.2], [0.5, 0.8]],
        closed: true,
        interpolation: CurveInterpolation::Bezier,
        handles: vec![],
    };
    let poly = c.flatten(16);
    assert_eq!(poly.len(), 3 * 16 + 1);
    assert_eq!(poly[0], [0.2, 0.2]);
    assert_eq!(*poly.last().unwrap(), [0.2, 0.2]);
}

// ── handles ───────────────────────────────────────────────────────────────

#[test]
fn materialize_handles_fills_and_preserves() {
    let mut c = Curve::default();
    c.interpolation = CurveInterpolation::Bezier;
    assert!(c.handles.is_empty());
    c.materialize_handles();
    assert_eq!(c.handles.len(), c.points.len());
    // Interior auto tangent: (next − prev) / 6.
    let expected = [
        (c.points[2][0] - c.points[0][0]) / 6.0,
        (c.points[2][1] - c.points[0][1]) / 6.0,
    ];
    assert!(approx(c.handles[1][0], expected[0], 1e-6));
    assert!(approx(c.handles[1][1], expected[1], 1e-6));

    // Already-aligned handles are left untouched.
    c.handles[1] = [0.42, -0.13];
    c.materialize_handles();
    assert_eq!(c.handles[1], [0.42, -0.13]);
}

#[test]
fn serde_missing_handles_field_defaults_empty() {
    // Curves saved before handles existed must still parse.
    let json = r#"{"points":[[0.1,0.2],[0.8,0.9]],"closed":false,"interpolation":"Smooth"}"#;
    let c: Curve = serde_json::from_str(json).unwrap();
    assert!(c.handles.is_empty());
    assert_eq!(c.points.len(), 2);
}

// ── flatten: degenerates ──────────────────────────────────────────────────

#[test]
fn flatten_degenerate_zero_points() {
    let c = Curve { points: vec![], closed: false, interpolation: CurveInterpolation::Smooth, handles: vec![] };
    assert!(c.flatten(16).is_empty());
}

#[test]
fn flatten_degenerate_one_point() {
    let c = Curve { points: vec![[0.3, 0.4]], closed: false, interpolation: CurveInterpolation::Smooth, handles: vec![] };
    assert_eq!(c.flatten(16), vec![[0.3, 0.4]]);
}

#[test]
fn flatten_degenerate_two_points() {
    let c = Curve {
        points: vec![[0.1, 0.2], [0.9, 0.8]],
        closed: true,
        interpolation: CurveInterpolation::Smooth,
        handles: vec![],
    };
    assert_eq!(c.flatten(16), vec![[0.1, 0.2], [0.9, 0.8]]);
}

// ── sample / length ───────────────────────────────────────────────────────

#[test]
fn sample_l_shape() {
    // An L: down one unit, then right one unit. Total arc length 2.
    let c = Curve {
        points: vec![[0.0, 0.0], [0.0, 1.0], [1.0, 1.0]],
        closed: false,
        interpolation: CurveInterpolation::Linear,
        handles: vec![],
    };
    let s0 = c.sample(0.0);
    let s_mid = c.sample(0.5);
    let s1 = c.sample(1.0);
    assert!(approx(s0[0], 0.0, 1e-4) && approx(s0[1], 0.0, 1e-4));
    // Halfway along arc length (1.0 of 2.0) is the corner.
    assert!(approx(s_mid[0], 0.0, 1e-3) && approx(s_mid[1], 1.0, 1e-3));
    assert!(approx(s1[0], 1.0, 1e-4) && approx(s1[1], 1.0, 1e-4));
}

#[test]
fn length_of_unit_square() {
    let c = Curve {
        points: vec![[0.0, 0.0], [1.0, 0.0], [1.0, 1.0], [0.0, 1.0]],
        closed: true,
        interpolation: CurveInterpolation::Linear,
        handles: vec![],
    };
    assert!(approx(c.length(), 4.0, 1e-4));
}

// ── bounds / signed_area / tangent_at ───────────────────────────────────────

#[test]
fn bounds_of_known_square() {
    let c = Curve {
        points: vec![[0.2, 0.3], [0.6, 0.3], [0.6, 0.8], [0.2, 0.8]],
        closed: true,
        interpolation: CurveInterpolation::Linear,
        handles: vec![],
    };
    let b = c.bounds().unwrap();
    assert!(approx(b[0], 0.2, 1e-6), "min_x {}", b[0]);
    assert!(approx(b[1], 0.3, 1e-6), "min_y {}", b[1]);
    assert!(approx(b[2], 0.4, 1e-6), "width {}", b[2]);
    assert!(approx(b[3], 0.5, 1e-6), "height {}", b[3]);
}

#[test]
fn bounds_empty_is_none() {
    let c = Curve { points: vec![], closed: false, interpolation: CurveInterpolation::Linear, handles: vec![] };
    assert!(c.bounds().is_none());
}

#[test]
fn signed_area_clockwise_on_screen_is_positive() {
    // A 0.25×0.25 square wound clockwise on screen (y-down): top-left →
    // top-right → bottom-right → bottom-left. Positive area per the docs.
    let cw = Curve {
        points: vec![[0.0, 0.0], [0.25, 0.0], [0.25, 0.25], [0.0, 0.25]],
        closed: true,
        interpolation: CurveInterpolation::Linear,
        handles: vec![],
    };
    assert!(approx(cw.signed_area(), 0.0625, 1e-5), "cw area {}", cw.signed_area());

    // Reversed winding (counter-clockwise on screen) flips the sign.
    let ccw = Curve {
        points: vec![[0.0, 0.0], [0.0, 0.25], [0.25, 0.25], [0.25, 0.0]],
        ..cw.clone()
    };
    assert!(approx(ccw.signed_area(), -0.0625, 1e-5), "ccw area {}", ccw.signed_area());
}

#[test]
fn signed_area_open_curve_implicitly_closes() {
    // The same square, but marked open — implicit close gives the same area.
    let open = Curve {
        points: vec![[0.0, 0.0], [0.25, 0.0], [0.25, 0.25], [0.0, 0.25]],
        closed: false,
        interpolation: CurveInterpolation::Linear,
        handles: vec![],
    };
    assert!(approx(open.signed_area(), 0.0625, 1e-5), "open area {}", open.signed_area());
}

#[test]
fn tangent_at_horizontal_line() {
    let c = Curve {
        points: vec![[0.1, 0.5], [0.9, 0.5]],
        closed: false,
        interpolation: CurveInterpolation::Linear,
        handles: vec![],
    };
    let t = c.tangent_at(0.5);
    assert!(approx(t[0], 1.0, 1e-4) && approx(t[1], 0.0, 1e-4), "tangent {:?}", t);
}

#[test]
fn tangent_at_degenerate_fallback() {
    let empty = Curve { points: vec![], closed: false, interpolation: CurveInterpolation::Linear, handles: vec![] };
    assert_eq!(empty.tangent_at(0.5), [1.0, 0.0]);

    let single = Curve { points: vec![[0.4, 0.4]], closed: false, interpolation: CurveInterpolation::Linear, handles: vec![] };
    assert_eq!(single.tangent_at(0.5), [1.0, 0.0]);
}

// ── rasterize ─────────────────────────────────────────────────────────────

fn at(gray: &[f32], w: usize, x: usize, y: usize) -> f32 {
    gray[y * w + x]
}

#[test]
fn rasterize_horizontal_line_on_and_off() {
    // 1024px so the op's scale_to_resolution would be identity; here we call
    // rasterize directly with a pixel radius.
    let w = 1024usize;
    let h = 1024usize;
    let c = Curve {
        points: vec![[0.1, 0.5], [0.9, 0.5]],
        closed: false,
        interpolation: CurveInterpolation::Linear,
        handles: vec![],
    };
    let r = 8.0;
    let gray = c.rasterize(w as u32, h as u32, r, 0.0, false);
    // On the line (row ~512, mid x): fully covered.
    let on = at(&gray, w, 512, 512);
    assert!(on > 0.95, "on-line alpha was {}", on);
    // Well beyond r + 2px perpendicular: fully off.
    let off = at(&gray, w, 512, 512 + (r as usize) + 6);
    assert!(off < 1e-4, "off-line alpha was {}", off);
}

#[test]
fn rasterize_closed_triangle_fill() {
    let w = 512usize;
    let h = 512usize;
    let c = Curve {
        points: vec![[0.3, 0.3], [0.7, 0.3], [0.5, 0.75]],
        closed: true,
        interpolation: CurveInterpolation::Linear,
        handles: vec![],
    };
    let gray = c.rasterize(w as u32, h as u32, 2.0, 0.0, true);
    // Interior point (roughly the centroid) is filled.
    let inside = at(&gray, w, 256, 230);
    assert!(inside > 0.95, "interior alpha was {}", inside);
    // A corner well outside the triangle is empty.
    let outside = at(&gray, w, 20, 20);
    assert!(outside < 1e-4, "exterior alpha was {}", outside);
}

#[test]
fn rasterize_fill_false_is_outline_only() {
    let w = 512usize;
    let h = 512usize;
    let c = Curve {
        points: vec![[0.3, 0.3], [0.7, 0.3], [0.5, 0.75]],
        closed: true,
        interpolation: CurveInterpolation::Linear,
        handles: vec![],
    };
    let gray = c.rasterize(w as u32, h as u32, 2.0, 0.0, false);
    // Interior is empty when fill is off (only the thin outline is drawn).
    let inside = at(&gray, w, 256, 230);
    assert!(inside < 1e-4, "interior alpha with fill=false was {}", inside);
}

#[test]
fn rasterize_feather_widens_ramp_monotonically() {
    let w = 1024usize;
    let h = 1024usize;
    let c = Curve {
        points: vec![[0.1, 0.5], [0.9, 0.5]],
        closed: false,
        interpolation: CurveInterpolation::Linear,
        handles: vec![],
    };
    let r = 6.0f32;
    // A pixel just beyond the stroke radius: alpha should grow with feather.
    let sample_row = 512 + r as usize + 3;
    let a0 = at(&c.rasterize(w as u32, h as u32, r, 0.0, false), w, 512, sample_row);
    let a8 = at(&c.rasterize(w as u32, h as u32, r, 8.0, false), w, 512, sample_row);
    let a20 = at(&c.rasterize(w as u32, h as u32, r, 20.0, false), w, 512, sample_row);
    assert!(a0 < a8, "feather 0 ({}) should be < feather 8 ({})", a0, a8);
    assert!(a8 < a20, "feather 8 ({}) should be < feather 20 ({})", a8, a20);
}

// ── fingerprint ───────────────────────────────────────────────────────────

fn fp(c: &Curve) -> u64 {
    use std::collections::hash_map::DefaultHasher;
    use std::hash::Hasher;
    let mut h = DefaultHasher::new();
    c.fingerprint_into(&mut h);
    h.finish()
}

#[test]
fn fingerprint_changes_and_is_stable() {
    let c = Curve::default();
    assert_eq!(fp(&c), fp(&c.clone()));

    let mut moved = c.clone();
    moved.points[0][0] += 0.1;
    assert_ne!(fp(&c), fp(&moved));

    let mut flipped = c.clone();
    flipped.closed = !flipped.closed;
    assert_ne!(fp(&c), fp(&flipped));

    let mut interp = c.clone();
    interp.interpolation = CurveInterpolation::Linear;
    assert_ne!(fp(&c), fp(&interp));

    let mut handled = c.clone();
    handled.materialize_handles();
    handled.handles[0] = [0.2, -0.1];
    assert_ne!(fp(&c), fp(&handled));
}
