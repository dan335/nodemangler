//! Trace the iso-contour of an image into a curve.
//!
//! Thresholds an image's luminance into an inside/outside mask, runs marching
//! squares (with linear edge interpolation and saddle disambiguation via the
//! cell-center sample) to extract the boundary contours, keeps the longest one
//! by arc length, normalizes it to `[0,1]²` and decimates it with RDP into a
//! `Linear` curve. The reverse of `rasterize curve`.

use crate::curve::Curve;
use crate::get_id;
use crate::input::{Input, InputSettings};
use crate::node_settings::NodeSettings;
use crate::operations::curves::common::{linear_curve, rdp_decimate, MAX_OUTPUT_POINTS};
use crate::operations::numbers::image::pixel_luma;
use crate::operations::{convert_input, default_image, OperationError, OperationResponse, OutputResponse};
use crate::output::Output;
use crate::value::{Value, ValueType};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::time::Instant;

#[cfg(test)]
#[path = "trace_contour_tests.rs"]
mod tests;

// Edge codes for a marching-squares cell (T=top, R=right, B=bottom, L=left).
const T: u8 = 0;
const R: u8 = 1;
const B: u8 = 2;
const L: u8 = 3;

/// A single marching-squares contour: a polyline in normalized `[0,1]²` and
/// whether it closed back on itself (a loop) or ran off the image border.
struct Contour {
    points: Vec<[f64; 2]>,
    closed: bool,
}

/// Global id of the grid edge a crossing sits on, so crossings shared between
/// two adjacent cells match exactly and link into connected contours.
///
/// Horizontal edges (between samples `(x,y)`-`(x+1,y)`) are numbered first,
/// then vertical edges (between `(x,y)`-`(x,y+1)`).
#[inline]
fn edge_id(cx: usize, cy: usize, edge: u8, w: usize, h: usize) -> u64 {
    let h_base = 0u64;
    let v_base = ((w - 1) * h) as u64;
    match edge {
        // top: horizontal edge (cx, cy)
        0 => h_base + (cy * (w - 1) + cx) as u64,
        // right: vertical edge (cx+1, cy)
        1 => v_base + (cy * w + (cx + 1)) as u64,
        // bottom: horizontal edge (cx, cy+1)
        2 => h_base + ((cy + 1) * (w - 1) + cx) as u64,
        // left: vertical edge (cx, cy)
        _ => v_base + (cy * w + cx) as u64,
    }
}

/// Linearly interpolated crossing point (in sample-index space) of `edge` for
/// the cell with top-left sample `(cx, cy)`, given the four corner luminances.
#[inline]
fn edge_point(cx: usize, cy: usize, edge: u8, v: [f32; 4], thr: f32) -> [f64; 2] {
    // Corner order: v[0]=TL(cx,cy), v[1]=TR(cx+1,cy), v[2]=BR(cx+1,cy+1), v[3]=BL(cx,cy+1).
    let interp = |a: f32, b: f32| -> f64 {
        let denom = (b - a) as f64;
        if denom.abs() < 1e-20 {
            0.5
        } else {
            ((thr - a) as f64 / denom).clamp(0.0, 1.0)
        }
    };
    let (cxf, cyf) = (cx as f64, cy as f64);
    match edge {
        0 => [cxf + interp(v[0], v[1]), cyf],           // top
        1 => [cxf + 1.0, cyf + interp(v[1], v[2])],     // right
        2 => [cxf + interp(v[3], v[2]), cyf + 1.0],     // bottom
        _ => [cxf, cyf + interp(v[0], v[3])],           // left
    }
}

/// Marching-squares segment pairs for a cell case (corner bits TL=1, TR=2,
/// BR=4, BL=8). Saddle cases (5, 10) are resolved by the cell-center sample.
fn segments_for_case(case: u8, center_in: bool) -> &'static [(u8, u8)] {
    // Non-crossing pairings of the 4 edge crossings on a square:
    //   optA = [(T,R),(B,L)]   optB = [(L,T),(R,B)]
    const OPT_A: [(u8, u8); 2] = [(T, R), (B, L)];
    const OPT_B: [(u8, u8); 2] = [(L, T), (R, B)];
    match case {
        1 => &[(L, T)],
        2 => &[(T, R)],
        3 => &[(L, R)],
        4 => &[(R, B)],
        5 => {
            if center_in {
                &OPT_A
            } else {
                &OPT_B
            }
        }
        6 => &[(T, B)],
        7 => &[(L, B)],
        8 => &[(L, B)],
        9 => &[(T, B)],
        10 => {
            if center_in {
                &OPT_B
            } else {
                &OPT_A
            }
        }
        11 => &[(R, B)],
        12 => &[(L, R)],
        13 => &[(T, R)],
        14 => &[(L, T)],
        _ => &[],
    }
}

/// Extract all boundary contours of the `luma >= thr` mask via marching
/// squares, returned as polylines in normalized `[0,1]²` (sample `i` maps to
/// `(i + 0.5) / dim`, matching the rasterizer's pixel-center convention).
fn trace_contours(luma: &[f32], w: usize, h: usize, thr: f32) -> Vec<Contour> {
    if w < 2 || h < 2 {
        return Vec::new();
    }
    // Each crossing (keyed by edge id) links to at most two neighbors, one per
    // adjacent cell that emits a segment through it.
    let mut points: HashMap<u64, [f64; 2]> = HashMap::new();
    let mut adj: HashMap<u64, Vec<u64>> = HashMap::new();

    let val = |x: usize, y: usize| luma[y * w + x];

    for cy in 0..h - 1 {
        for cx in 0..w - 1 {
            let v = [val(cx, cy), val(cx + 1, cy), val(cx + 1, cy + 1), val(cx, cy + 1)];
            let mut case = 0u8;
            if v[0] >= thr {
                case |= 1;
            }
            if v[1] >= thr {
                case |= 2;
            }
            if v[2] >= thr {
                case |= 4;
            }
            if v[3] >= thr {
                case |= 8;
            }
            if case == 0 || case == 15 {
                continue;
            }
            let center_in = (v[0] + v[1] + v[2] + v[3]) * 0.25 >= thr;
            for &(ea, eb) in segments_for_case(case, center_in) {
                let ia = edge_id(cx, cy, ea, w, h);
                let ib = edge_id(cx, cy, eb, w, h);
                points.entry(ia).or_insert_with(|| edge_point(cx, cy, ea, v, thr));
                points.entry(ib).or_insert_with(|| edge_point(cx, cy, eb, v, thr));
                adj.entry(ia).or_default().push(ib);
                adj.entry(ib).or_default().push(ia);
            }
        }
    }

    if points.is_empty() {
        return Vec::new();
    }

    // Deterministic iteration: sort the crossing ids.
    let mut ids: Vec<u64> = points.keys().copied().collect();
    ids.sort_unstable();

    let norm = |p: [f64; 2]| [(p[0] + 0.5) / w as f64, (p[1] + 0.5) / h as f64];

    let mut visited: HashSet<u64> = HashSet::new();
    let mut contours: Vec<Contour> = Vec::new();

    // Walk a chain/loop from `start`. `closed` reports whether it returned to
    // its origin (a loop) versus terminating at a degree-1 border crossing.
    let walk = |start: u64, visited: &mut HashSet<u64>| -> Contour {
        let mut poly = Vec::new();
        let mut prev: Option<u64> = None;
        let mut cur = start;
        let mut closed = false;
        loop {
            visited.insert(cur);
            poly.push(norm(points[&cur]));
            let mut next = None;
            if let Some(nbrs) = adj.get(&cur) {
                for &nb in nbrs {
                    if Some(nb) != prev && !visited.contains(&nb) {
                        next = Some(nb);
                        break;
                    }
                }
                // Loop closure: a neighbor equal to the start (already visited).
                if next.is_none() && nbrs.iter().any(|&nb| nb == start && Some(nb) != prev) {
                    closed = true;
                }
            }
            match next {
                Some(n) => {
                    prev = Some(cur);
                    cur = n;
                }
                None => break,
            }
        }
        Contour { points: poly, closed }
    };

    // Open chains first (crossings with a single neighbor are border ends).
    for &id in &ids {
        if visited.contains(&id) {
            continue;
        }
        if adj.get(&id).map(|n| n.len()).unwrap_or(0) == 1 {
            contours.push(walk(id, &mut visited));
        }
    }
    // Remaining crossings form closed loops.
    for &id in &ids {
        if visited.contains(&id) {
            continue;
        }
        let mut c = walk(id, &mut visited);
        c.closed = true;
        contours.push(c);
    }

    contours
}

/// Arc length of a polyline (adds the closing segment when `closed`).
fn contour_length(c: &Contour) -> f64 {
    let n = c.points.len();
    if n < 2 {
        return 0.0;
    }
    let mut total = 0.0;
    for w in c.points.windows(2) {
        let dx = w[1][0] - w[0][0];
        let dy = w[1][1] - w[0][1];
        total += (dx * dx + dy * dy).sqrt();
    }
    if c.closed {
        let dx = c.points[0][0] - c.points[n - 1][0];
        let dy = c.points[0][1] - c.points[n - 1][1];
        total += (dx * dx + dy * dy).sqrt();
    }
    total
}

/// Operation that traces an image's threshold contour into a curve.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpCurveFromImageTraceContour {}

impl OpCurveFromImageTraceContour {
    /// Returns the node metadata (name, description, help).
    pub fn settings() -> NodeSettings {
        NodeSettings {
            name: "trace contour".to_string(),
            description: "Traces an image's threshold boundary into a curve.".to_string(),
            help: "Thresholds the image's luminance into an inside/outside mask, then runs marching squares (linear edge interpolation, saddles disambiguated by the cell-center sample) to extract the boundary. The longest contour by arc length is normalized to 0-1 curve space and simplified with the tolerance (in pixels at a 1024px reference) into a Linear curve.\n\nA loop (a shape fully inside the image) traces closed; a boundary running off the image edge traces open. An empty mask, a full (all-inside) mask, or an image smaller than 2x2 yields the default placeholder curve. The reverse of 'rasterize curve'.".to_string(),
        }
    }

    /// Creates the default inputs: image, threshold, tolerance.
    pub fn create_inputs() -> Vec<Input> {
        vec![
            Input::new("image".to_string(), Value::Image { data: default_image(), change_id: get_id() }, None, None)
                .with_description("The image whose threshold boundary is traced."),
            Input::new("threshold".to_string(), Value::Decimal(0.5), Some(InputSettings::Slider { range: (0.0, 1.0), step_by: None, clamp_to_range: true }), None)
                .with_description("Luminance cutoff; pixels at or above it count as inside the shape."),
            Input::new("tolerance".to_string(), Value::Decimal(2.0), Some(InputSettings::DragValue { clamp: Some((0.1, 32.0)), speed: Some(0.1) }), None)
                .with_description("Simplification tolerance in pixels at a 1024px reference; larger drops more points."),
        ]
    }

    /// Creates the default output: a single traced curve.
    pub fn create_outputs() -> Vec<Output> {
        vec![
            Output::new("output".to_string(), Value::Curve(Curve::default()), None)
                .with_description("The traced boundary as a Linear curve, or the default placeholder for an empty/full mask."),
        ]
    }

    /// Traces the longest threshold contour of the input image into a curve.
    pub async fn run(inputs: &mut [Input]) -> Result<OperationResponse, OperationError> {
        let start_time = Instant::now();
        let mut input_errors: Vec<(usize, String)> = vec![];

        let image_converted = convert_input(inputs, 0, ValueType::Image, &mut input_errors);
        let threshold_converted = convert_input(inputs, 1, ValueType::Decimal, &mut input_errors);
        let tolerance_converted = convert_input(inputs, 2, ValueType::Decimal, &mut input_errors);

        if !input_errors.is_empty() {
            return Err(OperationError { input_errors, node_error: None });
        }

        let Value::Image { data: image, change_id: _ } = image_converted.unwrap() else { unreachable!() };
        let Value::Decimal(threshold) = threshold_converted.unwrap() else { unreachable!() };
        let Value::Decimal(tolerance) = tolerance_converted.unwrap() else { unreachable!() };

        let w = image.width() as usize;
        let h = image.height() as usize;
        let thr = threshold.clamp(0.0, 1.0);

        let luma: Vec<f32> = image.pixels().map(pixel_luma).collect();
        let contours = trace_contours(&luma, w, h, thr);

        // Keep the longest contour by arc length; fall back to the default curve.
        let curve = contours
            .into_iter()
            .max_by(|a, b| {
                contour_length(a)
                    .partial_cmp(&contour_length(b))
                    .unwrap_or(std::cmp::Ordering::Equal)
            })
            .filter(|c| c.points.len() >= 2)
            .map(|c| {
                // tolerance is px@1024, divided into normalized units (not a
                // raster op, so no scale_to_resolution).
                let tol = (tolerance.max(0.0) / 1024.0) as f64;
                let pts = rdp_decimate(&c.points, tol, MAX_OUTPUT_POINTS);
                if pts.len() >= 2 {
                    linear_curve(pts, c.closed)
                } else {
                    Curve::default()
                }
            })
            .unwrap_or_default();

        Ok(OperationResponse {
            time: Instant::now().duration_since(start_time),
            responses: vec![OutputResponse { value: Value::Curve(curve) }],
        })
    }
}
