//! Shared geometry helpers for the curve operations.
//!
//! Polyline math in `f64` (distance, arc length, resampling, decimation,
//! tangents) plus small [`Curve`] construction/flattening conveniences, factored
//! out of the meander simulation so the generator/modifier/analysis nodes can
//! reuse the exact same primitives. Everything works on flattened polylines in
//! normalized `[0,1]²` coordinates (y-down).

use crate::curve::{Curve, CurveInterpolation};

/// Cap on points in a Curve *output* value. Output curves persist as JSON in
/// every graph save, so the modifier/generator nodes decimate to this before
/// emitting to keep saves small.
pub(crate) const MAX_OUTPUT_POINTS: usize = 4000;

/// Euclidean distance between two points.
pub(crate) fn dist(a: [f64; 2], b: [f64; 2]) -> f64 {
    let dx = b[0] - a[0];
    let dy = b[1] - a[1];
    (dx * dx + dy * dy).sqrt()
}

/// Total arc length of a polyline.
pub(crate) fn polyline_length(points: &[[f64; 2]]) -> f64 {
    points.windows(2).map(|s| dist(s[0], s[1])).sum()
}

/// Shortest distance from point `p` to the segment `a`-`b` (f64 twin of the
/// curve rasterizer's helper).
pub(crate) fn point_segment_distance(p: [f64; 2], a: [f64; 2], b: [f64; 2]) -> f64 {
    let dx = b[0] - a[0];
    let dy = b[1] - a[1];
    let len_sq = dx * dx + dy * dy;
    if len_sq < 1e-24 {
        return dist(p, a);
    }
    let t = (((p[0] - a[0]) * dx + (p[1] - a[1]) * dy) / len_sq).clamp(0.0, 1.0);
    dist(p, [a[0] + t * dx, a[1] + t * dy])
}

/// Resamples a polyline to uniform spacing as close to `ds` as divides the
/// total length evenly (so index * spacing = arc position exactly), into the
/// reusable `out` buffer (callers running this every iteration swap-buffer it
/// against the live points so steady-state iterations allocate nothing).
/// The first and last input points are preserved exactly. Spacing widens
/// automatically when `ds` would exceed `max_points`. Returns the actual
/// spacing used.
pub(crate) fn resample(points: &[[f64; 2]], ds: f64, max_points: usize, out: &mut Vec<[f64; 2]>) -> f64 {
    out.clear();
    let total = polyline_length(points);
    if points.len() < 2 || total <= 0.0 {
        out.extend_from_slice(points);
        return ds;
    }
    let ds = ds.max(total / max_points as f64);
    let n_seg = (total / ds).round().max(1.0) as usize;
    let spacing = total / n_seg as f64;

    out.reserve(n_seg + 1);
    out.push(points[0]);
    let mut target = spacing;
    let mut acc = 0.0;
    for seg in points.windows(2) {
        let len = dist(seg[0], seg[1]);
        if len <= 0.0 {
            continue;
        }
        while target <= acc + len && out.len() < n_seg {
            let t = (target - acc) / len;
            out.push([
                seg[0][0] + t * (seg[1][0] - seg[0][0]),
                seg[0][1] + t * (seg[1][1] - seg[0][1]),
            ]);
            target += spacing;
        }
        acc += len;
    }
    out.push(*points.last().unwrap());
    spacing
}

/// Ramer-Douglas-Peucker decimation (iterative stack), doubling the tolerance
/// until the result fits `max_points`. Keeps a persisted output Curve small —
/// its points serialize into every graph save.
pub(crate) fn rdp_decimate(points: &[[f64; 2]], mut tol: f64, max_points: usize) -> Vec<[f32; 2]> {
    let n = points.len();
    if n <= 2 {
        return points.iter().map(|p| [p[0] as f32, p[1] as f32]).collect();
    }
    loop {
        let mut keep = vec![false; n];
        keep[0] = true;
        keep[n - 1] = true;
        let mut stack = vec![(0usize, n - 1)];
        while let Some((s, e)) = stack.pop() {
            if e <= s + 1 {
                continue;
            }
            let mut d_max = 0.0;
            let mut i_max = s;
            for i in s + 1..e {
                let d = point_segment_distance(points[i], points[s], points[e]);
                if d > d_max {
                    d_max = d;
                    i_max = i;
                }
            }
            if d_max > tol {
                keep[i_max] = true;
                stack.push((s, i_max));
                stack.push((i_max, e));
            }
        }
        let kept: Vec<[f32; 2]> = points
            .iter()
            .zip(&keep)
            .filter(|(_, &k)| k)
            .map(|(p, _)| [p[0] as f32, p[1] as f32])
            .collect();
        if kept.len() <= max_points {
            return kept;
        }
        tol *= 2.0;
    }
}

/// Flatten a curve into an `f64` polyline in normalized `[0,1]²` coordinates
/// (the `f64` twin of [`Curve::flatten`], the working precision for the curve
/// geometry ops).
// Consumed by the generator/modifier nodes landing in later phases.
#[allow(dead_code)]
pub(crate) fn flatten_f64(curve: &Curve, samples_per_segment: usize) -> Vec<[f64; 2]> {
    curve
        .flatten(samples_per_segment)
        .iter()
        .map(|p| [p[0] as f64, p[1] as f64])
        .collect()
}

/// Build an open/closed `Linear` curve from a set of control points — the
/// output shape most geometry ops emit after flattening to a polyline.
pub(crate) fn linear_curve(points: Vec<[f32; 2]>, closed: bool) -> Curve {
    Curve {
        points,
        closed,
        interpolation: CurveInterpolation::Linear,
        handles: Vec::new(),
    }
}

/// Fills `out` with the cumulative arc length at each vertex: `out[0] == 0.0`,
/// `out[i] == out[i-1] + dist(points[i-1], points[i])`, so `out.len() ==
/// points.len()` and the last entry is the total polyline length. Clears `out`
/// first.
// Consumed by the modifier/analysis nodes landing in later phases.
#[allow(dead_code)]
pub(crate) fn cumulative_arc(points: &[[f64; 2]], out: &mut Vec<f64>) {
    out.clear();
    out.reserve(points.len());
    let mut acc = 0.0;
    for (i, &p) in points.iter().enumerate() {
        if i > 0 {
            acc += dist(points[i - 1], p);
        }
        out.push(acc);
    }
}

/// Unit tangent at vertex `i` of a polyline via a central difference
/// (`points[i+1] - points[i-1]`), falling back to a forward/backward difference
/// at the endpoints. Degenerate (fewer than 2 points, out-of-range index, or a
/// zero-length difference) returns `[1.0, 0.0]`.
// Consumed by the modifier nodes (jitter, offset) landing in later phases.
#[allow(dead_code)]
pub(crate) fn vertex_tangent(points: &[[f64; 2]], i: usize) -> [f64; 2] {
    let n = points.len();
    if n < 2 || i >= n {
        return [1.0, 0.0];
    }
    let (a, b) = if i == 0 {
        (points[0], points[1])
    } else if i == n - 1 {
        (points[n - 2], points[n - 1])
    } else {
        (points[i - 1], points[i + 1])
    };
    let dx = b[0] - a[0];
    let dy = b[1] - a[1];
    let len = (dx * dx + dy * dy).sqrt();
    if len <= 1e-12 {
        [1.0, 0.0]
    } else {
        [dx / len, dy / len]
    }
}

#[cfg(test)]
#[path = "common_tests.rs"]
mod tests;
