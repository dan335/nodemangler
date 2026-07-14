use super::*;

fn approx(a: f64, b: f64, tol: f64) -> bool {
    (a - b).abs() <= tol
}

// ── cumulative_arc ──────────────────────────────────────────────────────────

#[test]
fn cumulative_arc_matches_lengths() {
    let pts = [[0.0, 0.0], [3.0, 0.0], [3.0, 4.0]];
    let mut out = Vec::new();
    cumulative_arc(&pts, &mut out);
    assert_eq!(out.len(), pts.len());
    assert_eq!(out[0], 0.0);
    assert!(approx(out[1], 3.0, 1e-9));
    assert!(approx(out[2], 7.0, 1e-9)); // 3 across + 4 up
    assert!(approx(*out.last().unwrap(), polyline_length(&pts), 1e-9));
}

#[test]
fn cumulative_arc_clears_and_handles_empty() {
    let mut out = vec![1.0, 2.0, 3.0];
    cumulative_arc(&[], &mut out);
    assert!(out.is_empty());

    cumulative_arc(&[[0.5, 0.5]], &mut out);
    assert_eq!(out, vec![0.0]);
}

// ── vertex_tangent ──────────────────────────────────────────────────────────

#[test]
fn vertex_tangent_central_and_endpoints() {
    // A horizontal polyline: every tangent points +x.
    let pts = [[0.0, 0.5], [0.3, 0.5], [0.6, 0.5], [1.0, 0.5]];
    for i in 0..pts.len() {
        let t = vertex_tangent(&pts, i);
        assert!(approx(t[0], 1.0, 1e-9), "tangent {:?} at {}", t, i);
        assert!(approx(t[1], 0.0, 1e-9));
    }
}

#[test]
fn vertex_tangent_corner_is_diagonal() {
    // L-shape: the corner's central difference averages the two legs.
    let pts = [[0.0, 0.0], [1.0, 0.0], [1.0, 1.0]];
    let t = vertex_tangent(&pts, 1);
    let s = std::f64::consts::FRAC_1_SQRT_2;
    assert!(approx(t[0], s, 1e-9) && approx(t[1], s, 1e-9), "corner tangent {:?}", t);
}

#[test]
fn vertex_tangent_degenerate_fallback() {
    assert_eq!(vertex_tangent(&[], 0), [1.0, 0.0]);
    assert_eq!(vertex_tangent(&[[0.2, 0.2]], 0), [1.0, 0.0]);
    // Out-of-range index.
    assert_eq!(vertex_tangent(&[[0.0, 0.0], [1.0, 0.0]], 5), [1.0, 0.0]);
    // Coincident points → zero-length difference → fallback.
    assert_eq!(vertex_tangent(&[[0.5, 0.5], [0.5, 0.5]], 0), [1.0, 0.0]);
}

// ── linear_curve / flatten_f64 ──────────────────────────────────────────────

#[test]
fn linear_curve_builds_expected_shape() {
    let c = linear_curve(vec![[0.0, 0.0], [1.0, 1.0]], true);
    assert!(c.closed);
    assert_eq!(c.interpolation, CurveInterpolation::Linear);
    assert!(c.handles.is_empty());
    assert_eq!(c.points, vec![[0.0, 0.0], [1.0, 1.0]]);
}

#[test]
fn flatten_f64_matches_curve_flatten() {
    let c = linear_curve(vec![[0.1, 0.2], [0.9, 0.8]], false);
    let poly = flatten_f64(&c, 16);
    let expected = c.flatten(16);
    assert_eq!(poly.len(), expected.len());
    for (a, b) in poly.iter().zip(&expected) {
        assert!(approx(a[0], b[0] as f64, 1e-6));
        assert!(approx(a[1], b[1] as f64, 1e-6));
    }
}

// ── resample / rdp_decimate sanity ──────────────────────────────────────────

#[test]
fn resample_uniform_spacing_and_endpoints() {
    let pts = [[0.0, 0.0], [1.0, 0.0]]; // length 1
    let mut out = Vec::new();
    let spacing = resample(&pts, 0.1, 8000, &mut out);
    // First/last preserved exactly.
    assert_eq!(out[0], [0.0, 0.0]);
    assert_eq!(*out.last().unwrap(), [1.0, 0.0]);
    // Uniform spacing.
    for w in out.windows(2) {
        assert!(approx(dist(w[0], w[1]), spacing, 1e-9));
    }
    assert!(approx(spacing, 0.1, 1e-9));
}

#[test]
fn resample_respects_max_points() {
    let pts = [[0.0, 0.0], [1.0, 0.0]];
    let mut out = Vec::new();
    // Ask for absurdly fine spacing but cap at 10 points → spacing widens.
    resample(&pts, 1e-6, 10, &mut out);
    assert!(out.len() <= 11, "got {} points", out.len());
}

#[test]
fn rdp_decimate_drops_collinear_points() {
    // Collinear run collapses to the two endpoints.
    let pts = [[0.0, 0.0], [0.25, 0.0], [0.5, 0.0], [0.75, 0.0], [1.0, 0.0]];
    let kept = rdp_decimate(&pts, 0.01, 4000);
    assert_eq!(kept, vec![[0.0, 0.0], [1.0, 0.0]]);
}

#[test]
fn rdp_decimate_keeps_under_two() {
    let pts = [[0.3, 0.4]];
    let kept = rdp_decimate(&pts, 0.01, 4000);
    assert_eq!(kept, vec![[0.3f32, 0.4f32]]);
}

// ── drop_closing_duplicate ───────────────────────────────────────────────────

#[test]
fn drop_closing_duplicate_removes_last_when_closed() {
    let mut pts = vec![[0.0, 0.0], [1.0, 0.0], [1.0, 1.0], [0.0, 0.0]];
    drop_closing_duplicate(&mut pts, true);
    assert_eq!(pts, vec![[0.0, 0.0], [1.0, 0.0], [1.0, 1.0]]);
}

#[test]
fn drop_closing_duplicate_noop_when_open_or_short() {
    let mut pts = vec![[0.0, 0.0], [1.0, 0.0], [0.0, 0.0]];
    drop_closing_duplicate(&mut pts, false);
    assert_eq!(pts.len(), 3);

    let mut single = vec![[0.5, 0.5]];
    drop_closing_duplicate(&mut single, true);
    assert_eq!(single.len(), 1);
}

// ── laplacian_smooth_once ────────────────────────────────────────────────────

#[test]
fn laplacian_smooth_pins_open_endpoints() {
    let pts = [[0.0, 0.0], [0.5, 1.0], [1.0, 0.0]];
    let out = laplacian_smooth_once(&pts, false);
    assert_eq!(out[0], pts[0]);
    assert_eq!(out[2], pts[2]);
    // Interior point moves toward the average of its neighbors.
    assert!(approx(out[1][1], 0.5, 1e-9));
}

#[test]
fn laplacian_smooth_wraps_when_closed() {
    // A square: every vertex has two neighbors even at index 0.
    let pts = [[0.0, 0.0], [1.0, 0.0], [1.0, 1.0], [0.0, 1.0]];
    let out = laplacian_smooth_once(&pts, true);
    // Vertex 0 = (0,0); neighbors are index 3 = (0,1) and index 1 = (1,0).
    // 0.5*(0,0) + 0.25*(0,1) + 0.25*(1,0) = (0.25, 0.25).
    assert!(approx(out[0][0], 0.25, 1e-9));
    assert!(approx(out[0][1], 0.25, 1e-9));
}

#[test]
fn laplacian_smooth_short_input_is_noop() {
    let pts = [[0.1, 0.2], [0.3, 0.4]];
    let out = laplacian_smooth_once(&pts, false);
    assert_eq!(out, pts.to_vec());
}
