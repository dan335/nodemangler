//! Meander node: curvature-driven river evolution.
//!
//! Evolves a user-drawn curve (the initial river centerline) with the Howard &
//! Knutson (1984) bank-migration model — the same physics as Sylvester's
//! meanderpy: water flows faster on the outside of a bend, so each centerline
//! point migrates along its normal at a rate set by an upstream-weighted
//! average of signed curvature. Bends grow, translate downstream, skew, and
//! when a neck pinches shut the loop is cut off and left behind as an oxbow
//! lake.
//!
//! The simulation runs entirely in normalized [0,1]² curve space (y-down), so
//! the evolved curve is resolution-independent; only the raster outputs use
//! the width/height inputs. Everything is deterministic from the seed: the
//! only randomness is the initial perturbation that breaks the symmetry of a
//! straight line (zero curvature never evolves), and the iteration loop is
//! strictly serial.
//!
//! Stability (this is a growth instability): curvature is tanh-saturated at
//! one over the channel width — a bend can't be tighter than the channel is
//! wide — per-step displacement is clamped to half the sample spacing, and the
//! centerline is resampled to uniform spacing with a 1-2-1 curvature smoothing
//! pass every iteration.

use crate::curve::{Curve, CurveInterpolation};
use crate::float_image::FloatImage;
use crate::get_id;
use crate::input::{Input, InputSettings};
use crate::node_settings::NodeSettings;
use crate::operations::images::simulation::{guidance_map_to_grid, is_unconnected};
use crate::operations::{
    convert_input, default_image, OperationError, OperationResponse, OutputResponse,
};
use crate::output::Output;
use crate::value::{Value, ValueType};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Instant;

/// Local-curvature coefficient of the Howard-Knutson migration rate. Negative:
/// the local term alone is stabilizing; the lagged upstream term drives growth.
const OMEGA: f64 = -1.0;
/// Upstream-convolution coefficient. OMEGA + GAMMA > 0 gives net outward bend
/// growth with a downstream phase lag (translation + skew). meanderpy's values.
const GAMMA: f64 = 2.5;
/// Hard cap on live centerline points; the resample spacing widens to hold it.
const MAX_POINTS: usize = 8000;
/// Cap on points in the output Curve value (persisted as JSON in graph saves).
const MAX_OUTPUT_POINTS: usize = 4000;
/// Cap on stored oxbow loops; later cutoffs still shorten the channel but are
/// no longer rendered (their trace survives in the migration map).
const MAX_OXBOWS: usize = 200;

/// Everything derived once from the node params, in normalized [0,1] units.
///
/// The channel width varies along the river (discharge grows downstream), so
/// most physical scales are *local*: the per-point `widths` array (rebuilt
/// after every resample by [`widths_along`]) is threaded through curvature,
/// lag, migration, and cutoff detection. What lives here is the width-
/// independent coefficients.
struct SimParams {
    /// Upstream-lag length in channel widths (local L = lag * width).
    lag: f64,
    /// Migration-rate coefficient; local step = rate_coeff * width.
    /// The 0.2 calibrates the default rate to grow developed meanders in
    /// ~150-250 iterations (set empirically from the render sweep; the
    /// linear-stability estimate under-predicts the discrete gain).
    rate_coeff: f64,
    /// Per-step displacement clamp (half the sample spacing).
    max_disp: f64,
    /// Downstream (maximum) channel width in normalized units.
    w_norm: f64,
    /// Channel width at the upstream end, as a fraction of `w_norm`.
    upstream_frac: f64,
    /// Bank-heterogeneity noise coefficient; local per-iteration noise
    /// amplitude = noise_coeff * width. Howard-Knutson is a convective
    /// instability — bends grow while translating downstream, so without
    /// continuous re-seeding the pinned upstream end relaminarizes into a
    /// straight dead zone that a one-time perturbation can't fix.
    noise_coeff: f64,
    /// Neck-cutoff trigger distance in channel widths (local = cutoff * width).
    cutoff: f64,
    /// Arc length over which migration tapers to zero at each pinned endpoint.
    taper_arc: f64,
}

/// Fills `widths` with the local channel width per point: sqrt growth from
/// `upstream_frac * w_norm` at the source to `w_norm` at the mouth (width
/// scales with the square root of discharge, and discharge grows roughly
/// linearly downstream). Uniform spacing makes index fraction = arc fraction.
fn widths_along(n: usize, p: &SimParams, widths: &mut Vec<f64>) {
    widths.clear();
    widths.reserve(n);
    let denom = (n.max(2) - 1) as f64;
    for i in 0..n {
        let t = i as f64 / denom;
        widths.push(p.w_norm * (p.upstream_frac + (1.0 - p.upstream_frac) * t.sqrt()));
    }
}

/// A connected erodibility guidance map, sampled bilinearly at the map's own
/// resolution so the simulation stays independent of the raster output size.
struct ErodGrid {
    data: Vec<f64>,
    w: usize,
    h: usize,
}

impl ErodGrid {
    /// Bilinear sample at a normalized [0,1]² position, clamped at the edges.
    fn sample(&self, p: [f64; 2]) -> f64 {
        let x = p[0].clamp(0.0, 1.0) * (self.w - 1) as f64;
        let y = p[1].clamp(0.0, 1.0) * (self.h - 1) as f64;
        let x0 = x.floor() as usize;
        let y0 = y.floor() as usize;
        let x1 = (x0 + 1).min(self.w - 1);
        let y1 = (y0 + 1).min(self.h - 1);
        let fx = x - x0 as f64;
        let fy = y - y0 as f64;
        let a = self.data[y0 * self.w + x0];
        let b = self.data[y0 * self.w + x1];
        let c = self.data[y1 * self.w + x0];
        let d = self.data[y1 * self.w + x1];
        (a * (1.0 - fx) + b * fx) * (1.0 - fy) + (c * (1.0 - fx) + d * fx) * fy
    }
}

/// Euclidean distance between two points.
fn dist(a: [f64; 2], b: [f64; 2]) -> f64 {
    let dx = b[0] - a[0];
    let dy = b[1] - a[1];
    (dx * dx + dy * dy).sqrt()
}

/// Total arc length of a polyline.
fn polyline_length(points: &[[f64; 2]]) -> f64 {
    points.windows(2).map(|s| dist(s[0], s[1])).sum()
}

/// Hermite smoothstep of `x` clamped to [0,1].
fn smoothstep01(x: f64) -> f64 {
    let t = x.clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

/// Resamples a polyline to uniform spacing as close to `ds` as divides the
/// total length evenly (so index * spacing = arc position exactly). The first
/// and last input points are preserved exactly. Spacing widens automatically
/// when `ds` would exceed [`MAX_POINTS`]. Returns the points and the actual
/// spacing used.
fn resample(points: &[[f64; 2]], ds: f64) -> (Vec<[f64; 2]>, f64) {
    let total = polyline_length(points);
    if points.len() < 2 || total <= 0.0 {
        return (points.to_vec(), ds);
    }
    let ds = ds.max(total / MAX_POINTS as f64);
    let n_seg = (total / ds).round().max(1.0) as usize;
    let spacing = total / n_seg as f64;

    let mut out = Vec::with_capacity(n_seg + 1);
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
    (out, spacing)
}

/// Signed curvature per point from the turning angle between adjacent
/// segments, non-dimensionalized by the *local* channel width and
/// tanh-saturated at 1 (a bend can't be tighter than the channel is wide —
/// and the saturation is the anti-blowup bound), followed by one 1-2-1
/// smoothing pass (this model amplifies grid-frequency modes without it).
/// Endpoints get 0. Positive curvature and the migration normal below are
/// same-handed: their product always points toward the outer bank, in either
/// y orientation.
fn signed_curvature(points: &[[f64; 2]], widths: &[f64], curv: &mut Vec<f64>, scratch: &mut Vec<f64>) {
    let n = points.len();
    curv.clear();
    curv.resize(n, 0.0);
    for i in 1..n - 1 {
        let a = [points[i][0] - points[i - 1][0], points[i][1] - points[i - 1][1]];
        let b = [points[i + 1][0] - points[i][0], points[i + 1][1] - points[i][1]];
        let la = (a[0] * a[0] + a[1] * a[1]).sqrt();
        let lb = (b[0] * b[0] + b[1] * b[1]).sqrt();
        if la <= 1e-12 || lb <= 1e-12 {
            continue;
        }
        let cross = a[0] * b[1] - a[1] * b[0];
        let dot = a[0] * b[0] + a[1] * b[1];
        let theta = cross.atan2(dot);
        let c = theta / (0.5 * (la + lb));
        curv[i] = (c * widths[i]).tanh();
    }
    scratch.clear();
    scratch.extend_from_slice(curv);
    for i in 1..n - 1 {
        curv[i] = 0.25 * scratch[i - 1] + 0.5 * scratch[i] + 0.25 * scratch[i + 1];
    }
}

/// The upstream-weighted curvature convolution as an O(n) recursive
/// exponential moving average, upstream (index 0) to downstream. For uniform
/// spacing this IS the normalized convolution sum C(s-ξ)e^(-ξ/L) / sum
/// e^(-ξ/L) — normalization is what makes the OMEGA/GAMMA weights meaningful.
/// The lag length L = lag * local width, so the meander wavelength (~8.3 L)
/// scales with the channel: small tight bends upstream, big loops downstream.
fn upstream_filter(curv: &[f64], widths: &[f64], spacing: f64, p: &SimParams, out: &mut Vec<f64>) {
    out.clear();
    out.reserve(curv.len());
    let mut acc = 0.0;
    for (i, &c) in curv.iter().enumerate() {
        let a = (-spacing / (p.lag * widths[i])).exp();
        acc = if i == 0 { c } else { (1.0 - a) * c + a * acc };
        out.push(acc);
    }
}

/// Migration taper: 0 at each pinned endpoint, smoothstepping to 1 over
/// `taper_arc` of arc length.
fn endpoint_taper(arc: f64, total: f64, taper_arc: f64) -> f64 {
    smoothstep01(arc.min(total - arc) / taper_arc)
}

/// One migration step: displaces every interior point of `src` along its
/// normal into `dst` by the Howard-Knutson rate plus the continuous
/// bank-heterogeneity noise, tapered at the endpoints, modulated by the
/// erodibility map, and clamped to `max_disp`. All displacements are computed
/// from the same `src` snapshot. One RNG call per interior point in index
/// order — determinism depends on this.
fn migrate(
    src: &[[f64; 2]],
    dst: &mut Vec<[f64; 2]>,
    curv: &[f64],
    conv: &[f64],
    widths: &[f64],
    spacing: f64,
    p: &SimParams,
    erod: Option<&ErodGrid>,
    rng: &mut fastrand::Rng,
) {
    dst.clear();
    dst.extend_from_slice(src);
    let n = src.len();
    if n < 3 {
        return;
    }
    let total = (n - 1) as f64 * spacing;
    let mut last_t = [1.0f64, 0.0f64];
    for i in 1..n - 1 {
        let mut tx = src[i + 1][0] - src[i - 1][0];
        let mut ty = src[i + 1][1] - src[i - 1][1];
        let len = (tx * tx + ty * ty).sqrt();
        if len <= 1e-12 {
            tx = last_t[0];
            ty = last_t[1];
        } else {
            tx /= len;
            ty /= len;
            last_t = [tx, ty];
        }
        // Same-handed 90° rotation of the tangent: curvature * this normal
        // points toward the outer bank (verified against the cross-product
        // curvature sign; a y-flip flips both, so no y-down special case).
        let m = [ty, -tx];
        let taper = endpoint_taper(i as f64 * spacing, total, 8.0 * widths[i]);
        let e = erod.map_or(1.0, |g| g.sample(src[i]));
        let r = OMEGA * curv[i] + GAMMA * conv[i];
        let noise = p.noise_coeff * widths[i] * (rng.f64() * 2.0 - 1.0);
        let d = ((p.rate_coeff * widths[i] * r + noise) * taper * e).clamp(-p.max_disp, p.max_disp);
        dst[i] = [
            (src[i][0] + d * m[0]).clamp(-1.0, 2.0),
            (src[i][1] + d * m[1]).clamp(-1.0, 2.0),
        ];
    }
}

/// The one-time symmetry-breaking perturbation: displaces interior points
/// along their normals by a small random undulation seeded directly in the
/// model's growing wavelength band — a few random-phase sinusoids in arc
/// length around `lambda_star`, the fastest-growing wavelength (~8.3x the
/// upstream lag for this exponential kernel). White noise instead would put
/// almost all its energy at short, damped wavelengths, leaving the upstream
/// half of the river straight for hundreds of iterations while the downstream
/// half develops. RNG calls are fixed in number and order — determinism
/// depends on this.
fn seed_perturbation(
    points: &mut [[f64; 2]],
    rng: &mut fastrand::Rng,
    amp: f64,
    lambda_star: f64,
    spacing: f64,
    p: &SimParams,
) {
    let n = points.len();
    if n < 3 {
        return;
    }
    let tau = std::f64::consts::TAU;
    // Wavelength multipliers spanning the growing band around lambda_star.
    let waves: Vec<(f64, f64, f64)> = [0.7, 1.0, 1.5, 2.2]
        .iter()
        .map(|m| {
            let k = tau / (m * lambda_star);
            let phase = rng.f64() * tau;
            let a = amp * (0.5 + 0.5 * rng.f64());
            (a, k, phase)
        })
        .collect();
    let scale = 1.0 / waves.len() as f64;
    let total = (n - 1) as f64 * spacing;
    let mut last_t = [1.0f64, 0.0f64];
    for i in 1..n - 1 {
        let mut tx = points[i + 1][0] - points[i - 1][0];
        let mut ty = points[i + 1][1] - points[i - 1][1];
        let len = (tx * tx + ty * ty).sqrt();
        if len <= 1e-12 {
            tx = last_t[0];
            ty = last_t[1];
        } else {
            tx /= len;
            ty /= len;
            last_t = [tx, ty];
        }
        let arc = i as f64 * spacing;
        let taper = endpoint_taper(arc, total, p.taper_arc);
        let d: f64 = waves.iter().map(|(a, k, ph)| a * (k * arc + ph).sin()).sum::<f64>()
            * scale
            * taper;
        points[i][0] += d * ty;
        points[i][1] += d * -tx;
    }
}

/// Finds the first neck about to pinch shut: the pair (i, j) with the smallest
/// i (then smallest j) whose along-curve separation exceeds the local minimum
/// but whose Euclidean distance is under the *local* cutoff distance (cutoff
/// multiplier x the narrower of the two local widths — upstream necks pinch at
/// upstream scale). A spatial hash keeps the scan O(n); the hash is only an
/// accelerator — the returned pair is selected by index order, never by map
/// iteration order, so the result is deterministic. Endpoints are never part
/// of a cutoff. The hash cell uses the maximum (downstream) cutoff distance,
/// so every local-threshold candidate is within the 3x3 neighborhood.
fn find_cutoff(
    points: &[[f64; 2]],
    widths: &[f64],
    spacing: f64,
    p: &SimParams,
    hash: &mut HashMap<(i32, i32), Vec<u32>>,
) -> Option<(usize, usize)> {
    let n = points.len();
    if n < 8 {
        return None;
    }
    let cell = p.cutoff * p.w_norm;
    hash.clear();
    for (i, pt) in points.iter().enumerate() {
        let key = ((pt[0] / cell).floor() as i32, (pt[1] / cell).floor() as i32);
        hash.entry(key).or_default().push(i as u32);
    }
    let sep_widths = 6.0f64.max(4.0 * p.cutoff);
    for i in 1..n - 1 {
        let pt = points[i];
        let kx = (pt[0] / cell).floor() as i32;
        let ky = (pt[1] / cell).floor() as i32;
        let min_sep_idx = (sep_widths * widths[i] / spacing).ceil() as usize;
        let mut best: Option<usize> = None;
        for dy in -1..=1 {
            for dx in -1..=1 {
                let Some(bucket) = hash.get(&(kx + dx, ky + dy)) else { continue };
                for &ju in bucket {
                    let j = ju as usize;
                    if j <= i + min_sep_idx || j >= n - 1 {
                        continue;
                    }
                    let local = p.cutoff * widths[i].min(widths[j]);
                    let ex = points[j][0] - pt[0];
                    let ey = points[j][1] - pt[1];
                    if ex * ex + ey * ey < local * local && best.is_none_or(|b| j < b) {
                        best = Some(j);
                    }
                }
            }
        }
        if let Some(j) = best {
            return Some((i, j));
        }
    }
    None
}

/// Splices the loop between `i` and `j` out of the channel and returns it as
/// the oxbow — points plus the local channel widths it had when abandoned, so
/// the lake renders at the width the river was there (its ends nearly touch,
/// so it renders as a closed loop). The splice kink is healed by the next
/// iteration's resample + smoothing.
fn apply_cutoff(
    points: &mut Vec<[f64; 2]>,
    widths: &[f64],
    i: usize,
    j: usize,
) -> (Vec<[f64; 2]>, Vec<f64>) {
    let oxbow = (points[i..=j].to_vec(), widths[i..=j].to_vec());
    points.drain(i + 1..j);
    oxbow
}

/// Stamps the channel corridor into the migration-history grid as discs of
/// the *local* channel radius, max-composed with `age`. Much cheaper than a
/// full rasterize per iteration; discs at the point spacing overlap into a
/// solid corridor. For radii much larger than the point spacing every k-th
/// point suffices (stride derived from the narrowest radius so the upstream
/// reach never gaps).
fn stamp_corridor(
    grid: &mut [f32],
    gw: usize,
    gh: usize,
    points: &[[f64; 2]],
    widths: &[f64],
    spacing: f64,
    age: f32,
) {
    let gmax = gw.max(gh) as f64;
    let r_min = (widths.iter().copied().fold(f64::INFINITY, f64::min) * gmax * 0.5).max(1.0);
    let spacing_px = (spacing * gmax).max(1e-6);
    let stride = ((r_min / spacing_px) * 0.5).floor().max(1.0) as usize;
    for (pt, w_i) in points.iter().zip(widths).step_by(stride) {
        let r = (w_i * gmax * 0.5).max(1.0);
        let r2 = r * r;
        let cx = pt[0] * gw as f64;
        let cy = pt[1] * gh as f64;
        let x0 = ((cx - r).floor() as i64).clamp(0, gw as i64) as usize;
        let x1 = ((cx + r).ceil() as i64).clamp(0, gw as i64) as usize;
        let y0 = ((cy - r).floor() as i64).clamp(0, gh as i64) as usize;
        let y1 = ((cy + r).ceil() as i64).clamp(0, gh as i64) as usize;
        for y in y0..y1 {
            let py = y as f64 + 0.5;
            for x in x0..x1 {
                let px = x as f64 + 0.5;
                let dx = px - cx;
                let dy = py - cy;
                if dx * dx + dy * dy <= r2 {
                    let idx = y * gw + x;
                    grid[idx] = grid[idx].max(age);
                }
            }
        }
    }
}

/// Rasterizes a polyline with a per-point stroke radius (in normalized width
/// units, converted to pixels here) into a 1-channel [0,1] mask with a ~1px
/// anti-aliased edge. The per-segment signed field `radius - distance` is
/// max-composed, so joints between different-width segments blend smoothly.
/// Cost is proportional to stroke area (per-segment padded bounding boxes),
/// like `Curve::rasterize`.
fn rasterize_variable(
    points: &[[f64; 2]],
    widths: &[f64],
    gw: u32,
    gh: u32,
    closed: bool,
) -> Vec<f32> {
    let w = gw as usize;
    let h = gh as usize;
    let mut field = vec![f32::NEG_INFINITY; w * h];
    let n = points.len();
    if n >= 2 {
        let gmax = gw.max(gh) as f64;
        let px_of = |p: [f64; 2]| [p[0] * gw as f64, p[1] * gh as f64];
        let r_of = |i: usize| (widths[i] * gmax * 0.5).max(0.5);
        let seg_count = if closed { n } else { n - 1 };
        for s in 0..seg_count {
            let e = (s + 1) % n;
            let a = px_of(points[s]);
            let b = px_of(points[e]);
            let ra = r_of(s);
            let rb = r_of(e);
            let pad = ra.max(rb) + 2.0;
            let x0 = ((a[0].min(b[0]) - pad).floor() as i64).clamp(0, w as i64) as usize;
            let x1 = ((a[0].max(b[0]) + pad).ceil() as i64).clamp(0, w as i64) as usize;
            let y0 = ((a[1].min(b[1]) - pad).floor() as i64).clamp(0, h as i64) as usize;
            let y1 = ((a[1].max(b[1]) + pad).ceil() as i64).clamp(0, h as i64) as usize;
            let dx = b[0] - a[0];
            let dy = b[1] - a[1];
            let len_sq = (dx * dx + dy * dy).max(1e-12);
            for y in y0..y1 {
                let py = y as f64 + 0.5;
                for x in x0..x1 {
                    let px = x as f64 + 0.5;
                    let t = (((px - a[0]) * dx + (py - a[1]) * dy) / len_sq).clamp(0.0, 1.0);
                    let cx = a[0] + t * dx;
                    let cy = a[1] + t * dy;
                    let ex = px - cx;
                    let ey = py - cy;
                    let d = (ex * ex + ey * ey).sqrt();
                    let r = ra + t * (rb - ra);
                    let v = (r - d) as f32;
                    let idx = y * w + x;
                    if v > field[idx] {
                        field[idx] = v;
                    }
                }
            }
        }
    }
    field
        .into_iter()
        .map(|v| {
            let t = ((v + 1.0) * 0.5).clamp(0.0, 1.0);
            t * t * (3.0 - 2.0 * t)
        })
        .collect()
}

/// Shortest distance from point `p` to the segment `a`-`b` (f64 twin of the
/// curve rasterizer's helper).
fn point_segment_distance(p: [f64; 2], a: [f64; 2], b: [f64; 2]) -> f64 {
    let dx = b[0] - a[0];
    let dy = b[1] - a[1];
    let len_sq = dx * dx + dy * dy;
    if len_sq < 1e-24 {
        return dist(p, a);
    }
    let t = (((p[0] - a[0]) * dx + (p[1] - a[1]) * dy) / len_sq).clamp(0.0, 1.0);
    dist(p, [a[0] + t * dx, a[1] + t * dy])
}

/// Ramer-Douglas-Peucker decimation (iterative stack), doubling the tolerance
/// until the result fits `max_points`. Keeps the persisted output Curve small
/// — its points serialize into every graph save.
fn rdp_decimate(points: &[[f64; 2]], mut tol: f64, max_points: usize) -> Vec<[f32; 2]> {
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

/// Per-pixel max-composite of `src` into `dst`.
fn max_composite(dst: &mut [f32], src: &[f32]) {
    for (d, s) in dst.iter_mut().zip(src) {
        *d = d.max(*s);
    }
}

/// Wraps raw 1-channel pixels into an image output value. Raw linear mask
/// values, no sRGB encode — matches rasterize curve, not the heightmap nodes.
fn image_value(w: u32, h: u32, pixels: Vec<f32>) -> Value {
    Value::Image {
        data: Arc::new(FloatImage::from_raw(w, h, 1, pixels).unwrap()),
        change_id: get_id(),
    }
}

/// Operation that evolves a curve into a meandering river with oxbow cutoffs.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpCurveSimulationMeander {}

impl OpCurveSimulationMeander {
    /// Returns the node metadata (name, description, help) for this operation.
    pub fn settings() -> NodeSettings {
        NodeSettings {
            name: "meander".to_string(),
            description: "Evolves a drawn curve into a meandering river (Howard-Knutson bank migration): bends grow, translate downstream, and cut off into oxbow lakes.".to_string(),
            help: "Evolves a drawn curve as a river centerline using the Howard & Knutson (1984) curvature-driven bank-migration model (the physics behind meanderpy): each point migrates along its normal at a rate set by an upstream-weighted average of curvature, so bends grow, translate downstream, and skew - and when a neck pinches shut, the loop is cut off and left behind as an oxbow lake. Step through iterations to watch the river age.\n\nOutputs the evolved centerline as a curve (feed it into rasterize curve or carve river), plus three masks: the river with its oxbow lakes, the oxbows alone, and a migration map - the age-graded corridor the channel swept over time (newer = brighter), the scroll-bar/point-bar scarring visible around real rivers.\n\nThe channel is not constant width: it grows from upstream width x channel width at the source to the full channel width downstream (like accumulating discharge), and width variation adds bend widening plus gentle noise in the rendered masks. The local width is also the simulation's local physical scale - the tightest possible bend, the meander wavelength, the cutoff distance, and the migration step all follow it - so the upstream reach forms small tight bends and the downstream reach big lazy loops, like a real river. Channel width is in pixels at a 1024px reference.\n\nA perfectly straight line has zero curvature and never evolves; seed wobble adds the perturbation that starts the process (vary the seed for different rivers from the same curve). The optional erodibility map scales migration spatially (bright = mobile banks, dark = resistant; unconnected = uniform). Endpoints stay pinned. Closed curves evolve as an open path with a pinned seam. Deterministic from the seed; the rasters do not tile. The curve output is the centerline only - width lives in the raster outputs.".to_string(),
        }
    }

    /// Creates the inputs in the simulation convention: seed and dimensions
    /// first, the curve and optional guidance map, the main driver
    /// (iterations), then the fine-tuning parameters.
    pub fn create_inputs() -> Vec<Input> {
        vec![
            Input::new("seed".to_string(), Value::Integer(1), Some(InputSettings::DragValue { clamp: None, speed: None }), None)
                .with_description("Random seed for the initial perturbation; vary it for different rivers from the same curve."),
            Input::new("width".to_string(), Value::Integer(512), Some(InputSettings::DragValue { clamp: Some((1.0, 4096.0)), speed: None }), None)
                .with_description("Width of the raster outputs in pixels (the curve output is resolution-independent)."),
            Input::new("height".to_string(), Value::Integer(512), Some(InputSettings::DragValue { clamp: Some((1.0, 4096.0)), speed: None }), None)
                .with_description("Height of the raster outputs in pixels (the curve output is resolution-independent)."),
            Input::new("curve".to_string(), Value::Curve(Curve::default()), None, None)
                .with_description("The initial river centerline; usually connected from a curve node."),
            Input::new("erodibility".to_string(), Value::Image { data: default_image(), change_id: get_id() }, None, None)
                .with_description("Optional map scaling bank migration spatially: bright = mobile banks, dark = resistant. Uniform when unconnected."),
            Input::new("iterations".to_string(), Value::Integer(100), Some(InputSettings::DragValue { clamp: Some((0.0, 2000.0)), speed: None }), None)
                .with_description("Simulation steps; step through to watch the river age. 0 passes the curve through unchanged."),
            Input::new("migration rate".to_string(), Value::Decimal(0.4), Some(InputSettings::Slider { range: (0.0, 1.0), step_by: Some(0.01), clamp_to_range: true }), None)
                .with_description("How far the banks migrate per iteration, as a fraction of the channel width."),
            Input::new("channel width".to_string(), Value::Decimal(10.0), Some(InputSettings::DragValue { clamp: Some((1.0, 128.0)), speed: Some(0.1) }), None)
                .with_description("River width at the downstream end, in pixels at a 1024px reference (scales with output size). Also the simulation's local physical scale: bend tightness, meander wavelength, cutoff necks, and step size all follow the local width."),
            Input::new("upstream width".to_string(), Value::Decimal(0.35), Some(InputSettings::Slider { range: (0.05, 1.0), step_by: Some(0.01), clamp_to_range: true }), None)
                .with_description("Channel width at the source as a fraction of the downstream width; the river widens downstream like accumulating discharge (sqrt growth). 1 = constant width."),
            Input::new("width variation".to_string(), Value::Decimal(0.2), Some(InputSettings::Slider { range: (0.0, 1.0), step_by: Some(0.01), clamp_to_range: true }), None)
                .with_description("Local width irregularity in the rendered masks: wider at bends (cut bank + point bar) plus gentle noise along the length, so the banks aren't perfect parallel lines."),
            Input::new("upstream lag".to_string(), Value::Decimal(1.5), Some(InputSettings::Slider { range: (0.5, 8.0), step_by: Some(0.1), clamp_to_range: true }), None)
                .with_description("How far upstream curvature influences migration, in channel widths. Sets the meander wavelength (~8x the lag) and the downstream translation of bends; longer lag = longer, slower-growing bends."),
            Input::new("cutoff distance".to_string(), Value::Decimal(1.5), Some(InputSettings::Slider { range: (0.5, 4.0), step_by: Some(0.1), clamp_to_range: true }), None)
                .with_description("Neck separation below which a loop is cut off into an oxbow, in channel widths."),
            Input::new("seed wobble".to_string(), Value::Decimal(0.25), Some(InputSettings::Slider { range: (0.0, 1.0), step_by: Some(0.01), clamp_to_range: true }), None)
                .with_description("Bank irregularity, in channel widths: the initial symmetry-breaking undulation plus a little continuous per-iteration noise that keeps seeding new bends (a straight line never meanders without it)."),
        ]
    }

    /// Creates the outputs: the evolved centerline curve and three masks.
    pub fn create_outputs() -> Vec<Output> {
        vec![
            Output::new("curve".to_string(), Value::Curve(Curve::default()), None)
                .with_description("The evolved river centerline (main channel only; oxbows are in the rasters). Feed into rasterize curve or carve river."),
            Output::new("river mask".to_string(), Value::Image { data: default_image(), change_id: get_id() }, None)
                .with_description("The river at channel width, oxbow lakes included, white on black."),
            Output::new("oxbows".to_string(), Value::Image { data: default_image(), change_id: get_id() }, None)
                .with_description("Only the cut-off oxbow lakes, white on black; black when none have formed yet."),
            Output::new("migration map".to_string(), Value::Image { data: default_image(), change_id: get_id() }, None)
                .with_description("Age-graded corridor the channel swept while migrating (newer = brighter): scroll-bar/point-bar scarring for floodplain texturing."),
        ]
    }

    /// Runs the meander simulation.
    ///
    /// 1. Flattens and uniformly resamples the input centerline
    /// 2. Applies the seeded symmetry-breaking perturbation
    /// 3. Per iteration: resample, curvature (saturated + smoothed), upstream
    ///    EMA, normal migration, neck cutoffs, migration-map stamp
    /// 4. Rasterizes the channel + oxbows and decimates the output curve
    pub async fn run(inputs: &mut [Input]) -> Result<OperationResponse, OperationError> {
        let start_time = Instant::now();
        let mut input_errors: Vec<(usize, String)> = vec![];

        let seed_converted = convert_input(inputs, 0, ValueType::Integer, &mut input_errors);
        let width_converted = convert_input(inputs, 1, ValueType::Integer, &mut input_errors);
        let height_converted = convert_input(inputs, 2, ValueType::Integer, &mut input_errors);
        let curve_converted = convert_input(inputs, 3, ValueType::Curve, &mut input_errors);
        let erod_converted = convert_input(inputs, 4, ValueType::Image, &mut input_errors);
        let iterations_converted = convert_input(inputs, 5, ValueType::Integer, &mut input_errors);
        let rate_converted = convert_input(inputs, 6, ValueType::Decimal, &mut input_errors);
        let chan_width_converted = convert_input(inputs, 7, ValueType::Decimal, &mut input_errors);
        let upstream_converted = convert_input(inputs, 8, ValueType::Decimal, &mut input_errors);
        let variation_converted = convert_input(inputs, 9, ValueType::Decimal, &mut input_errors);
        let lag_converted = convert_input(inputs, 10, ValueType::Decimal, &mut input_errors);
        let cutoff_converted = convert_input(inputs, 11, ValueType::Decimal, &mut input_errors);
        let wobble_converted = convert_input(inputs, 12, ValueType::Decimal, &mut input_errors);

        if !input_errors.is_empty() {
            return Err(OperationError { input_errors, node_error: None });
        }

        let Value::Integer(seed) = seed_converted.unwrap() else { unreachable!() };
        let Value::Integer(width) = width_converted.unwrap() else { unreachable!() };
        let Value::Integer(height) = height_converted.unwrap() else { unreachable!() };
        let Value::Curve(curve) = curve_converted.unwrap() else { unreachable!() };
        let Value::Image { data: erod_data, .. } = erod_converted.unwrap() else { unreachable!() };
        let Value::Integer(iterations) = iterations_converted.unwrap() else { unreachable!() };
        let Value::Decimal(migration_rate) = rate_converted.unwrap() else { unreachable!() };
        let Value::Decimal(channel_width) = chan_width_converted.unwrap() else { unreachable!() };
        let Value::Decimal(upstream_frac) = upstream_converted.unwrap() else { unreachable!() };
        let Value::Decimal(variation) = variation_converted.unwrap() else { unreachable!() };
        let Value::Decimal(lag) = lag_converted.unwrap() else { unreachable!() };
        let Value::Decimal(cutoff) = cutoff_converted.unwrap() else { unreachable!() };
        let Value::Decimal(wobble) = wobble_converted.unwrap() else { unreachable!() };

        let w = width.clamp(1, 4096) as u32;
        let h = height.clamp(1, 4096) as u32;
        let iterations = iterations.clamp(0, 2000) as usize;
        let migration_rate = migration_rate.clamp(0.0, 1.0) as f64;
        let channel_width = channel_width.clamp(1.0, 128.0);
        let upstream_frac = upstream_frac.clamp(0.05, 1.0) as f64;
        let variation = variation.clamp(0.0, 1.0) as f64;
        let lag = lag.clamp(0.5, 8.0) as f64;
        let cutoff = (cutoff.clamp(0.5, 4.0)) as f64;
        let wobble = wobble.clamp(0.0, 1.0) as f64;

        let w_norm = channel_width as f64 / 1024.0;
        // Sample spacing follows the *narrowest* reach so upstream bends (whose
        // wavelength scales with the local width) stay resolved.
        let ds = (0.5 * w_norm * upstream_frac.max(0.25)).min(0.01);
        let params = SimParams {
            lag,
            rate_coeff: 0.2 * migration_rate,
            max_disp: 0.5 * ds,
            w_norm,
            upstream_frac,
            noise_coeff: 0.2 * wobble,
            cutoff,
            taper_arc: 8.0 * w_norm,
        };

        let poly: Vec<[f64; 2]> = curve
            .flatten(48)
            .iter()
            .map(|p| [p[0] as f64, p[1] as f64])
            .collect();
        let degenerate = poly.len() < 2 || polyline_length(&poly) < 4.0 * ds;

        let mut widths: Vec<f64> = Vec::new();

        // Passthrough: degenerate input or zero iterations. The RNG is never
        // consumed, the curve passes through untouched, and the rasters stay
        // valid (the un-migrated corridor at its base width profile; no
        // cosmetic width variation, which would need the RNG).
        if degenerate || iterations == 0 {
            let pixel_count = (w * h) as usize;
            let mask = if degenerate {
                vec![0.0f32; pixel_count]
            } else {
                let (pts, _) = resample(&poly, ds);
                widths_along(pts.len(), &params, &mut widths);
                rasterize_variable(&pts, &widths, w, h, false)
            };
            return Ok(OperationResponse {
                time: Instant::now().duration_since(start_time),
                responses: vec![
                    OutputResponse { value: Value::Curve(curve) },
                    OutputResponse { value: image_value(w, h, mask.clone()) },
                    OutputResponse { value: image_value(w, h, vec![0.0f32; pixel_count]) },
                    OutputResponse { value: image_value(w, h, mask) },
                ],
            });
        }

        let erod = if is_unconnected(&erod_data) {
            None
        } else {
            let gw = erod_data.width().max(1) as usize;
            let gh = erod_data.height().max(1) as usize;
            Some(ErodGrid { data: guidance_map_to_grid(&erod_data, gw, gh), w: gw, h: gh })
        };

        let mut rng = fastrand::Rng::with_seed(seed.max(1) as u64);
        let (mut pts, mut spacing) = resample(&poly, ds);
        let lambda_star = 8.3 * lag * w_norm;
        seed_perturbation(&mut pts, &mut rng, wobble * w_norm, lambda_star, spacing, &params);
        // Width-variation noise: fixed frequencies (cycles per unit of arc
        // fraction), seeded amplitude + phase. Drawn up front so the pattern
        // doesn't re-roll when the iteration count changes.
        let width_waves: Vec<(f64, f64, f64)> = [3.0, 7.0, 13.0]
            .iter()
            .map(|&f| (0.5 + 0.5 * rng.f64(), f, rng.f64() * std::f64::consts::TAU))
            .collect();

        let mut curv: Vec<f64> = Vec::new();
        let mut conv: Vec<f64> = Vec::new();
        let mut scratch: Vec<f64> = Vec::new();
        let mut buf: Vec<[f64; 2]> = Vec::new();
        let mut hash: HashMap<(i32, i32), Vec<u32>> = HashMap::new();
        let mut oxbows: Vec<(Vec<[f64; 2]>, Vec<f64>)> = Vec::new();
        let mut migration = vec![0.0f32; (w * h) as usize];

        for it in 0..iterations {
            (pts, spacing) = resample(&pts, ds);
            if pts.len() < 4 {
                break; // cutoffs consumed the channel; freeze evolution
            }
            widths_along(pts.len(), &params, &mut widths);
            signed_curvature(&pts, &widths, &mut curv, &mut scratch);
            upstream_filter(&curv, &widths, spacing, &params, &mut conv);
            migrate(&pts, &mut buf, &curv, &conv, &widths, spacing, &params, erod.as_ref(), &mut rng);
            std::mem::swap(&mut pts, &mut buf);

            while let Some((i, j)) = find_cutoff(&pts, &widths, spacing, &params, &mut hash) {
                let oxbow = apply_cutoff(&mut pts, &widths, i, j);
                // The widths array is stale after the splice (it is positional)
                // but is rebuilt at the top of the next iteration; the cutoff
                // loop only needs it as a local length scale, where the error
                // is one splice's worth of downstream shift.
                if oxbows.len() < MAX_OXBOWS {
                    oxbows.push(oxbow);
                }
                if pts.len() < 8 {
                    break;
                }
            }

            let age = (it + 1) as f32 / iterations as f32;
            widths_along(pts.len(), &params, &mut widths);
            stamp_corridor(&mut migration, w as usize, h as usize, &pts, &widths, spacing, age);
        }

        // Rendered width profile: the base downstream widening plus the
        // cosmetic variation — wider at bends (cut bank + point bar) and a
        // gentle seeded noise along the length.
        widths_along(pts.len(), &params, &mut widths);
        signed_curvature(&pts, &widths, &mut curv, &mut scratch);
        let denom = (pts.len().max(2) - 1) as f64;
        let render_widths: Vec<f64> = widths
            .iter()
            .enumerate()
            .map(|(i, &wi)| {
                let t = i as f64 / denom;
                let noise: f64 = width_waves
                    .iter()
                    .map(|(a, f, ph)| a * (f * std::f64::consts::TAU * t + ph).sin())
                    .sum::<f64>()
                    / width_waves.len() as f64;
                wi * (1.0 + variation * (0.6 * curv[i].abs() + 0.4 * noise))
            })
            .collect();

        // Rasters: the main channel stroked at its local width, each oxbow as
        // a closed *stroked* ring at the widths it was abandoned with (an
        // oxbow lake is the old channel, not its filled interior),
        // max-composed together for the river mask.
        let mut river = rasterize_variable(&pts, &render_widths, w, h, false);
        let mut oxbow_px = vec![0.0f32; (w * h) as usize];
        for (ox_pts, ox_widths) in &oxbows {
            let px = rasterize_variable(ox_pts, ox_widths, w, h, true);
            max_composite(&mut oxbow_px, &px);
        }
        max_composite(&mut river, &oxbow_px);

        let out_curve = Curve {
            points: rdp_decimate(&pts, 0.5 * ds, MAX_OUTPUT_POINTS),
            closed: false,
            interpolation: CurveInterpolation::Linear,
            handles: Vec::new(),
        };

        Ok(OperationResponse {
            time: Instant::now().duration_since(start_time),
            responses: vec![
                OutputResponse { value: Value::Curve(out_curve) },
                OutputResponse { value: image_value(w, h, river) },
                OutputResponse { value: image_value(w, h, oxbow_px) },
                OutputResponse { value: image_value(w, h, migration) },
            ],
        })
    }
}

#[cfg(test)]
#[path = "meander_tests.rs"]
mod tests;
