//! User-drawn 2D curves: free paths and closed shapes as first-class values.
//!
//! A [`Curve`] is a single subpath of control points in normalized `[0,1]²`
//! coordinates (y-down, image convention). It carries an interpolation kind
//! (straight segments, a smooth centripetal Catmull-Rom spline, or cubic
//! Bézier spans with mirrored per-anchor tangent handles) and a `closed` flag
//! distinguishing an open path from a closed loop/shape. Curves
//! flow through the node graph as [`crate::value::Value::Curve`] and can be
//! rasterized into grayscale image masks (see [`Curve::rasterize`], shared by
//! the `rasterize curve` op and the value thumbnail).

use serde::{Deserialize, Serialize};
use std::hash::{Hash, Hasher};

/// A user-drawn 2D path in normalized `[0,1]²` coordinates, y-down.
///
/// Single subpath in v1 — use multiple curve nodes for multiple paths. The
/// curve passes *through* every control point. In `Bezier` mode each anchor
/// additionally carries one mirrored tangent handle (see [`Curve::handles`]).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Curve {
    /// Control points the curve passes through, in `[0,1]²` (x, y), y-down.
    pub points: Vec<[f32; 2]>,
    /// Whether the curve is a closed loop (shape) rather than an open path.
    pub closed: bool,
    /// How the points are joined into a continuous curve.
    pub interpolation: CurveInterpolation,
    /// Per-anchor mirrored tangent offsets, used only in `Bezier` mode: the
    /// out-handle of `points[i]` sits at `points[i] + handles[i]` and the
    /// in-handle at `points[i] - handles[i]`, so the curve is C¹-smooth at
    /// every anchor by construction. Stored as offsets, so moving an anchor
    /// carries its handles along. May be shorter than `points` (older saves,
    /// or points added in another mode) — missing entries fall back to
    /// [`Curve::auto_handle`], which matches the `Smooth` tangents.
    #[serde(default)]
    pub handles: Vec<[f32; 2]>,
}

/// How a [`Curve`]'s control points are joined.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum CurveInterpolation {
    /// Straight line segments between consecutive points.
    Linear,
    /// Centripetal Catmull-Rom spline — smooth, no cusps or self-intersections.
    Smooth,
    /// Cubic Bézier spans with a mirrored, draggable tangent handle per anchor.
    Bezier,
}

impl CurveInterpolation {
    /// Returns all interpolation variants in display order (matches dropdown order).
    pub fn types() -> [CurveInterpolation; 3] {
        [
            CurveInterpolation::Linear,
            CurveInterpolation::Smooth,
            CurveInterpolation::Bezier,
        ]
    }
}

impl Default for Curve {
    fn default() -> Self {
        // A gentle, clearly visible arc so a fresh curve node draws something.
        Curve {
            points: vec![[0.15, 0.65], [0.5, 0.35], [0.85, 0.65]],
            closed: false,
            interpolation: CurveInterpolation::Smooth,
            handles: Vec::new(),
        }
    }
}

/// Samples generated per segment when flattening at standard tolerance.
/// 48 keeps a single high-curvature bezier span (2 anchors, long handles)
/// visually smooth at typical render sizes; 16 showed faceting.
const STANDARD_SAMPLES_PER_SEGMENT: usize = 48;

impl Curve {
    /// Flatten the curve into a polyline of `[0,1]²` points.
    ///
    /// `Linear` returns the control points directly (with the first re-appended
    /// when closed). `Smooth` evaluates a centripetal Catmull-Rom spline with
    /// `samples_per_segment` points per span, endpoint neighbors clamped when
    /// open and wrapped when closed. `Bezier` evaluates a cubic per span using
    /// the mirrored per-anchor handles ([`Curve::handle`]). Degenerate inputs
    /// pass through: 0 points → empty, 1 → that point, 2 → the single segment
    /// (except in `Bezier`, where handles make even a 2-point span curve).
    pub fn flatten(&self, samples_per_segment: usize) -> Vec<[f32; 2]> {
        let n = self.points.len();
        if n == 0 {
            return Vec::new();
        }
        if n == 1 {
            return vec![self.points[0]];
        }
        if n == 2 && self.interpolation != CurveInterpolation::Bezier {
            // A single span — a straight segment for Linear and Smooth.
            return vec![self.points[0], self.points[1]];
        }

        match self.interpolation {
            CurveInterpolation::Linear => {
                let mut out = self.points.clone();
                if self.closed {
                    out.push(self.points[0]);
                }
                out
            }
            CurveInterpolation::Smooth => {
                let samples = samples_per_segment.max(1);
                let mut out = Vec::new();
                let seg_count = if self.closed { n } else { n - 1 };
                for i in 0..seg_count {
                    let p1 = self.points[i];
                    let p2 = self.points[(i + 1) % n];
                    // Open endpoints use *reflected* phantom neighbors
                    // (2·end − inner). Duplicating the terminal point instead
                    // would collapse the centripetal knot spacing and drop the
                    // whole end span to the linear fallback below.
                    let p0 = if self.closed {
                        self.points[(i + n - 1) % n]
                    } else if i == 0 {
                        reflect(self.points[0], self.points[1])
                    } else {
                        self.points[i - 1]
                    };
                    let p3 = if self.closed {
                        self.points[(i + 2) % n]
                    } else if i + 2 < n {
                        self.points[i + 2]
                    } else {
                        reflect(self.points[n - 1], self.points[n - 2])
                    };
                    // Emit t in [0,1) so the shared endpoint isn't duplicated.
                    for s in 0..samples {
                        let t = s as f32 / samples as f32;
                        out.push(catmull_rom(p0, p1, p2, p3, t));
                    }
                }
                // Append the final endpoint exactly (closed → back to start).
                if self.closed {
                    out.push(self.points[0]);
                } else {
                    out.push(self.points[n - 1]);
                }
                out
            }
            CurveInterpolation::Bezier => {
                let samples = samples_per_segment.max(1);
                let mut out = Vec::new();
                let seg_count = if self.closed { n } else { n - 1 };
                for i in 0..seg_count {
                    let j = (i + 1) % n;
                    let p0 = self.points[i];
                    let p3 = self.points[j];
                    let h0 = self.handle(i);
                    let h1 = self.handle(j);
                    // Mirrored handles: out-handle of the span start, in-handle
                    // (= anchor − offset) of the span end.
                    let c1 = [p0[0] + h0[0], p0[1] + h0[1]];
                    let c2 = [p3[0] - h1[0], p3[1] - h1[1]];
                    // Emit t in [0,1) so the shared endpoint isn't duplicated.
                    for s in 0..samples {
                        let t = s as f32 / samples as f32;
                        out.push(cubic_bezier(p0, c1, c2, p3, t));
                    }
                }
                if self.closed {
                    out.push(self.points[0]);
                } else {
                    out.push(self.points[n - 1]);
                }
                out
            }
        }
    }

    /// The mirrored tangent offset for anchor `i`: the stored handle when
    /// present, otherwise [`Curve::auto_handle`]. Out of range → zero offset.
    pub fn handle(&self, i: usize) -> [f32; 2] {
        self.handles
            .get(i)
            .copied()
            .unwrap_or_else(|| self.auto_handle(i))
    }

    /// The automatic tangent offset for anchor `i`, derived from its neighbors
    /// exactly like a uniform Catmull-Rom tangent: `(next − prev) / 6`, with
    /// reflected phantom neighbors at open ends. Switching a `Smooth` curve to
    /// `Bezier` therefore keeps (approximately) its shape.
    pub fn auto_handle(&self, i: usize) -> [f32; 2] {
        let n = self.points.len();
        if n < 2 || i >= n {
            return [0.0, 0.0];
        }
        let prev = if self.closed {
            self.points[(i + n - 1) % n]
        } else if i == 0 {
            reflect(self.points[0], self.points[1])
        } else {
            self.points[i - 1]
        };
        let next = if self.closed {
            self.points[(i + 1) % n]
        } else if i + 1 < n {
            self.points[i + 1]
        } else {
            reflect(self.points[n - 1], self.points[n - 2])
        };
        [(next[0] - prev[0]) / 6.0, (next[1] - prev[1]) / 6.0]
    }

    /// Ensure `handles` has exactly one entry per point, filling any missing
    /// entries via [`Curve::handle`] (stored value where present, auto tangent
    /// otherwise). Called by the overlay editor before handle drags so a drag
    /// can write `handles[i]` directly.
    pub fn materialize_handles(&mut self) {
        if self.handles.len() != self.points.len() {
            self.handles = (0..self.points.len()).map(|i| self.handle(i)).collect();
        }
    }

    /// Approximate arc-length total of the flattened polyline in `[0,1]` units.
    pub fn length(&self) -> f32 {
        let poly = self.flatten(STANDARD_SAMPLES_PER_SEGMENT);
        let mut total = 0.0;
        for seg in poly.windows(2) {
            total += dist(seg[0], seg[1]);
        }
        total
    }

    /// Sample the curve at normalized arc-length parameter `t` in `[0,1]`.
    ///
    /// Approximates arc-length parameterization over the flattened polyline.
    /// Returns the first/last point for out-of-range or degenerate curves.
    pub fn sample(&self, t: f32) -> [f32; 2] {
        let poly = self.flatten(STANDARD_SAMPLES_PER_SEGMENT);
        if poly.is_empty() {
            return [0.0, 0.0];
        }
        if poly.len() == 1 {
            return poly[0];
        }
        let total = self.length();
        if total <= 0.0 {
            return poly[0];
        }
        let target = t.clamp(0.0, 1.0) * total;
        let mut acc = 0.0;
        for seg in poly.windows(2) {
            let seg_len = dist(seg[0], seg[1]);
            if acc + seg_len >= target {
                let local = if seg_len > 0.0 {
                    (target - acc) / seg_len
                } else {
                    0.0
                };
                return [
                    seg[0][0] + local * (seg[1][0] - seg[0][0]),
                    seg[0][1] + local * (seg[1][1] - seg[0][1]),
                ];
            }
            acc += seg_len;
        }
        *poly.last().unwrap()
    }

    /// Axis-aligned bounding box of the flattened curve as
    /// `[min_x, min_y, width, height]` in normalized `[0,1]²` units. Returns
    /// `None` when the curve has no points (a single point yields a zero-size
    /// box at that point).
    pub fn bounds(&self) -> Option<[f32; 4]> {
        let poly = self.flatten(STANDARD_SAMPLES_PER_SEGMENT);
        let mut it = poly.iter();
        let first = it.next()?;
        let (mut min_x, mut min_y) = (first[0], first[1]);
        let (mut max_x, mut max_y) = (first[0], first[1]);
        for p in it {
            min_x = min_x.min(p[0]);
            min_y = min_y.min(p[1]);
            max_x = max_x.max(p[0]);
            max_y = max_y.max(p[1]);
        }
        Some([min_x, min_y, max_x - min_x, max_y - min_y])
    }

    /// Shoelace-formula signed area of the flattened polyline, treated as
    /// *implicitly closed* (open curves are measured as if a segment joined the
    /// last point back to the first). Fewer than 3 points → 0.
    ///
    /// Sign convention: coordinates are y-down (image convention), so a
    /// **positive** area corresponds to a **clockwise** winding on screen (the
    /// opposite of the y-up mathematical convention).
    pub fn signed_area(&self) -> f32 {
        let poly = self.flatten(STANDARD_SAMPLES_PER_SEGMENT);
        let n = poly.len();
        if n < 3 {
            return 0.0;
        }
        let mut area = 0.0f32;
        for i in 0..n {
            let a = poly[i];
            let b = poly[(i + 1) % n];
            area += a[0] * b[1] - b[0] * a[1];
        }
        area * 0.5
    }

    /// Unit tangent at normalized arc-length parameter `t` in `[0,1]`, by a
    /// finite difference of [`Curve::sample`] either side of `t` (so it shares
    /// that method's arc-length parameterization). Degenerate or zero-length
    /// curves return `[1.0, 0.0]`.
    pub fn tangent_at(&self, t: f32) -> [f32; 2] {
        if self.length() <= 0.0 {
            return [1.0, 0.0];
        }
        let eps = 1e-3;
        let t = t.clamp(0.0, 1.0);
        let (t0, t1) = if t < eps {
            (t, t + eps)
        } else if t > 1.0 - eps {
            (t - eps, t)
        } else {
            (t - eps, t + eps)
        };
        let a = self.sample(t0);
        let b = self.sample(t1);
        let dx = b[0] - a[0];
        let dy = b[1] - a[1];
        let len = (dx * dx + dy * dy).sqrt();
        if len <= 1e-12 {
            [1.0, 0.0]
        } else {
            [dx / len, dy / len]
        }
    }

    /// Rasterize the curve into a 1-channel grayscale mask (`width * height`
    /// f32 values in `[0,1]`, row-major, white line on black background).
    ///
    /// `stroke_radius_px` is the stroke half-width in pixels; `feather_px`
    /// softens the edge (0 = crisp ~1px anti-aliased edge). `fill` applies only
    /// when the curve is `closed` — open paths are always stroke-only.
    ///
    /// Cost is proportional to stroke area, not image area: distances are
    /// stamped only within each segment's expanded bounding box, so large
    /// canvases stay cheap.
    pub fn rasterize(
        &self,
        width: u32,
        height: u32,
        stroke_radius_px: f32,
        feather_px: f32,
        fill: bool,
    ) -> Vec<f32> {
        use rayon::prelude::*;

        let w = width as usize;
        let h = height as usize;
        let mut out = vec![0.0f32; w * h];
        if w == 0 || h == 0 {
            return out;
        }

        // Flatten to pixel space. Fewer than 2 points → nothing to draw.
        let poly_norm = self.flatten(STANDARD_SAMPLES_PER_SEGMENT);
        if poly_norm.len() < 2 {
            return out;
        }
        let poly: Vec<[f32; 2]> = poly_norm
            .iter()
            .map(|p| [p[0] * width as f32, p[1] * height as f32])
            .collect();

        let r = stroke_radius_px.max(0.0);
        let feather = feather_px.max(0.0);
        let edge = (feather * 0.5).max(1.0);

        // Stroke pass: distance to the nearest polyline segment, stamped only
        // within each segment's padded bbox (cost ∝ stroke area).
        let mut dist_buf = vec![f32::INFINITY; w * h];
        let pad = r + feather + 2.0;
        for seg in poly.windows(2) {
            let a = seg[0];
            let b = seg[1];
            let minx = ((a[0].min(b[0]) - pad).floor() as i64).clamp(0, w as i64) as usize;
            let maxx = ((a[0].max(b[0]) + pad).ceil() as i64).clamp(0, w as i64) as usize;
            let miny = ((a[1].min(b[1]) - pad).floor() as i64).clamp(0, h as i64) as usize;
            let maxy = ((a[1].max(b[1]) + pad).ceil() as i64).clamp(0, h as i64) as usize;
            for y in miny..maxy {
                let py = y as f32 + 0.5;
                for x in minx..maxx {
                    let px = x as f32 + 0.5;
                    let d = point_segment_distance([px, py], a, b);
                    let idx = y * w + x;
                    if d < dist_buf[idx] {
                        dist_buf[idx] = d;
                    }
                }
            }
        }

        // Fill pass (closed only): even-odd scanline parity.
        let do_fill = fill && self.closed;
        let mut inside = vec![false; w * h];
        if do_fill {
            let mut xs: Vec<f32> = Vec::new();
            for y in 0..h {
                let cy = y as f32 + 0.5;
                xs.clear();
                for seg in poly.windows(2) {
                    let y0 = seg[0][1];
                    let y1 = seg[1][1];
                    // Half-open crossing test avoids double-counting shared vertices.
                    if (y0 <= cy && y1 > cy) || (y1 <= cy && y0 > cy) {
                        let t = (cy - y0) / (y1 - y0);
                        xs.push(seg[0][0] + t * (seg[1][0] - seg[0][0]));
                    }
                }
                xs.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
                let mut k = 0;
                while k + 1 < xs.len() {
                    let x_start = (xs[k].ceil() as i64).clamp(0, w as i64) as usize;
                    let x_end = (xs[k + 1].ceil() as i64).clamp(0, w as i64) as usize;
                    for x in x_start..x_end {
                        inside[y * w + x] = true;
                    }
                    k += 2;
                }
            }
        }

        // Compose (rows in parallel, per shape-op convention).
        out.par_chunks_mut(w).enumerate().for_each(|(y, row)| {
            for x in 0..w {
                let idx = y * w + x;
                let d = dist_buf[idx];
                let stroke_alpha =
                    1.0 - smoothstep((r - edge) as f64, (r + edge) as f64, d as f64) as f32;
                let mut alpha = stroke_alpha;
                if do_fill {
                    let signed = if inside[idx] { -d } else { d };
                    let fill_alpha =
                        1.0 - smoothstep((-edge) as f64, edge as f64, signed as f64) as f32;
                    alpha = alpha.max(fill_alpha);
                }
                row[x] = alpha.clamp(0.0, 1.0);
            }
        });

        out
    }

    /// Hash the curve's identity (point count, coordinate bits, closed flag,
    /// interpolation, handle bits) into `h` for cache-invalidation fingerprints.
    pub fn fingerprint_into<H: Hasher>(&self, h: &mut H) {
        self.points.len().hash(h);
        for p in &self.points {
            p[0].to_bits().hash(h);
            p[1].to_bits().hash(h);
        }
        self.closed.hash(h);
        std::mem::discriminant(&self.interpolation).hash(h);
        self.handles.len().hash(h);
        for p in &self.handles {
            p[0].to_bits().hash(h);
            p[1].to_bits().hash(h);
        }
    }
}

/// Evaluate a centripetal Catmull-Rom spline span between `p1` and `p2` at
/// `t01` in `[0,1]`, using `p0`/`p3` as the neighboring control points.
///
/// Centripetal parameterization (alpha = 0.5) avoids the cusps and loops that
/// the uniform variant produces near sharp or clustered points. Coincident
/// points collapse the local parameter spacing to zero; those spans fall back
/// to a straight interpolation.
fn catmull_rom(p0: [f32; 2], p1: [f32; 2], p2: [f32; 2], p3: [f32; 2], t01: f32) -> [f32; 2] {
    let t0 = 0.0f32;
    let t1 = t0 + dist(p0, p1).sqrt();
    let t2 = t1 + dist(p1, p2).sqrt();
    let t3 = t2 + dist(p2, p3).sqrt();

    // Degenerate spacing (coincident points) — fall back to linear p1→p2.
    if t1 <= t0 || t2 <= t1 || t3 <= t2 {
        return [
            p1[0] + t01 * (p2[0] - p1[0]),
            p1[1] + t01 * (p2[1] - p1[1]),
        ];
    }

    let t = t1 + t01 * (t2 - t1);
    let a1 = lerp2(p0, p1, (t1 - t) / (t1 - t0), (t - t0) / (t1 - t0));
    let a2 = lerp2(p1, p2, (t2 - t) / (t2 - t1), (t - t1) / (t2 - t1));
    let a3 = lerp2(p2, p3, (t3 - t) / (t3 - t2), (t - t2) / (t3 - t2));
    let b1 = lerp2(a1, a2, (t2 - t) / (t2 - t0), (t - t0) / (t2 - t0));
    let b2 = lerp2(a2, a3, (t3 - t) / (t3 - t1), (t - t1) / (t3 - t1));
    lerp2(b1, b2, (t2 - t) / (t2 - t1), (t - t1) / (t2 - t1))
}

/// Weighted sum `wa * a + wb * b` of two points.
fn lerp2(a: [f32; 2], b: [f32; 2], wa: f32, wb: f32) -> [f32; 2] {
    [wa * a[0] + wb * b[0], wa * a[1] + wb * b[1]]
}

/// Evaluate a cubic Bézier span at `t` in `[0,1]`.
fn cubic_bezier(p0: [f32; 2], c1: [f32; 2], c2: [f32; 2], p3: [f32; 2], t: f32) -> [f32; 2] {
    let u = 1.0 - t;
    let w0 = u * u * u;
    let w1 = 3.0 * u * u * t;
    let w2 = 3.0 * u * t * t;
    let w3 = t * t * t;
    [
        w0 * p0[0] + w1 * c1[0] + w2 * c2[0] + w3 * p3[0],
        w0 * p0[1] + w1 * c1[1] + w2 * c2[1] + w3 * p3[1],
    ]
}

/// Reflect `inner` about `end`: the phantom neighbor `2·end − inner` used for
/// open-curve terminal spans.
fn reflect(end: [f32; 2], inner: [f32; 2]) -> [f32; 2] {
    [2.0 * end[0] - inner[0], 2.0 * end[1] - inner[1]]
}

/// Euclidean distance between two points.
fn dist(a: [f32; 2], b: [f32; 2]) -> f32 {
    let dx = b[0] - a[0];
    let dy = b[1] - a[1];
    (dx * dx + dy * dy).sqrt()
}

/// Shortest distance from point `p` to the line segment `a`–`b`.
fn point_segment_distance(p: [f32; 2], a: [f32; 2], b: [f32; 2]) -> f32 {
    let dx = b[0] - a[0];
    let dy = b[1] - a[1];
    let len_sq = dx * dx + dy * dy;
    if len_sq < 1e-12 {
        return dist(p, a);
    }
    let t = (((p[0] - a[0]) * dx + (p[1] - a[1]) * dy) / len_sq).clamp(0.0, 1.0);
    let cx = a[0] + t * dx;
    let cy = a[1] + t * dy;
    let ex = p[0] - cx;
    let ey = p[1] - cy;
    (ex * ex + ey * ey).sqrt()
}

/// Hermite smoothstep between two edges (matches the shape ops' local helper).
fn smoothstep(edge0: f64, edge1: f64, x: f64) -> f64 {
    if edge0 == edge1 {
        return if x < edge0 { 0.0 } else { 1.0 };
    }
    let t = ((x - edge0) / (edge1 - edge0)).clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

#[cfg(test)]
#[path = "curve_tests.rs"]
mod tests;
